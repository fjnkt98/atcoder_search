use atcoder_search_libs::{
    api::{FieldFacetCount, RangeFacetCountKind},
    solr::model::*,
    FieldList,
};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_with::serde_as;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct SearchResultResponse {
    pub stats: SearchResultStats,
    pub items: Vec<ResponseDocument>,
    pub message: Option<String>,
}

impl SearchResultResponse {
    pub fn error(params: &impl Serialize, message: impl ToString) -> Self {
        Self {
            stats: SearchResultStats {
                time: 0,
                total: 0,
                index: 0,
                pages: 0,
                count: 0,
                params: json!(params),
                facet: BTreeMap::new(),
            },
            items: Vec::new(),
            message: Some(message.to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResultStats {
    pub time: u32,
    pub total: u32,
    pub index: u32,
    pub pages: u32,
    pub count: u32,
    pub params: Value,
    pub facet: BTreeMap<String, FacetResultKind>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, FieldList)]
pub struct ResponseDocument {
    pub problem_id: String,
    pub problem_title: String,
    pub problem_url: String,
    pub contest_id: String,
    pub contest_title: String,
    pub contest_url: String,
    pub difficulty: Option<i32>,
    #[serde_as(as = "FromSolrDateTime")]
    pub start_at: DateTime<FixedOffset>,
    pub duration: i64,
    pub rate_change: String,
    pub category: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FacetResultKind {
    Field(FieldFacetCount),
    Range(RangeFacetCountKind),
}
