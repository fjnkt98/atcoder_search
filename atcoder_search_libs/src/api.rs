use crate::solr::model::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::string::ToString;

pub trait ToQueryParameter {
    fn to_query(&self) -> Vec<(String, String)>;
}

pub trait FieldList {
    fn field_list() -> &'static str;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacetCountElement {
    pub key: String,
    pub count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldFacetCount {
    pub counts: Vec<FacetCountElement>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RangeFacetCountElement {
    pub begin: String,
    pub end: String,
    pub count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RangeFacetCount<T: Sync + Send + Clone + ToString> {
    pub counts: Vec<RangeFacetCountElement>,
    pub start: T,
    pub end: T,
    pub gap: T,
    pub before: Option<u32>,
    pub after: Option<u32>,
    pub between: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RangeFacetCountKind {
    Integer(RangeFacetCount<i64>),
    Float(RangeFacetCount<f64>),
    DateTime(RangeFacetCount<String>),
}

impl From<Vec<(String, u32)>> for FieldFacetCount {
    fn from(counts: Vec<(String, u32)>) -> FieldFacetCount {
        FieldFacetCount {
            counts: counts
                .into_iter()
                .map(|(key, count)| FacetCountElement { key, count })
                .collect(),
        }
    }
}

impl<T: Sync + Send + Clone + ToString> From<SolrRangeFacet<T>> for RangeFacetCount<T> {
    fn from(facet: SolrRangeFacet<T>) -> RangeFacetCount<T> {
        let mut counts = facet.counts.clone();
        counts.push((facet.end.to_string(), 0));

        RangeFacetCount {
            counts: counts
                .into_iter()
                .tuples()
                .map(|(begin, end)| RangeFacetCountElement {
                    begin: begin.0,
                    end: end.0,
                    count: begin.1,
                })
                .collect(),
            start: facet.start,
            end: facet.end,
            gap: facet.gap,
            before: facet.before,
            after: facet.after,
            between: facet.between,
        }
    }
}
