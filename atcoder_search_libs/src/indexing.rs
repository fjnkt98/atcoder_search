use serde_json::Value;

pub trait ExpandField {
    fn expand(&self) -> Value;
}
