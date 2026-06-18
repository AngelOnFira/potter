//! Server-side application state: the loaded IR + the tantivy index.

use std::collections::HashMap;
use std::sync::OnceLock;

use clay_ir::{CollectionInfo, Page};

use crate::app::{Card, COLLECTIONS};
use crate::search::SearchIndex;

pub struct AppState {
    pub pages: HashMap<String, Page>,
    pub aliases: HashMap<String, String>,
    pub by_collection: HashMap<String, Vec<Card>>,
    pub collections: Vec<CollectionInfo>,
    pub total: usize,
    pub featured: Vec<Card>,
    pub search: SearchIndex,
}

static STATE: OnceLock<AppState> = OnceLock::new();

pub fn init() {
    if STATE.get().is_some() {
        return;
    }
    let dir = std::env::var("CLAY_IR_DIR")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../data/ir").to_string());
    let pages_path = format!("{dir}/pages.jsonl");
    let raw = std::fs::read_to_string(&pages_path)
        .unwrap_or_else(|e| panic!("clay-app: cannot read {pages_path}: {e}\nRun the extractor first."));

    let mut pages = HashMap::new();
    let mut by_collection: HashMap<String, Vec<Card>> = HashMap::new();
    let mut list: Vec<Page> = Vec::new();

    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let p: Page = match serde_json::from_str(line) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[state] skip bad line: {e}");
                continue;
            }
        };
        let card = Card {
            title: p.title.clone(),
            path: p.path.clone(),
            collection: p.collection.clone(),
            summary: p.summary.clone(),
        };
        by_collection.entry(p.collection.clone()).or_default().push(card);
        list.push(p.clone());
        pages.insert(p.path.clone(), p);
    }

    for v in by_collection.values_mut() {
        v.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    }

    let collections: Vec<CollectionInfo> = COLLECTIONS
        .iter()
        .filter_map(|(name, title)| {
            let count = by_collection.get(*name).map(|v| v.len()).unwrap_or(0);
            (count > 0).then(|| CollectionInfo {
                name: name.to_string(),
                title: title.to_string(),
                count,
            })
        })
        .collect();

    let total = pages.len();
    let featured: Vec<Card> = by_collection
        .get("glossary")
        .map(|v| v.iter().take(8).cloned().collect())
        .unwrap_or_default();

    let aliases: HashMap<String, String> = std::fs::read_to_string(format!("{dir}/aliases.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let search = SearchIndex::build(&list).expect("clay-app: failed to build search index");

    let _ = STATE.set(AppState {
        pages,
        aliases,
        by_collection,
        collections,
        total,
        featured,
        search,
    });
    eprintln!("[state] loaded {total} pages and built the search index");
}

pub fn get() -> &'static AppState {
    STATE.get().expect("clay-app: state not initialized (call state::init() first)")
}
