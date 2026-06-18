//! clay-images — web-optimize + rename the images referenced by the IR.
//!
//! For every image referenced by a page's body_html (`src="/media/images/..."`),
//! this:
//!   1. resizes it to a web-sane size (max 1200px) and re-encodes (JPEG q82),
//!   2. renames it to an incrementing `N.ext` (jpeg normalized to .jpg),
//!   3. writes a mapping (new <-> old), and
//!   4. rewrites every reference in the IR to the new `/img/N.ext` path.
//!
//! Output (default `data/web/`): pages.jsonl (refs rewritten), img/<N>.<ext>,
//! img_map.json, plus copies of collections.json / aliases.json.
//!
//! Resizing shells out to ImageMagick `magick` (parallelised with rayon). GIF/SVG
//! are copied as-is (preserve animation / vectors).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use clay_ir::Page;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use serde::Serialize;

static SRC_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?:src|href)="(/media/[^"]+)""#).unwrap());
static MAX_DIM: &str = "1200x1200>";
static QUALITY: &str = "82";

#[derive(Serialize)]
struct MapEntry {
    new: String,
    old: String,
    old_bytes: u64,
    new_bytes: u64,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let ir_dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("data/ir"));
    let media = PathBuf::from(
        args.get(2).map(String::as_str).unwrap_or("../data/digitalfire-archive/media"),
    );
    let out = PathBuf::from(args.get(3).map(String::as_str).unwrap_or("data/web"));
    let img_out = out.join("img");
    fs::create_dir_all(&img_out)?;

    // ---- load pages, collect referenced media paths ----
    let raw = fs::read_to_string(ir_dir.join("pages.jsonl"))
        .with_context(|| format!("reading {}/pages.jsonl", ir_dir.display()))?;
    let mut pages: Vec<Page> = raw.lines().filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l)).collect::<Result<_, _>>()?;

    let mut refs: Vec<String> = {
        let mut set = std::collections::BTreeSet::new();
        for p in &pages {
            for c in SRC_RE.captures_iter(&p.body_html) {
                set.insert(c[1].to_string());
            }
        }
        set.into_iter().collect()
    };
    refs.sort();
    eprintln!("[images] {} pages, {} unique referenced media", pages.len(), refs.len());

    // ---- assign new names + plan conversions ----
    struct Job {
        old_ref: String, // "/media/images/foo.jpg"
        src: PathBuf,    // resolved source file
        new_name: String, // "12.jpg"
        new_ref: String,  // "/img/12.jpg"
        is_raster: bool,  // jpg/jpeg/png get resized; gif/svg copied
    }
    let mut jobs = Vec::new();
    for (i, r) in refs.iter().enumerate() {
        let base = r.rsplit('/').next().unwrap_or(r);
        let ext = base.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        let (new_ext, is_raster) = match ext.as_str() {
            "jpg" | "jpeg" => ("jpg", true),
            "png" => ("png", true),
            "gif" => ("gif", false),
            "svg" => ("svg", false),
            other => (Box::leak(other.to_string().into_boxed_str()) as &str, false),
        };
        let n = i + 1;
        let new_name = format!("{n}.{new_ext}");
        jobs.push(Job {
            old_ref: r.clone(),
            src: media.join("images").join(base),
            new_name: new_name.clone(),
            new_ref: format!("/img/{new_name}"),
            is_raster,
        });
    }

    // ---- process images in parallel ----
    let old_total = AtomicU64::new(0);
    let new_total = AtomicU64::new(0);
    let missing = AtomicU64::new(0);
    let map: Vec<MapEntry> = jobs
        .par_iter()
        .filter_map(|j| {
            if !j.src.exists() {
                missing.fetch_add(1, Ordering::Relaxed);
                return None;
            }
            let ob = fs::metadata(&j.src).map(|m| m.len()).unwrap_or(0);
            let dst = img_out.join(&j.new_name);
            let ok = if j.is_raster {
                Command::new("magick")
                    .arg(&j.src)
                    .args(["-resize", MAX_DIM, "-strip", "-interlace", "Plane", "-quality", QUALITY])
                    .arg(&dst)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            } else {
                fs::copy(&j.src, &dst).is_ok()
            };
            if !ok {
                // fall back to a plain copy so the reference still resolves
                let _ = fs::copy(&j.src, &dst);
            }
            let nb = fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
            old_total.fetch_add(ob, Ordering::Relaxed);
            new_total.fetch_add(nb, Ordering::Relaxed);
            Some(MapEntry { new: j.new_name.clone(), old: j.old_ref.clone(), old_bytes: ob, new_bytes: nb })
        })
        .collect();

    // ---- rewrite references in the IR ----
    let ref_to_new: BTreeMap<&str, &str> =
        jobs.iter().map(|j| (j.old_ref.as_str(), j.new_ref.as_str())).collect();
    for p in pages.iter_mut() {
        p.body_html = SRC_RE
            .replace_all(&p.body_html, |c: &regex::Captures| {
                let attr = if c[0].starts_with("href") { "href" } else { "src" };
                match ref_to_new.get(&c[1]) {
                    Some(nw) => format!("{attr}=\"{nw}\""),
                    None => format!("{attr}=\"{}\"", &c[1]),
                }
            })
            .into_owned();
    }

    // ---- write outputs ----
    let mut buf = String::with_capacity(pages.len() * 2048);
    for p in &pages {
        buf.push_str(&serde_json::to_string(p)?);
        buf.push('\n');
    }
    fs::write(out.join("pages.jsonl"), buf)?;
    for f in ["collections.json", "aliases.json"] {
        if ir_dir.join(f).exists() {
            fs::copy(ir_dir.join(f), out.join(f))?;
        }
    }
    fs::write(out.join("img_map.json"), serde_json::to_string_pretty(&map)?)?;

    let ot = old_total.load(Ordering::Relaxed);
    let nt = new_total.load(Ordering::Relaxed);
    eprintln!(
        "[images] processed {} images ({} missing). {:.1} MB -> {:.1} MB ({:.0}% smaller)",
        map.len(),
        missing.load(Ordering::Relaxed),
        ot as f64 / 1.048e6,
        nt as f64 / 1.048e6,
        if ot > 0 { 100.0 * (1.0 - nt as f64 / ot as f64) } else { 0.0 },
    );
    eprintln!("[images] wrote {}/pages.jsonl, img/, img_map.json", out.display());
    Ok(())
}

#[allow(dead_code)]
fn _p(_: &Path) {}
