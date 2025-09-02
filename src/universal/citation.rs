use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct Citation {
    pub cited_text: Option<String>,
    pub position: CitationPosition,
    pub source: CitationSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CitationPosition {
    /// Character-based positioning (most common)
    CharRange { start: i64, end: i64 },
    /// Page-based positioning (PDFs)
    PageRange { start_page: i64, end_page: i64 },
    /// Block-based positioning (structured content)
    BlockRange { start_block: i64, end_block: i64 },
    /// Search result index
    SearchIndex { index: i64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
pub struct CitationSource {
    pub url: Option<String>,
    pub title: Option<String>,
    pub document_title: Option<String>,
    pub document_index: Option<i64>,
    pub license: Option<String>,
    pub source_type: SourceType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/typescript/")]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Web,
    Document,
    Code,
    SearchResult,
}
