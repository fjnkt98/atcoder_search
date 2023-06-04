use crate::modules::models::response::{ResponseDocument, SearchResultResponse};
use atcoder_search_libs::{
    solr::query::{sanitize, EDisMaxQueryBuilder, Operator},
    FieldList, ToQueryParameter,
};
use axum::{async_trait, extract::FromRequestParts, http::StatusCode, Json};
use http::request::Parts;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet},
};
use validator::{Validate, ValidationError};

static VALID_SORT_OPTIONS: Lazy<HashSet<&str>> = Lazy::new(|| {
    HashSet::from([
        "start_at",
        "-start_at",
        "difficulty",
        "-difficulty",
        "-score",
    ])
});

static VALID_FACET_FIELDS: Lazy<HashSet<&str>> =
    Lazy::new(|| HashSet::from(["category", "difficulty"]));

fn validate_sort_field(value: &str) -> Result<(), ValidationError> {
    if VALID_SORT_OPTIONS.contains(value) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid sort field"))
    }
}

fn validate_facet_fields(values: &str) -> Result<(), ValidationError> {
    if values
        .split(',')
        .into_iter()
        .all(|value| VALID_FACET_FIELDS.contains(value.trim()))
    {
        Ok(())
    } else {
        Err(ValidationError::new("invalid facet field"))
    }
}

#[derive(Debug, Serialize, Deserialize, Validate, PartialEq, Eq)]
pub struct SearchQueryParameters {
    #[validate(length(max = 200))]
    #[serde(deserialize_with = "sanitize_keyword")]
    pub keyword: Option<String>,
    #[validate(range(min = 1, max = 200))]
    pub limit: Option<u32>,
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    #[serde(rename = "filter.category")]
    pub filter_category: Option<String>,
    #[serde(rename = "filter.difficulty.from")]
    pub filter_difficulty_from: Option<u32>,
    #[serde(rename = "filter.difficulty.to")]
    pub filter_difficulty_to: Option<u32>,
    #[validate(custom = "validate_sort_field")]
    pub sort: Option<String>,
    #[validate(custom = "validate_facet_fields")]
    pub facet: Option<String>,
}

fn sanitize_keyword<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = String::deserialize(deserializer)?;
    Ok(Some(sanitize(&v)))
}

impl ToQueryParameter for SearchQueryParameters {
    fn to_query(&self) -> Vec<(String, String)> {
        let rows = self.limit.unwrap_or(20);
        let page = self.page.unwrap_or(1);
        let start = (page - 1) * rows;
        let keyword = self
            .keyword
            .as_ref()
            .and_then(|keyword| Some(Cow::from(keyword)))
            .unwrap_or(Cow::Borrowed(""));

        let mut builder = EDisMaxQueryBuilder::new();
        builder = builder
            .rows(rows)
            .start(start)
            .fl(ResponseDocument::field_list())
            .q(keyword)
            .qf("text_ja text_en text_1gram")
            .q_alt("*:*")
            .op(Operator::AND)
            .sow(true);

        if let Some(sort) = &self.sort {
            if sort.starts_with("-") {
                builder = builder.sort(format!("{} desc", &sort[1..]));
            } else {
                builder = builder.sort(format!("{} asc", sort));
            }
        }

        if let Some(categories) = &self.filter_category {
            let expr = categories.split(',').into_iter().map(sanitize).join(" OR ");
            builder = builder.fq(format!("{{!tag=category}}category:({})", expr));
        }

        let difficulty_from = self
            .filter_difficulty_from
            .and_then(|from| Some(from.to_string()))
            .unwrap_or(String::from("*"));
        let difficulty_to = self
            .filter_difficulty_to
            .and_then(|to| Some(to.to_string()))
            .unwrap_or(String::from("*"));
        if difficulty_from != "*" || difficulty_to != "*" {
            builder = builder.fq(format!(
                "{{!tag=difficulty}}difficulty:[{} TO {}}}",
                difficulty_from, difficulty_to
            ));
        }

        if let Some(facet) = &self.facet {
            let mut facet_params: BTreeMap<&str, Value> = BTreeMap::new();
            for field in facet.split(',') {
                match field {
                    "category" => {
                        facet_params.insert(
                            field,
                            json!({
                                "type": "terms",
                                "field": "category",
                                "limit": -1,
                                "mincount": 0,
                                "domain": {
                                    "excludeTags": ["category"]
                                }
                            }),
                        );
                    }
                    "difficulty" => {
                        facet_params.insert(
                            field,
                            json!({
                                "type": "range",
                                "field": "difficulty",
                                "start": 0,
                                "end": 4000,
                                "gap": 400,
                                "other": "all",
                                "domain": {
                                    "excludeTags": ["difficulty"]
                                }
                            }),
                        );
                    }
                    _ => {}
                };
            }
            let facet = serde_json::to_string(&facet_params).unwrap_or_else(|e| {
                tracing::warn!("failed to serialize json.facet parameters cause: [{:?}]", e);
                String::new()
            });
            builder = builder.facet(facet);
        }

        builder.build()
    }
}

pub struct ValidatedSearchQueryParameters<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for ValidatedSearchQueryParameters<T>
where
    T: DeserializeOwned + Validate + Serialize,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<SearchResultResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        let value: T = serde_urlencoded::from_str(query).map_err(|rejection| {
            tracing::error!("Parsing error: {}", rejection);
            (
                StatusCode::BAD_REQUEST,
                Json(SearchResultResponse::error(
                    &Value::Null,
                    format!("invalid format query string: [{}]", rejection),
                )),
            )
        })?;

        value.validate().map_err(|rejection| {
            tracing::error!("Validation error: {}", rejection);
            (
                StatusCode::BAD_REQUEST,
                Json(SearchResultResponse::error(
                    &value,
                    format!("Validation error: [{}]", rejection).replace('\n', ", "),
                )),
            )
        })?;

        Ok(ValidatedSearchQueryParameters(value))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_deserialize() {
        let query = "keyword=OR";
        let params: SearchQueryParameters = serde_urlencoded::from_str(query).unwrap();

        assert_eq!(params.keyword, Some(String::from("\\OR")));
    }
}
