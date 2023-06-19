use crate::types::tables::User;
use anyhow::Result;
use async_trait::async_trait;
use atcoder_search_libs::{GenerateDocument, ReadRows, ToDocument};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::Postgres, Pool};
use std::path::{Path, PathBuf};
use tokio::macros::support::Pin;
use tokio_stream::Stream;

fn rate_to_color(rate: i32) -> String {
    match rate {
        0..=399 => "gray",
        400..=799 => "brown",
        800..=1199 => "green",
        1200..=1599 => "cyan",
        1600..=1999 => "blue",
        2000..=2399 => "yellow",
        2400..=2799 => "orange",
        2800..=3199 => "red",
        3200..=3599 => "silver",
        _ => "gold",
    }
    .to_string()
}

impl ToDocument for User {
    type Document = UserIndex;

    fn to_document(self) -> Result<UserIndex> {
        Ok(self.into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserIndex {
    pub user_name: String,
    pub rating: i32,
    pub color: String,
    pub highest_rating: i32,
    pub highest_color: String,
    pub affiliation: Option<String>,
    pub birth_year: Option<i32>,
    pub country: Option<String>,
    pub crown: Option<String>,
    pub join_count: i32,
    pub rank: i32,
    pub wins: i32,
}

impl From<User> for UserIndex {
    fn from(value: User) -> Self {
        let color = rate_to_color(value.rating);
        let highest_color = rate_to_color(value.highest_rating);

        Self {
            user_name: value.user_name,
            rating: value.rating,
            color,
            highest_rating: value.highest_rating,
            highest_color,
            affiliation: value.affiliation,
            birth_year: value.birth_year,
            country: value.country,
            crown: value.crown,
            join_count: value.join_count,
            rank: value.rank,
            wins: value.wins,
        }
    }
}

pub struct UserDocumentGenerator<'a> {
    pool: &'a Pool<Postgres>,
    save_dir: PathBuf,
}

impl<'a> UserDocumentGenerator<'a> {
    pub fn new(pool: &'a Pool<Postgres>, save_dir: &Path) -> Self {
        Self {
            pool,
            save_dir: save_dir.to_owned(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        match self.clean(&self.save_dir).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("failed to delete existing document: {:?}", e);
                return Err(anyhow::anyhow!(e));
            }
        };

        match self.generate(&self.save_dir, 10000).await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("failed to generate document: {:?}", e);
                return Err(anyhow::anyhow!(e));
            }
        };

        Ok(())
    }
}

#[async_trait]
impl<'a> ReadRows<'a> for UserDocumentGenerator<'a> {
    type Row = User;

    async fn read_rows(
        &'a self,
    ) -> Result<Pin<Box<dyn Stream<Item = std::result::Result<Self::Row, sqlx::Error>> + Send + 'a>>>
    {
        let stream = sqlx::query_as(
            r#"
            SELECT
                "user_name",
                "rating",
                "highest_rating",
                "affiliation",
                "birth_year",
                "country",
                "crown",
                "join_count",
                "rank",
                "wins"
            FROM
                "users"
            "#,
        )
        .fetch(self.pool);

        Ok(stream)
    }
}

#[async_trait]
impl<'a> GenerateDocument<'a> for UserDocumentGenerator<'a> {}
