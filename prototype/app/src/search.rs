//! In-memory tantivy full-text index over the IR (server-side only).

use crate::app::SearchHit;
use clay_ir::Page;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, STORED, STRING, TEXT};
use tantivy::snippet::SnippetGenerator;
use tantivy::{doc, Index, IndexReader, TantivyDocument};

pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    title: Field,
    body: Field,
    f_path: Field,
    f_collection: Field,
    f_summary: Field,
}

impl SearchIndex {
    pub fn build(pages: &[Page]) -> tantivy::Result<Self> {
        let mut sb = Schema::builder();
        let title = sb.add_text_field("title", TEXT | STORED);
        let body = sb.add_text_field("body", TEXT | STORED);
        let f_path = sb.add_text_field("path", STORED);
        let f_collection = sb.add_text_field("collection", STRING | STORED);
        let f_summary = sb.add_text_field("summary", STORED);
        let schema = sb.build();

        let index = Index::create_in_ram(schema);
        let mut writer = index.writer(80_000_000)?;
        for p in pages {
            writer.add_document(doc!(
                title => p.title.clone(),
                body => p.text.clone(),
                f_path => p.path.clone(),
                f_collection => p.collection.clone(),
                f_summary => p.summary.clone(),
            ))?;
        }
        writer.commit()?;
        let reader = index.reader()?;
        Ok(Self { index, reader, title, body, f_path, f_collection, f_summary })
    }

    pub fn query(&self, raw_q: &str, limit: usize) -> Vec<SearchHit> {
        let q = sanitize_query(raw_q);
        if q.is_empty() {
            return Vec::new();
        }
        let searcher = self.reader.searcher();
        let mut parser = QueryParser::for_index(&self.index, vec![self.title, self.body]);
        parser.set_field_boost(self.title, 3.0);
        let query = match parser.parse_query(&q) {
            Ok(query) => query,
            Err(_) => return Vec::new(),
        };
        let collector = TopDocs::with_limit(limit).order_by_score();
        let top = match searcher.search(&*query, &collector) {
            Ok(top) => top,
            Err(_) => return Vec::new(),
        };
        let snippet_gen = SnippetGenerator::create(&searcher, &*query, self.body).ok();

        let mut hits = Vec::with_capacity(top.len());
        for (_score, addr) in top {
            let doc: TantivyDocument = match searcher.doc(addr) {
                Ok(doc) => doc,
                Err(_) => continue,
            };
            let get = |f: Field| -> String {
                doc.get_first(f).and_then(|v| v.as_str()).unwrap_or("").to_string()
            };
            let snippet = snippet_gen
                .as_ref()
                .map(|g| g.snippet_from_doc(&doc).to_html())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| get(self.f_summary));
            hits.push(SearchHit {
                title: get(self.title),
                path: get(self.f_path),
                collection: get(self.f_collection),
                snippet,
            });
        }
        hits
    }
}

/// Drop query characters that would break the tantivy query parser; append a
/// trailing `*` so the in-progress last word matches as a prefix (as-you-type).
fn sanitize_query(q: &str) -> String {
    let cleaned: String = q
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect();
    let mut terms: Vec<String> = cleaned.split_whitespace().map(|s| s.to_lowercase()).collect();
    if let Some(last) = terms.last_mut() {
        if last.len() >= 2 {
            last.push('*'); // prefix-match the final term
        }
    }
    terms.join(" ")
}
