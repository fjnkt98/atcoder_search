use anyhow::Result;
use async_trait::async_trait;
use atcoder_search_libs::{GenerateDocument, ReadRows, ToDocument};
use itertools::Itertools;
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
        let difficulty_correlation = if let Some(difficulty) = &self.data.difficulty {
            let rows = sqlx::query!(
                r#"
            SELECT
                "problem_id",
                CAST (
                    ROUND(
                        EXP(-POW(($1::integer - "difficulty"), 2.0) / 57707.8),
                        6
                    )
                    AS DOUBLE PRECISION
                ) AS "correlation"
            FROM
                "problems"
                LEFT JOIN "difficulties" USING("problem_id")
            WHERE
                "problem_id" <> $2::text
                AND "difficulty" IS NOT NULL
            ORDER BY
                "correlation" DESC
            LIMIT
                100
            "#,
                difficulty,
                self.data.problem_id
            )
            .fetch_all(&self.pool)
            .await?;

            Some(
                rows.iter()
                    .filter(|&row| row.correlation.is_some())
                    .map(|row| format!("{}|{}", row.problem_id, row.correlation.unwrap()))
                    .join(" "),
            )
        } else {
            None
        };

        Ok(RecommendIndex {
            problem_id: self.data.problem_id,
            difficulty_correlation,
            difficulty: self.data.difficulty,
            is_experimental: self.data.is_experimental.unwrap_or(false),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct RecommendIndex {
    pub problem_id: String,
    pub difficulty_correlation: Option<String>,
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
