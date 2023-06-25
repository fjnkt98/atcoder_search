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
use http::request::Parts;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use serde_with::skip_serializing_none;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};
use tokio::time::Instant;
use validator::{Validate, ValidationError};

static VALID_SORT_OPTIONS: Lazy<HashSet<&str>> = Lazy::new(|| {
    HashSet::from([
        "-birth_year",
        "-highest_rating",
        "-join_count",
        "-rank",
        "-rating",
        "-wins",
        "birth_year",
        "highest_rating",
        "join_count",
        "rank",
        "rating",
        "wins",
    ])
});
fn validate_sort_field(value: &str) -> Result<(), ValidationError> {
    if VALID_SORT_OPTIONS.contains(value) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid sort field"))
    }
}

// `facet`パラメータに指定できる値 => 実際にファセットカウントに使用するフィールドの名前
static FACET_FIELDS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    HashMap::from([
        ("rating", "color"),
        ("highest_rating", "highest_color"),
        ("birth_year", "period"),
        ("join_count", "join_count_grade"),
        ("affiliation", "affiliation"),
        ("country", "country"),
        ("crown", "crown"),
    ])
});

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
pub struct UserSearchParameter {
    #[validate(length(max = 200))]
    pub keyword: Option<String>,
    #[validate(range(min = 1, max = 200))]
    pub limit: Option<u32>,
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    pub filter: Option<FilterParameter>,
    #[validate(custom = "validate_sort_field")]
    pub sort: Option<String>,
    #[validate(custom = "validate_facet_fields")]
    pub facet: Option<Vec<String>>,
}

impl Default for UserSearchParameter {
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
    rating: Option<RangeFilterParameter>,
    color: Option<Vec<String>>,
    highest_color: Option<Vec<String>>,
    highest_rating: Option<RangeFilterParameter>,
    affiliation: Option<Vec<String>>,
    birth_year: Option<RangeFilterParameter>,
    country: Option<Vec<String>>,
    crown: Option<Vec<String>>,
    join_count: Option<RangeFilterParameter>,
    rank: Option<RangeFilterParameter>,
    wins: Option<RangeFilterParameter>,
}

fn term_filtering(
    field_name: &'static str,
    value: &Option<Vec<String>>,
    container: &mut Vec<String>,
) -> () {
    if let Some(fq) = value
        .as_ref()
        .and_then(|value| {
            if value.len() == 0 {
                None
            } else {
                Some(value.iter().map(|element| sanitize(element)).join(" OR "))
            }
        })
        .and_then(|expr| {
            Some(format!(
                "{{tag!={field_name}}}{field_name}:({expr})",
                field_name = field_name,
                expr = expr
            ))
        })
    {
        container.push(fq)
    }
}

fn range_filtering(
    field_name: &'static str,
    value: &Option<RangeFilterParameter>,
    container: &mut Vec<String>,
) -> () {
    if let Some(fq) = value
        .as_ref()
        .and_then(|value| value.to_range())
        .and_then(|expr| {
            Some(format!(
                "{{!tag={field_name}}}{field_name}:{expr}",
                field_name = field_name,
                expr = expr
            ))
        })
    {
        container.push(fq)
    }
}

impl FilterParameter {
    pub fn to_query(&self) -> Vec<String> {
        let mut query = vec![];

        term_filtering("color", &self.color, &mut query);
        term_filtering("highest_color", &self.highest_color, &mut query);
        term_filtering("affiliation", &self.affiliation, &mut query);
        term_filtering("country", &self.country, &mut query);
        term_filtering("crown", &self.crown, &mut query);

        range_filtering("rating", &self.rating, &mut query);
        range_filtering("highest_rating", &self.highest_rating, &mut query);
        range_filtering("birth_year", &self.birth_year, &mut query);
        range_filtering("join_count", &self.join_count, &mut query);
        range_filtering("rank", &self.rank, &mut query);
        range_filtering("wins", &self.wins, &mut query);

        query
    }
}

impl ToQuery for UserSearchParameter {
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
            .unwrap_or(String::from("rank asc"));
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
            .unwrap_or(String::new());

        EDisMaxQueryBuilder::new()
            .facet(facet)
            .fl(UserResponse::field_list())
            .fq(&fq)
            .op(Operator::AND)
            .q(keyword)
            .q_alt("*:*")
            .qf("user_name")
            .rows(rows)
            .sort(sort)
            .sow(true)
            .start(start)
            .build()
    }
}

#[derive(Debug, Serialize, Deserialize, FieldList)]
pub struct UserResponse {
    pub user_name: String,
    pub rating: i32,
    pub color: String,
    pub highest_rating: i32,
    pub highest_color: String,
    pub affiliation: Option<String>,
    pub birth_year: Option<i32>,
    pub country: Option<String>,
    pub crown: Option<String>,
    pub join_count: i32,
    pub rank: i32,
    pub wins: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacetCounts {
    count: u32,
    rating: Option<SolrTermFacetCount>,
    highest_rating: Option<SolrTermFacetCount>,
    birth_year: Option<SolrTermFacetCount>,
    join_count: Option<SolrTermFacetCount>,
    affiliation: Option<SolrTermFacetCount>,
    country: Option<SolrTermFacetCount>,
    crown: Option<SolrTermFacetCount>,
}

pub struct ValidatedUserSearchParameter<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for ValidatedUserSearchParameter<T>
where
    T: DeserializeOwned + Validate + Serialize + Default + Clone,
    S: Send + Sync,
{
    type Rejection = (
        StatusCode,
        Json<SearchResultResponse<T, UserResponse, FacetCounts>>,
    );

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        let value: T = serde_structuredqs::from_str(query).map_err(|rejection| {
            tracing::error!("Parsing error: {}", rejection);
            (
                StatusCode::BAD_REQUEST,
                Json(SearchResultResponse::<T, UserResponse, FacetCounts>::error(
                    T::default(),
                    format!("invalid format query string: [{}]", rejection),
                )),
            )
        })?;

        value.validate().map_err(|rejection| {
            tracing::error!("Validation error: {}", rejection);
            (
                StatusCode::BAD_REQUEST,
                Json(SearchResultResponse::<T, UserResponse, FacetCounts>::error(
                    value.clone(),
                    format!("Validation error: [{}]", rejection).replace('\n', ", "),
                )),
            )
        })?;

        Ok(ValidatedUserSearchParameter(value))
    }
}

pub async fn search_user(
    ValidatedUserSearchParameter(params): ValidatedUserSearchParameter<UserSearchParameter>,
    Extension(core): Extension<Arc<StandaloneSolrCore>>,
) -> (
    StatusCode,
    Json<SearchResultResponse<UserSearchParameter, UserResponse, FacetCounts>>,
) {
    let start_process = Instant::now();

    let response: SolrSelectResponse<UserResponse, FacetCounts> =
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

    // tracing::info!(
    //     target: "querylog",
    //     "elapsed_time={} hits={} params={}",
    //     time, total, serde_json::to_string(&params).unwrap_or(String::from(""))
    // );

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
            SearchResultResponse::<UserSearchParameter, UserResponse, FacetCounts> {
                stats,
                items: response.response.docs,
                message: None,
            },
        ),
    )
}
