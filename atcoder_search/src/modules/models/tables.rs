use atcoder_search_libs::ExpandField;
use chrono::{DateTime, Local};
use sqlx::{FromRow, Type};

#[derive(Debug, FromRow, Type)]
pub struct Contest {
    pub contest_id: String,
    pub start_epoch_second: i64,
    pub duration_second: i64,
    pub title: String,
    pub rate_change: String,
    pub category: String,
}

#[derive(Debug, FromRow, Type)]
pub struct Problem {
    pub problem_id: String,
    pub contest_id: String,
    pub problem_index: String,
    pub name: String,
    pub title: String,
    pub url: String,
    pub html: String,
    pub difficulty: i32,
}

#[derive(FromRow)]
pub struct Record {
    pub problem_id: String,
    pub problem_title: String,
    pub problem_url: String,
    pub contest_id: String,
    pub contest_title: String,
    pub difficulty: Option<i32>,
    pub start_at: i64,
    pub duration: i64,
    pub rate_change: String,
    pub category: String,
    pub html: String,
}

impl Record {
    pub fn to_document(self) -> IndexingDocument {
        todo!();
    }
}

#[derive(ExpandField)]
pub struct IndexingDocument {
    pub problem_id: String,
    pub problem_title: String,
    pub problem_url: String,
    pub contest_id: String,
    pub contest_title: String,
    pub contest_url: String,
    pub difficulty: Option<i32>,
    pub start_at: DateTime<Local>,
    pub duration: i64,
    pub rate_change: String,
    pub category: String,
    pub statement_ja: Vec<String>,
    pub statement_en: Vec<String>,
}
