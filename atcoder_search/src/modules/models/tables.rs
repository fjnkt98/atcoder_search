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
