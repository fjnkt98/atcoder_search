use atcoder_search_libs::{
    api::{
        deserialize_optional_comma_separated, RangeFilterParameter, SearchResultResponse,
        SearchResultStats,
    },
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
use std::{
    collections::{BTreeMap, HashSet},
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

static VALID_RANGE_FACET_FIELDS: Lazy<HashSet<&str>> = Lazy::new(|| {
    HashSet::from([
        "birth_year",
        "highest_rating",
        "join_count",
        "rank",
        "rating",
        "wins",
    ])
});
static VALID_TERM_FACET_FIELDS: Lazy<HashSet<&str>> =
    Lazy::new(|| HashSet::from(["color", "highest_color", "affiliation", "country", "crown"]));
static VALID_FACET_FIELDS: Lazy<HashSet<&str>> = Lazy::new(|| {
    HashSet::from_iter(
        VALID_RANGE_FACET_FIELDS
            .iter()
            .cloned()
            .chain(VALID_TERM_FACET_FIELDS.iter().cloned()),
    )
});
fn validate_facet_fields(values: &Vec<String>) -> Result<(), ValidationError> {
    if values
        .iter()
        .all(|value| VALID_FACET_FIELDS.contains(value.as_str()))
    {
        Ok(())
    } else {
        Err(ValidationError::new("invalid facet field"))
    }
}

#[derive(Debug, Serialize, Deserialize, Validate, PartialEq, Eq, Clone)]
pub struct UserSearchParameter {
    #[validate(length(max = 200))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[validate(range(min = 1, max = 200))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[validate(range(min = 1))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<FilterParameter>,
    #[validate(custom = "validate_sort_field")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[validate(custom = "validate_facet_fields")]
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_comma_separated"
    )]
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

#[derive(Debug, Serialize, Deserialize, Validate, PartialEq, Eq, Clone)]
pub struct FilterParameter {
    #[serde(skip_serializing_if = "Option::is_none")]
    rating: Option<RangeFilterParameter>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_comma_separated"
    )]
    color: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_comma_separated"
    )]
    highest_color: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    highest_rating: Option<RangeFilterParameter>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_comma_separated"
    )]
    affiliation: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    birth_year: Option<RangeFilterParameter>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_comma_separated"
    )]
    country: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_comma_separated"
    )]
    crown: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    join_count: Option<RangeFilterParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rank: Option<RangeFilterParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wins: Option<RangeFilterParameter>,
}

impl FilterParameter {
    pub fn to_query(&self) -> Vec<String> {
        let mut query = vec![];

        if let Some(fq) = self
            .rating
            .as_ref()
            .and_then(|rating| rating.to_range())
            .and_then(|rating| Some(format!("{{!tag=rating}}rating:{}", rating)))
        {
            query.push(fq);
        }
        if let Some(fq) = self.color.as_ref().and_then(|color| {
            Some(format!(
                "{{!tag=color}}color:({})",
                color.iter().map(|color| sanitize(color)).join(" OR ")
            ))
        }) {
            query.push(fq);
        }
        if let Some(fq) = self
            .highest_rating
            .as_ref()
            .and_then(|highest_rating| highest_rating.to_range())
            .and_then(|highest_rating| {
                Some(format!(
                    "{{!tag=highest_rating}}highest_rating:{}",
                    highest_rating
                ))
            })
        {
            query.push(fq);
        }
        if let Some(fq) = self.highest_color.as_ref().and_then(|highest_color| {
            Some(format!(
                "{{!tag=highest_color}}highest_color:({})",
                highest_color
                    .iter()
                    .map(|highest_color| sanitize(highest_color))
                    .join(" OR ")
            ))
        }) {
            query.push(fq);
        }
        if let Some(fq) = self.affiliation.as_ref().and_then(|affiliation| {
            Some(format!(
                "{{!tag=affiliation}}affiliation:({})",
                affiliation
                    .iter()
                    .map(|affiliation| sanitize(affiliation))
                    .join(" OR ")
            ))
        }) {
            query.push(fq);
        }
        if let Some(fq) = self
            .birth_year
            .as_ref()
            .and_then(|birth_year| birth_year.to_range())
            .and_then(|birth_year| Some(format!("{{!tag=birth_year}}birth_year:{}", birth_year)))
        {
            query.push(fq);
        }
        if let Some(fq) = self.country.as_ref().and_then(|country| {
            Some(format!(
                "{{!tag=country}}country:({})",
                country.iter().map(|country| sanitize(country)).join(" OR ")
            ))
        }) {
            query.push(fq);
        }
        if let Some(fq) = self.crown.as_ref().and_then(|crown| {
            Some(format!(
                "{{!tag=crown}}crown:({})",
                crown.iter().map(|crown| sanitize(crown)).join(" OR ")
            ))
        }) {
            query.push(fq);
        }
        if let Some(fq) = self
            .join_count
            .as_ref()
            .and_then(|join_count| join_count.to_range())
            .and_then(|join_count| Some(format!("{{!tag=join_count}}join_count:{}", join_count)))
        {
            query.push(fq);
        }
        if let Some(fq) = self
            .rank
            .as_ref()
            .and_then(|rank| rank.to_range())
            .and_then(|rank| Some(format!("{{!tag=rank}}rank:{}", rank)))
        {
            query.push(fq);
        }
        if let Some(fq) = self
            .wins
            .as_ref()
            .and_then(|wins| wins.to_range())
            .and_then(|wins| Some(format!("{{!tag=wins}}wins:{}", wins)))
        {
            query.push(fq);
        }

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
            .unwrap_or(String::from(""));
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
                    if VALID_TERM_FACET_FIELDS.contains(field.as_str()) {
                        facet_params.insert(
                            field,
                            json!({
                                "type": "terms",
                                "field": field,
                                "limit": -1,
                                "mincount": 0,
                                "domain": {
                                    "excludeTags": [field]
                                }
                            }),
                        );
                    } else if VALID_RANGE_FACET_FIELDS.contains(field.as_str()) {
                        facet_params.insert(
                            field,
                            json!({
                                "type": "range",
                                "field": field,
                                "start": 0,
                                "end": 4000,
                                "gap": 400,
                                "other": "all",
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
    color: Option<SolrTermFacetCount>,
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
