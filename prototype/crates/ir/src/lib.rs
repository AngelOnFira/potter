//! Shared intermediate-representation (IR) types for the clay-knowledge prototype.
//!
//! The pipeline is: digitalfire HTML  --(clay-extract)-->  IR (`pages.jsonl`)
//! --(clay-app loads + indexes with tantivy)-->  rendered pages + search.
//!
//! Keeping the IR as plain serde structs means the same types are shared by the
//! extractor (writes them) and the Leptos app (reads them), and they serialize
//! cleanly to JSONL so the corpus is inspectable and tool-agnostic.

use serde::{Deserialize, Serialize};

/// One reference page (an oxide, material, glossary term, article, …).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Page {
    /// Collection/type, e.g. `"oxide"`, `"material"`, `"glossary"`.
    pub collection: String,
    /// URL-safe slug unique within the collection, e.g. `"al2o3"`.
    pub slug: String,
    /// App route, e.g. `"/oxide/al2o3"`.
    pub path: String,
    /// Human title from the page `<title>`, e.g. `"Al2O3 (Aluminum Oxide, Alumina)"`.
    pub title: String,
    /// Short plain-text summary (first ~240 chars of body text).
    pub summary: String,
    /// Cleaned, link-rewritten, sanitized content HTML (boilerplate stripped).
    pub body_html: String,
    /// Plain text of the body, used to build the search index.
    pub text: String,
    /// A handful of internal "related" links discovered in the body.
    pub related: Vec<Link>,
    /// Original digitalfire.com URL this page was derived from (provenance).
    pub source_url: String,
    /// Structured oxide chemistry (materials/minerals), matched from the
    /// millandr121 dataset. `None` when no structured record matched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chemistry: Option<Chemistry>,
}

/// Typed oxide chemistry for a material or mineral (oxide analysis + unity formula).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chemistry {
    pub analysis: Vec<OxideRow>,
    /// Key/value reference properties (used for oxides: expansion, softening point…).
    #[serde(default)]
    pub properties: Vec<KeyVal>,
    pub oxide_weight: Option<f64>,
    pub formula_weight: Option<f64>,
    /// Chemical formula string (minerals), e.g. "NaAlSi3O8 to CaAl2Si2O8".
    pub formula: Option<String>,
    pub alternate_names: Option<String>,
    /// Where the structured data came from (attribution/provenance).
    pub data_source: String,
}

/// A labelled reference property (e.g. "Frit Softening Point" → "2130C").
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyVal {
    pub key: String,
    pub val: String,
}

/// One row of an oxide analysis: weight-% and unity-formula (UMF) value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OxideRow {
    pub oxide: String,
    pub analysis_pct: Option<f64>,
    pub formula: Option<f64>,
    pub tolerance: Option<String>,
}

/// A labelled hyperlink (internal app route or external URL).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link {
    pub label: String,
    pub href: String,
}

/// Summary of one collection for the sidebar / home page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectionInfo {
    /// Slug, e.g. `"oxide"`.
    pub name: String,
    /// Display title, e.g. `"Oxides"`.
    pub title: String,
    /// Number of pages in this collection.
    pub count: usize,
}
