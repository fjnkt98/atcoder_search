use anyhow::Result;
use async_trait::async_trait;
use atcoder_search_libs::{GenerateDocument, ReadRows, ToDocument};
use serde::Serialize;
use sqlx::{postgres::Postgres, FromRow, Pool};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

#[derive(Debug)]
pub struct Row {
    pool: Pool<Postgres>,
    data: Data,
}

#[derive(FromRow, Debug)]
pub struct Data {
    pub problem_id: String,
    pub difficulty: Option<i32>,
    pub is_experimental: Option<bool>,
}

#[async_trait]
impl ToDocument for Row {
    type Document = RecommendIndex;

    async fn to_document(self) -> Result<RecommendIndex> {
        let related_problem = String::from("");

        Ok(RecommendIndex {
            problem_id: self.data.problem_id,
            related_problem,
            difficulty: self.data.difficulty,
            is_experimental: self.data.is_experimental.unwrap_or(false),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct RecommendIndex {
    pub problem_id: String,
    pub related_problem: String,
    pub difficulty: Option<i32>,
    pub is_experimental: bool,
}

pub struct RecommendDocumentGenerator {
    pool: Pool<Postgres>,
    save_dir: PathBuf,
}

impl RecommendDocumentGenerator {
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

        match self.generate(self.pool.clone(), &self.save_dir, 1000).await {
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
impl ReadRows for RecommendDocumentGenerator {
    type Row = Row;

    async fn read_rows(pool: Pool<Postgres>, tx: Sender<<Self as ReadRows>::Row>) -> Result<()> {
        let mut stream = sqlx::query_as!(
            Data,
            r#"
            SELECT
                "problems"."problem_id" AS "problem_id",
                "difficulties"."difficulty" AS "difficulty",
                "difficulties"."is_experimental" AS "is_experimental"
            FROM
                "problems"
                LEFT JOIN "difficulties" ON "problems"."problem_id" = "difficulties"."problem_id"
            "#,
        )
        .fetch(&pool);

        while let Some(data) = stream.try_next().await? {
            let row = Row {
                pool: pool.clone(),
                data,
            };

            tx.send(row).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl GenerateDocument for RecommendDocumentGenerator {}
