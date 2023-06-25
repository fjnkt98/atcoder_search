use atcoder_search_libs::{
    api::{RangeFilterParameter, SearchResultResponse, SearchResultStats},
    solr::{
        core::{SolrCore, StandaloneSolrCore},
        model::*,
        query::{sanitize, EDisMaxQueryBuilder, Operator},
    },
    FieldList, ToQuery,
};
use axum::{
    async_trait,
    extract::{Extension, FromRequestParts},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, FixedOffset};
use http::request::Parts;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use serde_with::{serde_as, skip_serializing_none};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};
use tokio::time::Instant;
use validator::{Validate, ValidationError};

// ソート順に指定できるフィールドの集合
static VALID_SORT_OPTIONS: Lazy<HashSet<&str>> = Lazy::new(|| {
    HashSet::from([
        "start_at",
        "-start_at",
        "difficulty",
        "-difficulty",
        "-score",
    ])
});

// ソート順指定パラメータの値をバリデーションする関数
fn validate_sort_field(value: &str) -> Result<(), ValidationError> {
    if VALID_SORT_OPTIONS.contains(value) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid sort field"))
    }
}

// `facet`パラメータに指定できる値 => 実際にファセットカウントに使用するフィールドの名前
static FACET_FIELDS: Lazy<HashMap<&str, &str>> =
    Lazy::new(|| HashMap::from([("category", "category"), ("difficulty", "color")]));

// ファセットカウント指定パラメータの値をバリデーションする関数
fn validate_facet_fields(values: &Vec<String>) -> Result<(), ValidationError> {
    if values
        .iter()
        .all(|value| FACET_FIELDS.contains_key(value.as_str()))
    {
        Ok(())
    } else {
        Err(ValidationError::new("invalid facet field"))
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Validate, PartialEq, Eq, Clone)]
pub struct ProblemSearchParameter {
    #[validate(length(max = 200))]
    pub keyword: Option<String>,
    #[validate(range(min = 1, max = 200))]
    pub limit: Option<u32>,
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<FilterParameter>,
    #[validate(custom = "validate_sort_field")]
    pub sort: Option<String>,
    #[validate(custom = "validate_facet_fields")]
    pub facet: Option<Vec<String>>,
}

impl Default for ProblemSearchParameter {
    fn default() -> Self {
        Self {
            keyword: None,
            limit: None,
            page: None,
            filter: None,
            sort: None,
            facet: None,
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Validate, PartialEq, Eq, Clone)]
pub struct FilterParameter {
    category: Option<Vec<String>>,
    difficulty: Option<RangeFilterParameter>,
}

impl FilterParameter {
    pub fn to_query(&self) -> Vec<String> {
        let mut query = vec![];
        if let Some(categories) = &self.category {
            query.push(format!(
                "{{!tag=category}}category:({})",
                categories.iter().map(|c| sanitize(c)).join(" OR ")
            ));
        }
        if let Some(difficulty) = &self.difficulty {
            if let Some(range) = difficulty.to_range() {
                query.push(format!("{{!tag=difficulty}}difficulty:{}", range));
            }
        }

        query
    }
}

impl ToQuery for ProblemSearchParameter {
    fn to_query(&self) -> Vec<(String, String)> {
        let rows = self.limit.unwrap_or(20);
        let page = self.page.unwrap_or(1);
        let start = (page - 1) * rows;
        let keyword = self
            .keyword
            .as_ref()
            .map(|keyword| sanitize(keyword))
            .unwrap_or(String::from(""));
        let sort = self
            .sort
            .as_ref()
            .and_then(|sort| {
                if sort.starts_with("-") {
                    Some(format!("{} desc", &sort[1..]))
                } else {
                    Some(format!("{} asc", sort))
                }
            })
            .unwrap_or(String::from("problem_id asc"));
        let fq = self
            .filter
            .as_ref()
            .and_then(|filter| Some(filter.to_query()))
            .unwrap_or(vec![]);

        let facet = self
            .facet
            .as_ref()
            .and_then(|facet| {
                let mut facet_params: BTreeMap<&str, Value> = BTreeMap::new();
                for field in facet.iter() {
                    if let Some(facet_field) = FACET_FIELDS.get(field.as_str()) {
                        facet_params.insert(
                            field,
                            json!({
                                "type": "terms",
                                "field": facet_field,
                                "limit": -1,
                                "mincount": 0,
                                "sort": "index",
                                "domain": {
                                    "excludeTags": [field]
                                }
                            }),
                        );
                    }
                }
                serde_json::to_string(&facet_params).ok()
            })
            .unwrap_or(String::from(""));

        EDisMaxQueryBuilder::new()
            .facet(facet)
            .fl(ProblemResponse::field_list())
            .fq(&fq)
            .op(Operator::AND)
            .q(keyword)
            .q_alt("*:*")
            .qf("text_ja text_en text_1gram")
            .rows(rows)
            .sort(sort)
            .sow(true)
            .start(start)
            .build()
    }
}

pub struct ValidatedProblemSearchParameter<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for ValidatedProblemSearchParameter<T>
where
    T: DeserializeOwned + Validate + Serialize + Default + Clone,
    S: Send + Sync,
{
    type Rejection = (
        StatusCode,
        Json<SearchResultResponse<T, ProblemResponse, FacetCounts>>,
    );

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        let value: T = serde_structuredqs::from_str(query).map_err(|rejection| {
            tracing::error!("Parsing error: {}", rejection);
            (
                StatusCode::BAD_REQUEST,
                Json(
                    SearchResultResponse::<T, ProblemResponse, FacetCounts>::error(
                        T::default(),
                        format!("invalid format query string: [{}]", rejection),
                    ),
                ),
            )
        })?;

        value.validate().map_err(|rejection| {
            tracing::error!("Validation error: {}", rejection);
            (
                StatusCode::BAD_REQUEST,
                Json(
                    SearchResultResponse::<T, ProblemResponse, FacetCounts>::error(
                        value.clone(),
                        format!("Validation error: [{}]", rejection).replace('\n', ", "),
                    ),
                ),
            )
        })?;

        Ok(ValidatedProblemSearchParameter(value))
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, FieldList)]
pub struct ProblemResponse {
    pub problem_id: String,
    pub problem_title: String,
    pub problem_url: String,
    pub contest_id: String,
    pub contest_title: String,
    pub contest_url: String,
    pub difficulty: Option<i32>,
    pub color: Option<String>,
    #[serde_as(as = "FromSolrDateTime")]
    pub start_at: DateTime<FixedOffset>,
    pub duration: i64,
    pub rate_change: String,
    pub category: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacetCounts {
    count: u32,
    category: Option<SolrTermFacetCount>,
    difficulty: Option<SolrTermFacetCount>,
}

pub async fn search_problem(
    ValidatedProblemSearchParameter(params): ValidatedProblemSearchParameter<
        ProblemSearchParameter,
    >,
    Extension(core): Extension<Arc<StandaloneSolrCore>>,
) -> (
    StatusCode,
    Json<SearchResultResponse<ProblemSearchParameter, ProblemResponse, FacetCounts>>,
) {
    let start_process = Instant::now();

    let response: SolrSelectResponse<ProblemResponse, FacetCounts> =
        match core.select(&params.to_query()).await {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("request failed cause: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SearchResultResponse::error(params, "unexpected error")),
                );
            }
        };

    let time: u32 = Instant::now().duration_since(start_process).as_millis() as u32;
    let total: u32 = response.response.num_found;
    let count: u32 = response.response.docs.len() as u32;
    let rows: u32 = params.limit.unwrap_or(20);
    let index: u32 = (response.response.start / rows) + 1;
    let pages: u32 = (total + rows - 1) / rows;

    tracing::info!(
        target: "querylog",
        "elapsed_time={} hits={} params={}",
        time, total, serde_json::to_string(&params).unwrap_or(String::from(""))
    );

    let stats = SearchResultStats {
        time,
        total,
        index,
        count,
        pages,
        params,
        facet: response.facets,
    };

    (
        StatusCode::OK,
        Json(
            SearchResultResponse::<ProblemSearchParameter, ProblemResponse, FacetCounts> {
                stats,
                items: response.response.docs,
                message: None,
            },
        ),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_deserialize() {
        let query = "keyword=OR&facet=category,difficulty&filter.category=ABC,ARC&filter.difficulty.from=800&sort=-score";
        let params: ProblemSearchParameter = serde_structuredqs::from_str(query).unwrap();

        let expected = ProblemSearchParameter {
            keyword: Some(String::from("OR")),
            limit: None,
            page: None,
            filter: Some(FilterParameter {
                category: Some(vec![String::from("ABC"), String::from("ARC")]),
                difficulty: Some(RangeFilterParameter {
                    from: Some(800),
                    to: None,
                }),
            }),
            sort: Some(String::from("-score")),
            facet: Some(vec![String::from("category"), String::from("difficulty")]),
        };

        assert_eq!(params, expected);
    }

    #[test]
    fn empty_query_string() {
        let params: ProblemSearchParameter = serde_structuredqs::from_str("").unwrap();
        let expected = ProblemSearchParameter {
            keyword: None,
            limit: None,
            page: None,
            filter: None,
            sort: None,
            facet: None,
        };

        assert_eq!(params, expected);
    }
}
