pub mod api;
pub mod indexing;
pub mod solr;

pub use atcoder_search_derive::{ExpandField, FieldList};
pub use indexing::ExpandField;

#[cfg(test)]
mod test {
    use crate::{api::FieldList, indexing::ExpandField};
    use atcoder_search_derive::{ExpandField, FieldList};

    #[derive(ExpandField)]
    struct MyStruct {
        id: i32,
        title: String,
        #[suffix(text_ja, text_en)]
        sentence: Vec<String>,
    }

    #[test]
    fn test_to_document() {
        let obj = MyStruct {
            id: 1,
            title: String::from("my title"),
            sentence: vec![String::from("foo"), String::from("bar")],
        };

        let data = obj.expand();

        let expected = String::from(
            r#"{"id":1,"sentence":["foo","bar"],"sentence__text_en":["foo","bar"],"sentence__text_ja":["foo","bar"],"title":"my title"}"#,
        );
        assert_eq!(expected, serde_json::to_string(&data).unwrap())
    }

    #[allow(dead_code)]
    #[derive(FieldList)]
    struct ResponseDocument {
        id: i32,
        title: String,
        sentence: Vec<String>,
    }

    #[test]
    fn test_field_list() {
        let doc = ResponseDocument {
            id: 1,
            title: String::from("my title"),
            sentence: vec![String::from("foo"), String::from("bar")],
        };
        let field_list = doc.field_list();
        let expected = "id,title,sentence";
        assert_eq!(field_list, expected);
    }
}
