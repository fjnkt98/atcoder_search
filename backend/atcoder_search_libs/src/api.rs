pub trait ToQueryParameter {
    fn to_query(&self) -> Vec<(String, String)>;
}

pub trait FieldList {
    fn field_list() -> &'static str;
}
