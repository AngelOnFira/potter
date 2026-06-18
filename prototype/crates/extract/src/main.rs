//! clay-extract — convert digitalfire HTML pages into the clay-knowledge IR.
//!
//! Usage:  clay-extract <SRC_DIR> <OUT_DIR>
//!   SRC_DIR  directory with collection subdirs (e.g. data/digitalfire-archive)
//!   OUT_DIR  where pages.jsonl + collections.json are written (e.g. prototype/data/ir)
//!
//! Two passes:
//!   1. Build a map {collection/raw_filename_stem -> /collection/url-safe-slug} so
//!      internal links (which use raw stems like `../glossary/glaze+chemistry.html`)
//!      can be rewritten to clean app routes regardless of percent/`+` encoding.
//!   2. Extract each page in parallel: pick the main content container, sanitize it
//!      with ammonia (stripping nav/boilerplate/classes), rewrite internal links and
//!      image sources, and derive plain text for the search index.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clay_ir::{Chemistry, CollectionInfo, KeyVal, Link, OxideRow, Page};
use serde::Deserialize;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};

/// Knowledge-core collections (display titles). `typecode` is absent from the ZIP.
const COLLECTIONS: &[(&str, &str)] = &[
    ("material", "Materials"),
    ("oxide", "Oxides"),
    ("glossary", "Glossary"),
    ("article", "Articles"),
    ("recipe", "Recipes"),
    ("mineral", "Minerals"),
    ("hazard", "Hazards"),
    ("trouble", "Troubleshooting"),
    ("test", "Tests"),
    ("property", "Properties"),
    ("temperature", "Temperatures"),
    ("schedule", "Firing Schedules"),
];

static HREF_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"href="([^"]*)""#).unwrap());
static SRC_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"src="([^"]*)""#).unwrap());
static TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").unwrap());
static WS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
static A_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r##"(?s)<a\s+href="(/[^"#]+)"[^>]*>(.*?)</a>"##).unwrap());
static EL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"([A-Z][a-z]?)(\d*)").unwrap());

struct FileJob {
    collection: &'static str,
    path: PathBuf,
    raw_stem: String,
    slug: String,
    route: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let src = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("../data/digitalfire-archive"));
    let out = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("data/ir"));
    fs::create_dir_all(&out)?;

    eprintln!("[extract] src={} out={}", src.display(), out.display());

    // ---- pass 1: enumerate files, assign unique url-safe slugs, build link map ----
    let mut jobs: Vec<FileJob> = Vec::new();
    let mut link_map: HashMap<String, String> = HashMap::new();
    for (collection, _) in COLLECTIONS {
        let dir = src.join(collection);
        if !dir.is_dir() {
            eprintln!("[extract]   skip missing collection dir: {}", dir.display());
            continue;
        }
        let mut used: HashSet<String> = HashSet::new();
        let mut entries: Vec<PathBuf> = fs::read_dir(&dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|x| x == "html").unwrap_or(false))
            .collect();
        entries.sort();
        for path in entries {
            let stem = path.file_stem().unwrap().to_string_lossy().to_string();
            if stem == "list" || stem == "index" {
                continue; // collection index pages; the app builds its own lists
            }
            let mut slug = slugify(&stem);
            if slug.is_empty() {
                slug = format!("p{}", used.len());
            }
            // ensure uniqueness within the collection
            let mut candidate = slug.clone();
            let mut n = 2;
            while used.contains(&candidate) {
                candidate = format!("{slug}-{n}");
                n += 1;
            }
            slug = candidate;
            used.insert(slug.clone());
            let route = format!("/{collection}/{slug}");
            link_map.insert(format!("{collection}/{stem}"), route.clone());
            jobs.push(FileJob { collection, path, raw_stem: stem, slug, route });
        }
    }
    eprintln!("[extract] pass1: {} pages across {} collections", jobs.len(), COLLECTIONS.len());

    // ---- pass 2: extract content in parallel ----
    let pages: Vec<Page> = jobs
        .par_iter()
        .filter_map(|job| match extract_one(job, &link_map) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("[extract]   FAILED {}: {e}", job.path.display());
                None
            }
        })
        .collect();

    // ---- dedup slug/id duplicate URLs (same content at /glossary/200-mesh and
    // /glossary/343); keep the human-readable slug, alias the rest ----
    let before = pages.len();
    let (mut pages, aliases) = dedup_pages(pages);
    // Point inline links at canonical routes (e.g. /material/806 -> /material/gerstley-borate).
    for p in pages.iter_mut() {
        p.body_html = HREF_RE
            .replace_all(&p.body_html, |c: &regex::Captures| match aliases.get(&c[1]) {
                Some(canon) => format!("href=\"{canon}\""),
                None => format!("href=\"{}\"", &c[1]),
            })
            .into_owned();
        for l in p.related.iter_mut() {
            if let Some(canon) = aliases.get(&l.href) {
                l.href = canon.clone();
            }
        }
    }
    eprintln!(
        "[extract] dedup: {before} -> {} canonical pages ({} aliases)",
        pages.len(),
        aliases.len()
    );

    // ---- attach structured oxide chemistry (materials/minerals) from the
    // millandr121 dataset, matched by normalized name/slug ----
    let chem_dir = PathBuf::from(std::env::var("CLAY_CHEM").unwrap_or_else(|_| "data/chem".into()));
    let chem_mat = load_chem(&chem_dir.join("materials.json"));
    let chem_min = load_chem(&chem_dir.join("minerals.json"));
    let chem_ox = load_oxides(&chem_dir.join("oxides.json"));
    let chem_recipe = load_recipes(&chem_dir.join("recipes.json"), &chem_mat);
    let mut attached = 0usize;
    for p in pages.iter_mut() {
        let map = match p.collection.as_str() {
            "material" => &chem_mat,
            "mineral" => &chem_min,
            "oxide" => &chem_ox,
            "recipe" => &chem_recipe,
            _ => continue,
        };
        // try full title, the symbol before " (...)", and the slug
        let title_sym = p.title.split(" (").next().unwrap_or(&p.title);
        for k in [normkey(&p.title), normkey(title_sym), normkey(&p.slug)] {
            if let Some(c) = map.get(&k) {
                p.chemistry = Some(c.clone());
                attached += 1;
                break;
            }
        }
    }
    eprintln!(
        "[extract] chemistry: attached to {attached} pages (material:{} mineral:{} oxide:{} recipe:{} records)",
        chem_mat.len(),
        chem_min.len(),
        chem_ox.len(),
        chem_recipe.len()
    );

    // ---- write outputs ----
    let pages_path = out.join("pages.jsonl");
    let mut buf = String::with_capacity(pages.len() * 2048);
    for p in &pages {
        buf.push_str(&serde_json::to_string(p)?);
        buf.push('\n');
    }
    fs::write(&pages_path, buf).with_context(|| format!("writing {}", pages_path.display()))?;

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for p in &pages {
        *counts.entry(p.collection.as_str()).or_default() += 1;
    }
    let collections: Vec<CollectionInfo> = COLLECTIONS
        .iter()
        .map(|(name, title)| CollectionInfo {
            name: name.to_string(),
            title: title.to_string(),
            count: counts.get(name).copied().unwrap_or(0),
        })
        .filter(|c| c.count > 0)
        .collect();
    fs::write(out.join("collections.json"), serde_json::to_string_pretty(&collections)?)?;
    fs::write(out.join("aliases.json"), serde_json::to_string_pretty(&aliases)?)?;

    eprintln!("[extract] wrote {} pages -> {}", pages.len(), pages_path.display());
    for c in &collections {
        eprintln!("[extract]   {:<12} {}", c.name, c.count);
    }
    Ok(())
}

fn extract_one(job: &FileJob, link_map: &HashMap<String, String>) -> Result<Page> {
    let raw = fs::read_to_string(&job.path)?;
    let doc = Html::parse_document(&raw);

    let title = title_of(&doc).unwrap_or_else(|| pretty_stem(&job.raw_stem));

    // Pick the main content container (skip the navbar), then keep only its
    // content children — dropping link-dense navigation blocks (oxide/material
    // "pickers", mega-menus) that otherwise pollute the body and summary.
    let content_html = choose_root(&doc).map(|r| extract_content(&r)).unwrap_or_default();

    let cleaned = sanitize(&content_html);
    let body_html = rewrite_urls(&cleaned, link_map, job.collection);
    let text = strip_tags(&body_html);
    let summary = summarize(&text, 240);
    let related = related_links(&body_html, &job.route, 16);

    Ok(Page {
        collection: job.collection.to_string(),
        slug: job.slug.clone(),
        path: job.route.clone(),
        title,
        summary,
        body_html,
        text,
        related,
        source_url: format!("https://digitalfire.com/{}/{}", job.collection, job.raw_stem),
        chemistry: None,
    })
}

/// Choose the main content element: the highest-text candidate that is NOT inside
/// the `<nav>`. Falls back to `<body>`.
fn choose_root(doc: &Html) -> Option<ElementRef<'_>> {
    let sel = Selector::parse("main, article, div.container, #content, .content, #main").unwrap();
    let mut best: Option<(usize, ElementRef)> = None;
    for el in doc.select(&sel) {
        let in_nav = el
            .ancestors()
            .any(|n| n.value().as_element().map(|e| e.name() == "nav").unwrap_or(false));
        if in_nav {
            continue;
        }
        let n = el.text().map(|t| t.chars().count()).sum::<usize>();
        if best.as_ref().map(|(bn, _)| n > *bn).unwrap_or(true) {
            best = Some((n, el));
        }
    }
    if let Some((n, el)) = best {
        if n > 40 {
            return Some(el);
        }
    }
    doc.select(&Selector::parse("body").unwrap()).next()
}

/// Keep content child-blocks; drop navigation/picker blocks (many short links).
fn extract_content(root: &ElementRef) -> String {
    let a_sel = Selector::parse("a").unwrap();
    let img_sel = Selector::parse("img").unwrap();
    let mut kept = String::new();
    for child in root.children().filter_map(ElementRef::wrap) {
        if matches!(
            child.value().name(),
            "nav" | "script" | "style" | "footer" | "header" | "noscript" | "form" | "button"
        ) {
            continue;
        }
        let chtml = child.html();
        let ctext: String = child.text().collect();
        let text_len = ctext.trim().len();
        let has_img = child.select(&img_sel).next().is_some();
        if text_len == 0 && !has_img {
            continue;
        }
        // Drop the site-wide footer (donate/social/ko-fi/privacy), the picker
        // toggle ("All Glossary"…), breadcrumbs, and the linkmaker block.
        if is_boilerplate(&chtml, &ctext) {
            continue;
        }
        let labels: Vec<usize> = child
            .select(&a_sel)
            .map(|a| a.text().collect::<String>().trim().chars().count())
            .collect();
        let nlinks = labels.len();
        let link_text: usize = labels.iter().sum();
        let avg = if nlinks > 0 { link_text as f64 / nlinks as f64 } else { 0.0 };
        let density = link_text as f64 / text_len.max(1) as f64;
        // picker: lots of tiny links;  navlist: link-dominated block
        if (nlinks > 20 && avg < 7.0) || (nlinks > 10 && density > 0.8) {
            continue;
        }
        kept.push_str(&chtml);
    }
    if kept.trim().chars().count() < 80 {
        return root.inner_html();
    }
    kept
}

/// Identify digitalfire boilerplate child-blocks to drop: the site-wide footer
/// (PayPal donate, social row, Ko-fi, ReferenceLibrary logo, privacy), the picker
/// toggle ("All Glossary"…), the "Materials for Ceramics" breadcrumb, and the
/// "Key phrases linking here" linkmaker block.
fn is_boilerplate(html: &str, text: &str) -> bool {
    const SIGS: &[&str] = &[
        "PayPalDonate",
        "ko-fi",
        "Buy me a coffee",
        "ReferenceLibrary",
        "SignaturePhotoSmall",
        "No tracking, No ads",
        "Privacy Policy",
        "Key phrases linking here",
        "Follow me on",
    ];
    if SIGS.iter().any(|s| html.contains(s)) {
        return true;
    }
    // leftover picker toggle anchor
    if html.contains("#collapse") {
        return true;
    }
    // the footer social row (multiple social hosts, no real prose)
    let social = ["instagram.com", "facebook.com", "threads.", "linkedin.com", "x.com", "twitter.com"]
        .iter()
        .filter(|s| html.contains(**s))
        .count();
    if social >= 2 && text.trim().len() < 200 {
        return true;
    }
    let t = text.trim();
    if t.starts_with("Materials for Ceramics") {
        return true;
    }
    // short "All Glossary" / "All Minerals" … toggle/breadcrumb
    if t.len() < 60 && t.starts_with("All ") {
        return true;
    }
    false
}

/// ammonia clean: keep semantic structure + tables, drop scripts/nav/classes/styles
/// so the app's own CSS controls the look.
fn sanitize(html: &str) -> String {
    let mut b = ammonia::Builder::default();
    // Remove these from the default allow-list first (ammonia forbids a tag being in
    // both `tags` and `clean_content_tags`), then drop them *with* their content.
    b.rm_tags(["nav", "form", "button", "svg", "noscript", "input", "select", "script", "style"]);
    b.add_clean_content_tags(["script", "style", "nav", "form", "button", "noscript", "svg"]);
    b.add_tags(["figure", "figcaption", "section", "sub", "sup", "small", "h1"]);
    b.add_tag_attributes("img", ["src", "alt", "title"]);
    b.add_tag_attributes("td", ["colspan", "rowspan"]);
    b.add_tag_attributes("th", ["colspan", "rowspan"]);
    b.add_tag_attributes("a", ["href", "title"]);
    b.url_relative(ammonia::UrlRelative::PassThrough);
    b.clean(html).to_string()
}

/// Rewrite internal links (`../glossary/frit.html`, digitalfire absolute URLs) to
/// app routes via the slug map, and image `src`s into the local `/media/...` route.
fn rewrite_urls(html: &str, link_map: &HashMap<String, String>, collection: &str) -> String {
    let after_href = HREF_RE.replace_all(html, |c: &regex::Captures| {
        format!("href=\"{}\"", rewrite_href(&c[1], link_map, collection))
    });
    SRC_RE
        .replace_all(&after_href, |c: &regex::Captures| {
            format!("src=\"{}\"", rewrite_src(&c[1]))
        })
        .into_owned()
}

fn rewrite_href(href: &str, link_map: &HashMap<String, String>, collection: &str) -> String {
    let h = href.trim();
    if h.is_empty()
        || h.starts_with('#')
        || h.starts_with("mailto:")
        || h.starts_with("tel:")
        || h.starts_with("javascript:")
    {
        return h.to_string();
    }

    let mut root_relative = false;
    let mut s = h.to_string();

    // Absolute URL: only treat as internal if the *host* is digitalfire (so e.g.
    // instagram.com/tonyatdigitalfire stays external and isn't turned into /https://…).
    if let Some(rest) = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://")) {
        let (host, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let host = host.to_ascii_lowercase();
        let is_df = matches!(
            host.as_str(),
            "digitalfire.com" | "www.digitalfire.com" | "digitlfire.com" | "www.digitlfire.com"
        );
        if !is_df {
            return h.to_string(); // external link, pass through unchanged
        }
        s = path.to_string();
        root_relative = true;
    }

    if s.starts_with('/') {
        root_relative = true;
    }
    s = s.trim_start_matches("./").to_string();
    while s.starts_with("../") {
        root_relative = true; // any "../" resolves from the page dir up to site root
        s = s[3..].to_string();
    }
    let s = s.trim_start_matches('/').to_string();
    let mut key = s.strip_suffix(".html").unwrap_or(&s).to_string();

    // site home (e.g. ../home.html, /index.html)
    if key.is_empty() || key == "home" || key == "index" {
        return "/".to_string();
    }
    // Same-directory relative link (e.g. "b2o3.html") -> resolve against this collection.
    if !root_relative && !key.contains('/') {
        key = format!("{collection}/{key}");
    }
    // A collection's "list" index page maps to the collection landing route.
    if let Some(coll) = key.strip_suffix("/list") {
        return format!("/{coll}");
    }
    if let Some(route) = link_map.get(&key) {
        return route.clone();
    }
    // Unknown internal link (e.g. a collection we didn't extract): best-effort clean path.
    if key.contains('/') && !key.contains(' ') {
        return format!("/{key}");
    }
    href.to_string()
}

fn rewrite_src(src: &str) -> String {
    let s = src.trim();
    if let Some(idx) = s.find("media/") {
        return format!("/{}", &s[idx..]);
    }
    if s.starts_with("http") {
        return s.to_string();
    }
    s.to_string()
}

fn related_links(html: &str, self_route: &str, cap: usize) -> Vec<Link> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for c in A_RE.captures_iter(html) {
        let href = c[1].to_string();
        if href == self_route || !seen.insert(href.clone()) {
            continue;
        }
        let label = WS_RE.replace_all(&strip_tags(&c[2]), " ").trim().to_string();
        if label.is_empty() || label.len() > 80 {
            continue;
        }
        out.push(Link { label, href });
        if out.len() >= cap {
            break;
        }
    }
    out
}

fn title_of(doc: &Html) -> Option<String> {
    let sel = Selector::parse("title").unwrap();
    let raw = doc.select(&sel).next()?.text().collect::<String>();
    let t = WS_RE.replace_all(&raw, " ").trim().to_string();
    // strip common site suffixes
    let t = t
        .trim_end_matches("| Digitalfire")
        .trim_end_matches("- Digitalfire")
        .trim_end_matches("Digitalfire")
        .trim_end_matches([' ', '|', '-'])
        .to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

fn strip_tags(html: &str) -> String {
    let no_tags = TAG_RE.replace_all(html, " ");
    let decoded = no_tags
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    WS_RE.replace_all(&decoded, " ").trim().to_string()
}

fn summarize(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max).collect();
    match truncated.rfind(' ') {
        Some(i) => format!("{}…", &truncated[..i]),
        None => format!("{truncated}…"),
    }
}

// ---- structured chemistry (millandr121 dataset) ----

#[derive(Deserialize)]
struct MillRec {
    #[serde(default)]
    name: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    alternate_names: Option<String>,
    #[serde(default)]
    formula: Option<String>,
    #[serde(default)]
    analysis: Vec<MillOx>,
    #[serde(default)]
    oxide_weight: Option<f64>,
    #[serde(default)]
    formula_weight: Option<f64>,
}

#[derive(Deserialize)]
struct MillOx {
    #[serde(default)]
    oxide: String,
    #[serde(default)]
    analysis_pct: Option<f64>,
    #[serde(default)]
    formula: Option<f64>,
    #[serde(default)]
    tolerance: Option<serde_json::Value>,
}

fn tol_to_string(v: &Option<serde_json::Value>) -> Option<String> {
    match v {
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Normalized match key: ascii-alphanumeric, lowercased ("#1 Q-Rok" -> "1qrok").
fn normkey(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn load_chem(path: &Path) -> HashMap<String, Chemistry> {
    let mut map = HashMap::new();
    let raw = match fs::read_to_string(path) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("[extract]   (no chem file at {})", path.display());
            return map;
        }
    };
    let recs: Vec<MillRec> = serde_json::from_str(&raw).unwrap_or_default();
    for r in recs {
        if r.analysis.is_empty() {
            continue;
        }
        let chem = Chemistry {
            analysis: r
                .analysis
                .iter()
                .map(|o| OxideRow {
                    oxide: o.oxide.clone(),
                    analysis_pct: o.analysis_pct,
                    formula: o.formula,
                    tolerance: tol_to_string(&o.tolerance),
                })
                .collect(),
            properties: Vec::new(),
            oxide_weight: r.oxide_weight,
            formula_weight: r.formula_weight,
            formula: r.formula.clone(),
            alternate_names: r.alternate_names.clone(),
            data_source: "millandr121/digitalfire".to_string(),
        };
        let mut all_keys = vec![normkey(&r.name), normkey(&r.id)];
        for a in r.alternate_names.iter().flat_map(|s| s.split(',')) {
            all_keys.push(normkey(a));
        }
        for k in all_keys {
            if !k.is_empty() {
                map.entry(k).or_insert_with(|| chem.clone());
            }
        }
    }
    map
}

/// Oxides have a different schema: a `data` dict of reference properties (no
/// oxide-analysis array). Build a Chemistry with `properties` populated.
#[derive(Deserialize)]
struct OxideRec {
    #[serde(default)]
    symbol: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    data: std::collections::BTreeMap<String, serde_json::Value>,
}

fn load_oxides(path: &Path) -> HashMap<String, Chemistry> {
    let mut map = HashMap::new();
    let Ok(raw) = fs::read_to_string(path) else {
        eprintln!("[extract]   (no oxide file at {})", path.display());
        return map;
    };
    let recs: Vec<OxideRec> = serde_json::from_str(&raw).unwrap_or_default();
    for r in recs {
        let props: Vec<KeyVal> = r
            .data
            .iter()
            .map(|(k, v)| KeyVal {
                key: k.clone(),
                val: match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                },
            })
            .collect();
        if props.is_empty() {
            continue;
        }
        let chem = Chemistry {
            analysis: Vec::new(),
            properties: props,
            oxide_weight: None,
            formula_weight: None,
            formula: None,
            alternate_names: if r.name.is_empty() { None } else { Some(r.name.clone()) },
            data_source: "millandr121/digitalfire".to_string(),
        };
        for k in [normkey(&r.symbol), normkey(&r.id)] {
            if !k.is_empty() {
                map.entry(k).or_insert_with(|| chem.clone());
            }
        }
    }
    map
}

// ---- Seger unity-molecular-formula (UMF) engine, ported from millandr's chem.ts ----

fn atomic_weight(el: &str) -> Option<f64> {
    Some(match el {
        "H" => 1.008, "Li" => 6.94, "B" => 10.81, "C" => 12.011, "N" => 14.007,
        "O" => 15.999, "F" => 18.998, "Na" => 22.99, "Mg" => 24.305, "Al" => 26.982,
        "Si" => 28.085, "P" => 30.974, "S" => 32.06, "Cl" => 35.45, "K" => 39.098,
        "Ca" => 40.078, "Ti" => 47.867, "V" => 50.942, "Cr" => 51.996, "Mn" => 54.938,
        "Fe" => 55.845, "Co" => 58.933, "Ni" => 58.693, "Cu" => 63.546, "Zn" => 65.38,
        "Sr" => 87.62, "Zr" => 91.224, "Sn" => 118.71, "Sb" => 121.76, "Ba" => 137.327,
        "Pb" => 207.2, "Bi" => 208.98,
        _ => return None,
    })
}

/// Molecular weight of an oxide symbol ("Al2O3" -> 101.96); None for non-oxides
/// (LOI/Organics) or unparseable symbols ("Free SiO2").
fn molwt(sym: &str) -> Option<f64> {
    if sym.is_empty() || sym.contains(' ') || matches!(sym, "LOI" | "Organics" | "Trace") {
        return None;
    }
    let mut total = 0.0;
    let mut matched = String::new();
    for c in EL_RE.captures_iter(sym) {
        if c[0].is_empty() {
            break;
        }
        let n: f64 = if c[2].is_empty() { 1.0 } else { c[2].parse().ok()? };
        total += atomic_weight(&c[1])? * n;
        matched.push_str(&c[0]);
    }
    if matched != sym || total == 0.0 {
        return None;
    }
    Some(total)
}

fn is_flux(oxide: &str) -> bool {
    matches!(
        oxide,
        "Li2O" | "Na2O" | "K2O" | "KNaO" | "Rb2O" | "Cs2O" | "CaO" | "MgO" | "BaO"
            | "SrO" | "ZnO" | "PbO" | "MnO" | "FeO" | "CuO" | "CoO" | "NiO" | "CdO"
    )
}

/// Compute a glaze's unity formula from its resolved ingredients. Returns the
/// UMF rows plus how many ingredients resolved to a known material analysis.
fn compute_glaze_umf(
    ings: &[(String, f64)],
    mat_map: &HashMap<String, Chemistry>,
) -> Option<(Vec<OxideRow>, usize, usize)> {
    let mut moles: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    let mut resolved = 0;
    for (name, amount) in ings {
        let Some(chem) = mat_map.get(&normkey(name)) else { continue };
        resolved += 1;
        for row in &chem.analysis {
            let (Some(pct), Some(mw)) = (row.analysis_pct, molwt(&row.oxide)) else { continue };
            *moles.entry(row.oxide.clone()).or_default() += amount * (pct / 100.0) / mw;
        }
    }
    let flux: f64 = moles.iter().filter(|(o, _)| is_flux(o)).map(|(_, m)| *m).sum();
    if flux <= 0.0 || resolved == 0 {
        return None;
    }
    let umf = moles
        .iter()
        .map(|(o, m)| OxideRow {
            oxide: o.clone(),
            analysis_pct: None,
            formula: Some(((m / flux) * 1000.0).round() / 1000.0),
            tolerance: None,
        })
        .collect();
    Some((umf, resolved, ings.len()))
}

#[derive(Deserialize, Clone)]
struct RecMat {
    #[serde(default)]
    material: String,
    #[serde(default)]
    amount: Option<f64>,
    #[serde(default)]
    percent: Option<f64>,
}

#[derive(Deserialize)]
struct RecipeRec {
    #[serde(default)]
    code: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    materials: Vec<RecMat>,
}

/// millandr recipe records often concatenate several glaze variations (each
/// restarting with the same base material). Take just the first variation.
fn first_variation(mats: &[RecMat]) -> Vec<RecMat> {
    if mats.is_empty() {
        return vec![];
    }
    let first = normkey(&mats[0].material);
    let mut out = vec![mats[0].clone()];
    for m in &mats[1..] {
        if normkey(&m.material) == first {
            break; // next variation starts here
        }
        out.push(m.clone());
    }
    out
}

fn load_recipes(path: &Path, mat_map: &HashMap<String, Chemistry>) -> HashMap<String, Chemistry> {
    let mut map = HashMap::new();
    let Ok(raw) = fs::read_to_string(path) else {
        eprintln!("[extract]   (no recipe file at {})", path.display());
        return map;
    };
    let recs: Vec<RecipeRec> = serde_json::from_str(&raw).unwrap_or_default();
    for r in recs {
        let var = first_variation(&r.materials);
        let ings: Vec<(String, f64)> = var
            .iter()
            .filter_map(|m| m.amount.or(m.percent).map(|a| (m.material.clone(), a)))
            .collect();
        if ings.is_empty() {
            continue;
        }
        let Some((umf, resolved, total)) = compute_glaze_umf(&ings, mat_map) else { continue };
        let chem = Chemistry {
            analysis: umf,
            properties: vec![KeyVal {
                key: "Computed UMF".to_string(),
                val: format!("{resolved} of {total} ingredients resolved"),
            }],
            oxide_weight: None,
            formula_weight: None,
            formula: None,
            alternate_names: None,
            data_source: "computed from millandr121 recipe + material analyses".to_string(),
        };
        for k in [normkey(&r.code), normkey(&r.name)] {
            if !k.is_empty() {
                map.entry(k).or_insert_with(|| chem.clone());
            }
        }
    }
    map
}

/// Merge pages with identical (collection, title, body text) — digitalfire serves
/// the same page at both a slug URL and a numeric-id URL. Keep one canonical page
/// (preferring the human-readable, non-numeric slug) and return alias->canonical.
fn dedup_pages(pages: Vec<Page>) -> (Vec<Page>, std::collections::BTreeMap<String, String>) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();
    for (i, p) in pages.iter().enumerate() {
        let mut h = DefaultHasher::new();
        p.collection.hash(&mut h);
        p.title.hash(&mut h);
        p.text.hash(&mut h);
        groups.entry(h.finish()).or_default().push(i);
    }

    let is_numeric = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let mut keep = vec![false; pages.len()];
    let mut canonical_of: Vec<usize> = (0..pages.len()).collect();
    for idxs in groups.values() {
        let canon = *idxs
            .iter()
            .min_by(|&&a, &&b| {
                let pa = &pages[a];
                let pb = &pages[b];
                (is_numeric(&pa.slug), pa.slug.len(), &pa.slug)
                    .cmp(&(is_numeric(&pb.slug), pb.slug.len(), &pb.slug))
            })
            .unwrap();
        keep[canon] = true;
        for &i in idxs {
            canonical_of[i] = canon;
        }
    }

    let mut aliases = std::collections::BTreeMap::new();
    for (i, p) in pages.iter().enumerate() {
        if !keep[i] {
            aliases.insert(p.path.clone(), pages[canonical_of[i]].path.clone());
        }
    }
    let canonical: Vec<Page> = pages
        .into_iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, p)| p)
        .collect();
    (canonical, aliases)
}

fn slugify(stem: &str) -> String {
    let decoded = urlencoding::decode(stem).map(|c| c.into_owned()).unwrap_or_else(|_| stem.to_string());
    let mut out = String::with_capacity(decoded.len());
    let mut prev_dash = false;
    for ch in decoded.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn pretty_stem(stem: &str) -> String {
    let decoded = urlencoding::decode(stem).map(|c| c.into_owned()).unwrap_or_else(|_| stem.to_string());
    decoded.replace('+', " ")
}

#[allow(dead_code)]
fn _unused(_: &Path) {}
