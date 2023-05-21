pub trait SearchParameter {
    fn to_query(&self) -> Vec<(String, String)>;
}

pub trait FieldList {
    fn field_list(&self) -> &'static str;
}
