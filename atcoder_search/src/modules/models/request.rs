use crate::modules::models::response::{ResponseDocument, SearchResultResponse};
use atcoder_search_libs::{
    solr::query::{sanitize, EDisMaxQueryBuilder, Operator},
    FieldList, ToQueryParameter,
};
use axum::{async_trait, extract::FromRequestParts, http::StatusCode, Json};
use http::request::Parts;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
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

// 絞り込みに指定できるカテゴリの集合
static VALID_CATEGORY_OPTIONS: Lazy<HashSet<&str>> = Lazy::new(|| {
    HashSet::from([
        "ABC",
        "ARC",
        "AGC",
        "AHC",
        "AGC-Like",
        "ABC-Like",
        "ARC-Like",
        "PAST",
        "JOI",
        "JAG",
        "Marathon",
        "Other Sponsored",
        "Other Contests",
    ])
});

// ファセットカウントに指定できるフィールドの集合
static VALID_FACET_FIELDS: Lazy<HashSet<&str>> =
    Lazy::new(|| HashSet::from(["category", "difficulty"]));

// ソート順指定パラメータの値をバリデーションする関数
fn validate_sort_field(value: &str) -> Result<(), ValidationError> {
    if VALID_SORT_OPTIONS.contains(value) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid sort field"))
    }
}

// カテゴリ絞り込みパラメータの値をバリデーションする関数
fn validate_category_filtering(values: &Vec<String>) -> Result<(), ValidationError> {
    if values
        .iter()
        .all(|value| VALID_CATEGORY_OPTIONS.contains(value.as_str()))
    {
        Ok(())
    } else {
        Err(ValidationError::new("invalid category field"))
    }
}

// ファセットカウント指定パラメータの値をバリデーションする関数
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
pub struct SearchQueryParameters {
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
    pub filter: Option<FilterParameters>,
    #[validate(custom = "validate_sort_field")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[validate(custom = "validate_facet_fields")]
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "comma_separated_values"
    )]
    pub facet: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Validate, PartialEq, Eq, Clone)]
pub struct FilterParameters {
    #[validate(custom = "validate_category_filtering")]
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "comma_separated_values"
    )]
    category: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    difficulty: Option<RangeFilterParameter>,
}

#[derive(Debug, Serialize, Deserialize, Validate, PartialEq, Eq, Clone)]
pub struct RangeFilterParameter {
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<i32>,
}

impl RangeFilterParameter {
    pub fn to_range(&self) -> Option<String> {
        if self.from.is_none() && self.to.is_none() {
            return None;
        }

        let from = &self
            .from
            .and_then(|from| Some(from.to_string()))
            .unwrap_or(String::from("*"));
        let to = &self
            .to
            .and_then(|to| Some(to.to_string()))
            .unwrap_or(String::from("*"));
        Some(format!("[{} TO {}}}", from, to))
    }
}

// カンマ区切りの文字列フィールドをベクタに変換するカスタムデシリアライズ関数
fn comma_separated_values<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let values = value
        .split(',')
        .into_iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(String::from)
        .collect();

    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(values))
    }
}

impl ToQueryParameter for SearchQueryParameters {
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
                    match field.as_str() {
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
                serde_json::to_string(&facet_params).ok()
            })
            .unwrap_or(String::from(""));

        EDisMaxQueryBuilder::new()
            .facet(facet)
            .fl(ResponseDocument::field_list())
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

impl FilterParameters {
    pub fn to_query(&self) -> Vec<String> {
        let mut query = vec![];
        if let Some(categories) = &self.category {
            query.push(format!(
                "{{!tag=category}}category:({})",
                categories.join(" OR ")
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
        let value: T = serde_structuredqs::from_str(query).map_err(|rejection| {
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
        let query = "keyword=OR&facet=category,difficulty&filter.category=ABC,ARC&filter.difficulty.from=800&sort=-score";
        let params: SearchQueryParameters = serde_structuredqs::from_str(query).unwrap();

        let expected = SearchQueryParameters {
            keyword: Some(String::from("\\OR")),
            limit: None,
            page: None,
            filter: Some(FilterParameters {
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
        let params: SearchQueryParameters = serde_structuredqs::from_str("").unwrap();
        let expected = SearchQueryParameters {
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
