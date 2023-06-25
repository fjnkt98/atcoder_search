use serde::{de::IntoDeserializer, Deserialize, Deserializer, Serialize};
use validator::Validate;

pub trait ToQuery {
    fn to_query(&self) -> Vec<(String, String)>;
}

pub trait FieldList {
    fn field_list() -> &'static str;
}

#[derive(Debug, Serialize, Deserialize, Validate, PartialEq, Eq, Clone)]
pub struct RangeFilterParameter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<i32>,
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

pub fn deserialize_optional_comma_separated<'de, D, T>(
    deserializer: D,
) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let value = String::deserialize(deserializer)?;
    let values: Result<Vec<T>, D::Error> = value
        .split(',')
        .into_iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| T::deserialize(v.into_deserializer()))
        .collect();

    values.and_then(|values| {
        if values.is_empty() {
            Ok(None)
        } else {
            Ok(Some(values))
        }
    })
}

pub fn deserialize_comma_separated<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let value = String::deserialize(deserializer)?;
    let values: Result<Vec<T>, D::Error> = value
        .split(',')
        .into_iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| T::deserialize(v.into_deserializer()))
        .collect();

    values.and_then(|values| Ok(values))
}

#[derive(Debug, Serialize)]
pub struct SearchResultResponse<P, D, F>
where
    P: Serialize,
    D: Serialize,
    F: Serialize,
{
    pub stats: SearchResultStats<P, F>,
    pub items: Vec<D>,
    pub message: Option<String>,
}

impl<P, D, F> SearchResultResponse<P, D, F>
where
    P: Serialize,
    D: Serialize,
    F: Serialize,
{
    pub fn error(params: P, message: impl ToString) -> Self {
        Self {
            stats: SearchResultStats {
                time: 0,
                total: 0,
                index: 0,
                pages: 0,
                count: 0,
                params: params,
                facet: None,
            },
            items: Vec::new(),
            message: Some(message.to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResultStats<P, F> {
    pub time: u32,
    pub total: u32,
    pub index: u32,
    pub pages: u32,
    pub count: u32,
    pub params: P,
    pub facet: Option<F>,
}
