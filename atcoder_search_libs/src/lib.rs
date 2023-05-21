pub mod api;
pub mod indexing;
pub mod solr;

pub use atcoder_search_derive::{ExpandField, FieldList};
pub use indexing::ExpandField;

#[cfg(test)]
mod test {
    use crate::{api::FieldList, indexing::ExpandField};
    use atcoder_search_derive::{ExpandField, FieldList};
    use chrono::{DateTime, Local, TimeZone};

    #[derive(ExpandField)]
    struct MyStruct {
        id: i32,
        title: String,
        #[suffix(text_ja, text_en)]
        sentence: Vec<String>,
        published_at: DateTime<Local>,
    }

    #[test]
    fn test_to_document() {
        let obj = MyStruct {
            id: 1,
            title: String::from("my title"),
            sentence: vec![String::from("foo"), String::from("bar")],
            published_at: Local
                .datetime_from_str("2023/05/21 12:31:28", "%Y/%m/%d %H:%M:%S")
                .unwrap(),
        };

        let data = obj.expand();

        let expected = String::from(
            r#"{"id":1,"published_at":"2023-05-21T03:31:28Z","sentence":["foo","bar"],"sentence__text_en":["foo","bar"],"sentence__text_ja":["foo","bar"],"title":"my title"}"#,
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
