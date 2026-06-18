//! clay-ssg — render the IR into a fully static site (GitHub Pages ready).
//!
//! Input  : data/web/ (pages.jsonl with /img refs, collections.json, aliases.json)
//!          + app/style/main.scss + app/public/favicon.svg + data/web/img/
//! Output : site/  — one index.html per route, /img, /styles.css, alias redirects,
//!          and a Pagefind search box (run `npx pagefind --site site` afterwards).
//!
//! Direct HTML templating (no server, no hydration markers) so the output is a
//! clean, self-contained static archive. The CSS is the same main.scss the Leptos
//! dev app uses, compiled here with `grass`.
//!
//! Set BASE_URL (e.g. "/clay-knowledge/") for a GitHub Pages *project* site; the
//! default "/" suits a user/org page, a custom domain, or local preview.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clay_ir::{Chemistry, CollectionInfo, Page};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn main() -> Result<()> {
    let web = PathBuf::from(std::env::var("CLAY_WEB").unwrap_or_else(|_| "data/web".into()));
    let scss = PathBuf::from(std::env::var("CLAY_SCSS").unwrap_or_else(|_| "app/style/main.scss".into()));
    let favicon = PathBuf::from("app/public/favicon.svg");
    let out = PathBuf::from(std::env::var("CLAY_SITE").unwrap_or_else(|_| "site".into()));
    let mut base = std::env::var("BASE_URL").unwrap_or_else(|_| "/".into());
    if !base.starts_with('/') { base.insert(0, '/'); }
    if !base.ends_with('/') { base.push('/'); }
    let prefix = base.trim_end_matches('/').to_string(); // "" for "/", "/repo" otherwise

    // ---- load IR ----
    let pages: Vec<Page> = fs::read_to_string(web.join("pages.jsonl"))
        .with_context(|| format!("reading {}/pages.jsonl", web.display()))?
        .lines().filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l)).collect::<Result<_, _>>()?;
    let collections: Vec<CollectionInfo> =
        serde_json::from_str(&fs::read_to_string(web.join("collections.json"))?)?;
    let aliases: BTreeMap<String, String> =
        serde_json::from_str(&fs::read_to_string(web.join("aliases.json")).unwrap_or_else(|_| "{}".into()))?;

    // ---- compile CSS ----
    let mut css = grass::from_path(&scss, &grass::Options::default())
        .map_err(|e| anyhow::anyhow!("scss: {e}"))?;
    css.push_str(EXTRA_CSS);
    if out.exists() { let _ = fs::remove_dir_all(&out); }
    fs::create_dir_all(&out)?;
    fs::write(out.join("styles.css"), &css)?;
    if favicon.exists() { fs::copy(&favicon, out.join("favicon.svg"))?; }
    // GitHub Pages custom domain (e.g. potter.forest-anderson.ca)
    if let Ok(cname) = std::env::var("SITE_CNAME") {
        if !cname.trim().is_empty() { fs::write(out.join("CNAME"), format!("{}\n", cname.trim()))?; }
    }

    // ---- copy images ----
    let img_src = web.join("img");
    let img_dst = out.join("img");
    fs::create_dir_all(&img_dst)?;
    let mut n_img = 0;
    if let Ok(rd) = fs::read_dir(&img_src) {
        for e in rd.flatten() {
            fs::copy(e.path(), img_dst.join(e.file_name()))?;
            n_img += 1;
        }
    }

    let sidebar = render_sidebar(&collections, &prefix);
    let by_collection = group_by_collection(&pages);

    let write_page = |route: &str, title: &str, content: &str| -> Result<()> {
        let rel = route.trim_matches('/');
        let dir = if rel.is_empty() { out.clone() } else { out.join(rel) };
        fs::create_dir_all(&dir)?;
        let html = shell(title, &sidebar, content, &prefix, &base);
        fs::write(dir.join("index.html"), html)?;
        Ok(())
    };

    // ---- home ----
    write_page("/", "clay-knowledge — a modern ceramics reference",
        &render_home(&collections, pages.len()))?;

    // ---- collection list pages ----
    for c in &collections {
        let empty = Vec::new();
        let cards = by_collection.get(c.name.as_str()).unwrap_or(&empty);
        write_page(&format!("/{}", c.name), &format!("{} — clay-knowledge", c.title),
            &render_collection(&c.title, cards, &prefix))?;
    }

    // ---- pages ----
    for p in &pages {
        write_page(&p.path, &p.title, &render_page(p, &prefix))?;
    }

    // ---- alias redirect stubs ----
    for (alias, canon) in &aliases {
        let rel = alias.trim_matches('/');
        if rel.is_empty() { continue; }
        let dir = out.join(rel);
        fs::create_dir_all(&dir)?;
        let target = format!("{prefix}{canon}/");
        fs::write(dir.join("index.html"), format!(
            "<!doctype html><html data-pagefind-ignore><meta charset=utf-8>\
             <link rel=canonical href=\"{t}\"><meta http-equiv=refresh content=\"0; url={t}\">\
             <title>Redirecting…</title><body><a href=\"{t}\">{t}</a>", t = esc(&target)))?;
    }

    eprintln!(
        "[ssg] wrote {} pages + {} collections + {} alias redirects, {} images, to {}/",
        pages.len(), collections.len(), aliases.len(), n_img, out.display()
    );
    eprintln!("[ssg] next: `npx -y pagefind --site {}`  then serve {}/", out.display(), out.display());
    Ok(())
}

fn group_by_collection(pages: &[Page]) -> BTreeMap<&str, Vec<&Page>> {
    let mut m: BTreeMap<&str, Vec<&Page>> = BTreeMap::new();
    for p in pages { m.entry(p.collection.as_str()).or_default().push(p); }
    for v in m.values_mut() { v.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())); }
    m
}

fn shell(title: &str, sidebar: &str, content: &str, prefix: &str, base: &str) -> String {
    format!(r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="icon" href="{prefix}/favicon.svg">
<link rel="stylesheet" href="{prefix}/styles.css">
<link rel="stylesheet" href="{prefix}/pagefind/pagefind-ui.css">
<script src="{prefix}/pagefind/pagefind-ui.js"></script>
</head>
<body>
<div class="layout">
{sidebar}
<div class="main">
<header class="topbar"><div id="search" class="pf-search"></div></header>
<main class="content" data-pagefind-body>
{content}
</main>
</div>
</div>
<script>
window.addEventListener('DOMContentLoaded',function(){{
  if(window.PagefindUI){{ new PagefindUI({{element:"#search",showSubResults:true,showImages:false,baseUrl:"{base}"}}); }}
}});
</script>
</body>
</html>"##, title = esc(title))
}

fn render_sidebar(collections: &[CollectionInfo], prefix: &str) -> String {
    let mut items = String::new();
    for c in collections {
        items.push_str(&format!("<li><a href=\"{prefix}/{}\">{}</a></li>", c.name, esc(&c.title)));
    }
    format!(r#"<nav class="sidebar">
<a class="logo" href="{prefix}/">clay<span>knowledge</span></a>
<ul>{items}</ul>
<div class="sidebar-foot"><span>static archive</span><span>data: Wayback / digitalfire.com</span></div>
</nav>"#)
}

fn render_home(collections: &[CollectionInfo], total: usize) -> String {
    let mut tiles = String::new();
    for c in collections {
        tiles.push_str(&format!(
            "<a class=\"tile\" href=\"/{}\"><span class=\"tile-title\">{}</span><span class=\"tile-count\">{} pages</span></a>",
            c.name, esc(&c.title), c.count));
    }
    format!(r#"<div class="home">
<h1>A modern, searchable ceramics reference</h1>
<p class="lede">A preservation archive of the digitalfire glaze-chemistry library — materials, oxides, recipes, and glossary, rebuilt as fast static pages.</p>
<p class="muted">{total} pages · {n} collections</p>
<div class="grid">{tiles}</div>
</div>"#, n = collections.len())
}

fn render_collection(title: &str, cards: &[&Page], prefix: &str) -> String {
    let mut list = String::new();
    for p in cards {
        list.push_str(&format!(
            "<li class=\"cardrow\"><a href=\"{prefix}{}\"><span class=\"cardrow-title\">{}</span><span class=\"cardrow-sum\">{}</span></a></li>",
            p.path, esc(&p.title), esc(&p.summary)));
    }
    format!("<div class=\"collection\"><h1>{}</h1><p class=\"muted\">{} pages</p><ul class=\"cardlist\">{list}</ul></div>",
        esc(title), cards.len())
}

fn render_page(p: &Page, prefix: &str) -> String {
    let crumbs = format!(
        "<nav class=\"crumbs\"><a href=\"{prefix}/\">Home</a> / <a href=\"{prefix}/{}\">{}</a></nav>",
        p.collection, esc(&p.collection));
    let chem = p.chemistry.as_ref().map(render_chem).unwrap_or_default();
    // body links/img are root-relative; prefix them for project-site hosting
    let body = if prefix.is_empty() { p.body_html.clone() }
        else { p.body_html.replace("=\"/", &format!("=\"{prefix}/")) };
    let related = if p.related.is_empty() { String::new() } else {
        let mut r = String::from("<aside class=\"related\"><h3>Related</h3><ul>");
        for l in &p.related {
            let href = if prefix.is_empty() || !l.href.starts_with('/') { l.href.clone() }
                else { format!("{prefix}{}", l.href) };
            r.push_str(&format!("<li><a href=\"{}\">{}</a></li>", esc(&href), esc(&l.label)));
        }
        r.push_str("</ul></aside>");
        r
    };
    format!("{crumbs}{chem}<div class=\"article-body\">{body}</div>{related}\
        <footer class=\"provenance\">Archived from <a href=\"{src}\" target=\"_blank\" rel=\"noopener\">{src}</a></footer>",
        src = esc(&p.source_url))
}

fn render_chem(c: &Chemistry) -> String {
    let mut s = String::from("<section class=\"chem\"><h3>Chemistry</h3>");
    if let Some(f) = &c.formula { s.push_str(&format!("<p class=\"chem-formula\">{}</p>", esc(f))); }
    if !c.analysis.is_empty() {
        s.push_str("<table class=\"chem-table\"><thead><tr><th>Oxide</th><th>Analysis %</th><th>Unity (UMF)</th><th>\u{00b1}</th></tr></thead><tbody>");
        for o in &c.analysis {
            s.push_str(&format!("<tr><td class=\"ox\">{}</td><td>{}</td><td>{}</td><td class=\"tol\">{}</td></tr>",
                esc(&o.oxide),
                o.analysis_pct.map(|v| format!("{v:.2}")).unwrap_or_default(),
                o.formula.map(|v| format!("{v:.3}")).unwrap_or_default(),
                o.tolerance.as_deref().map(esc).unwrap_or_default()));
        }
        s.push_str("</tbody></table>");
    }
    // Stull point
    let sio2 = c.analysis.iter().find(|o| o.oxide == "SiO2").and_then(|o| o.formula);
    let al2o3 = c.analysis.iter().find(|o| o.oxide == "Al2O3").and_then(|o| o.formula);
    if let (Some(x), Some(y)) = (sio2, al2o3) {
        if x > 0.0 {
            let cx = 44.0 + (x.clamp(0.0, 5.0) / 5.0) * 256.0;
            let cy = 180.0 - (y.clamp(0.0, 1.0) / 1.0) * 164.0;
            s.push_str(&format!(r##"<figure class="stull-fig"><svg class="stull" width="320" height="210">
<rect x="44" y="16" width="256" height="164" fill="none" stroke="#d9d4cc"></rect>
<line x1="95.2" y1="16" x2="95.2" y2="180" stroke="#efeae2"></line><line x1="146.4" y1="16" x2="146.4" y2="180" stroke="#efeae2"></line>
<line x1="197.6" y1="16" x2="197.6" y2="180" stroke="#efeae2"></line><line x1="248.8" y1="16" x2="248.8" y2="180" stroke="#efeae2"></line>
<line x1="44" y1="147.2" x2="300" y2="147.2" stroke="#efeae2"></line><line x1="44" y1="114.4" x2="300" y2="114.4" stroke="#efeae2"></line>
<line x1="44" y1="81.6" x2="300" y2="81.6" stroke="#efeae2"></line><line x1="44" y1="48.8" x2="300" y2="48.8" stroke="#efeae2"></line>
<text x="92" y="194" class="tick">1</text><text x="143" y="194" class="tick">2</text><text x="194" y="194" class="tick">3</text><text x="245" y="194" class="tick">4</text><text x="296" y="194" class="tick">5</text>
<text x="22" y="151" class="tick">0.2</text><text x="22" y="118" class="tick">0.4</text><text x="22" y="85" class="tick">0.6</text><text x="22" y="52" class="tick">0.8</text><text x="22" y="20" class="tick">1.0</text>
<text x="150" y="207" class="ax">SiO₂ (UMF)</text><text x="14" y="104" class="ax" transform="rotate(-90 14 104)">Al₂O₃ (UMF)</text>
<text x="60" y="120" class="zone">crazing</text><text x="196" y="126" class="zone">glossy</text><text x="150" y="44" class="zone">matte</text><text x="236" y="40" class="zone">under-fired</text><text x="96" y="172" class="zone">fluid</text>
<circle cx="{cx:.1}" cy="{cy:.1}" r="5.5" fill="#b5502a" stroke="#fff" stroke-width="1.5"></circle>
</svg><figcaption class="stull-cap">SiO₂ {x:.2} · Al₂O₃ {y:.2} — Stull position (approximate zones, ≈cone 10)</figcaption></figure>"##));
        }
    }
    if !c.properties.is_empty() {
        s.push_str("<table class=\"chem-table props\"><tbody>");
        for kv in &c.properties {
            s.push_str(&format!("<tr><td class=\"ox\">{}</td><td class=\"pv\">{}</td></tr>", esc(&kv.key), esc(&kv.val)));
        }
        s.push_str("</tbody></table>");
    }
    s.push_str(&format!("<p class=\"chem-meta\">{}{}</p>",
        c.oxide_weight.map(|w| format!("Oxide weight {w:.2}")).unwrap_or_default(),
        c.formula_weight.map(|w| format!(" \u{00b7} Formula weight {w:.2}")).unwrap_or_default()));
    s.push_str("<p class=\"chem-src\">Structured data via <a href=\"https://github.com/millandr121/digitalfire\" target=\"_blank\" rel=\"noopener\">millandr121/digitalfire</a></p></section>");
    s
}

const EXTRA_CSS: &str = r#"
/* static-site search box (Pagefind) */
.pf-search { max-width: 640px; }
.pf-search .pagefind-ui__search-input { border-radius: 10px; }
"#;

#[allow(dead_code)]
fn _p(_: &Path) {}
