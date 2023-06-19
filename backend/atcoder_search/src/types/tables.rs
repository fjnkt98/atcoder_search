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

#[derive(Debug, FromRow)]
pub struct User {
    pub user_name: String,           // ユーザ名
    pub rating: i32,                 // レート
    pub highest_rating: i32,         // 最高レート
    pub affiliation: Option<String>, // 所属
    pub birth_year: Option<i32>,     // 誕生年
    pub country: Option<String>,     // 国
    pub crown: Option<String>,       // 王冠
    pub join_count: i32,             // 参加数
    pub rank: i32,                   // 順位
    pub wins: i32,                   // 優勝数
}
