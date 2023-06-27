use crate::{modules::utils::rate_to_color, types::tables::User};
use anyhow::Result;
use async_trait::async_trait;
use atcoder_search_libs::{GenerateDocument, ReadRows, ToDocument};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::Postgres, Pool};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

fn join_count_grade(join_count: i32) -> String {
    if join_count < 10 {
        String::from("    ~  10")
    } else if join_count < 100 {
        let c = join_count / 10;
        format!("{c}0  ~  {c}9", c = c)
    } else {
        let c = join_count / 100;
        format!("{c}00 ~ {c}99", c = c)
    }
}

#[async_trait]
impl ToDocument for User {
    type Document = UserIndex;

    async fn to_document(self) -> Result<UserIndex> {
        Ok(self.into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserIndex {
    pub user_name: String,
    pub rating: i32,
    pub highest_rating: i32,
    pub affiliation: Option<String>,
    pub birth_year: Option<i32>,
    pub country: Option<String>,
    pub crown: Option<String>,
    pub join_count: i32,
    pub rank: i32,
    pub wins: i32,
    pub color: String,
    pub highest_color: String,
    pub period: Option<String>,
    pub join_count_grade: String,
}

impl From<User> for UserIndex {
    fn from(value: User) -> Self {
        let color = rate_to_color(value.rating);
        let highest_color = rate_to_color(value.highest_rating);
        let period = value
            .birth_year
            .and_then(|year| Some(format!("{}0's", year / 10)));
        let join_count_grade = join_count_grade(value.join_count);

        Self {
            user_name: value.user_name,
            rating: value.rating,
            highest_rating: value.highest_rating,
            affiliation: value.affiliation,
            birth_year: value.birth_year,
            country: value.country,
            crown: value.crown,
            join_count: value.join_count,
            rank: value.rank,
            wins: value.wins,
            color,
            highest_color,
            period,
            join_count_grade,
        }
    }
}

pub struct UserDocumentGenerator {
    pool: Pool<Postgres>,
    save_dir: PathBuf,
}

impl UserDocumentGenerator {
    pub fn new(pool: Pool<Postgres>, save_dir: &Path) -> Self {
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

        match self
            .generate(self.pool.clone(), &self.save_dir, 10000)
            .await
        {
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
impl ReadRows for UserDocumentGenerator {
    type Row = User;

    async fn read_rows(pool: Pool<Postgres>, tx: Sender<<Self as ReadRows>::Row>) -> Result<()> {
        let mut stream = sqlx::query_as!(
            User,
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
        .fetch(&pool);

        while let Some(row) = stream.try_next().await? {
            tx.send(row).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl GenerateDocument for UserDocumentGenerator {}
