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
    pub problem_id: Option<String>,
    pub category: Option<String>,
    pub difficulty: Option<i32>,
    pub is_experimental: Option<bool>,
    pub solved_count: Option<f64>,
}

impl Row {
    pub async fn correlations(&self) -> Result<(Option<String>, Option<String>)> {
        if let Some(difficulty) = &self.data.difficulty {
            let rows = sqlx::query!(
                    r#"
            WITH "difficulty_correlations" AS (
                SELECT
                    "problem_id",
                    "contest_id",
                    CAST (
                        ROUND(
                            EXP(
                                - POW(($1::integer - "difficulty"), 2.0) / 57707.8
                            ),
                            6
                        ) AS DOUBLE PRECISION
                    ) AS "correlation"

                FROM
                    "problems"
                    LEFT JOIN "difficulties" USING("problem_id")
                WHERE
                    "problems"."problem_id" <> $2::text
                    AND "difficulty" IS NOT NULL
                ORDER BY
                    "correlation" DESC
                LIMIT
                    100
            )
            SELECT
                "problem_id",
                "correlation",
                "weight"
            FROM
                "difficulty_correlations"
            LEFT JOIN "contests" USING("contest_id")
            LEFT JOIN (SELECT "to", "weight" FROM "category_relationships" WHERE "from" = $3::text) AS "relations" ON "contests"."category" = "relations"."to"
            "#,
                difficulty,
                self.data.problem_id,
                self.data.category,
            )
            .fetch_all(&self.pool)
            .await?;

            let difficulty_correlation = rows
                .iter()
                .filter(|&row| row.correlation.is_some())
                .map(|row| format!("{}|{}", row.problem_id, row.correlation.unwrap()))
                .join(" ");
            let category_correlation = rows
                .iter()
                .filter(|&row| row.correlation.is_some())
                .map(|row| format!("{}|{}", row.problem_id, row.weight.unwrap_or(1.0)))
                .join(" ");
            Ok((Some(difficulty_correlation), Some(category_correlation)))
        } else {
            Ok((None, None))
        }
    }
}

#[async_trait]
impl ToDocument for Row {
    type Document = RecommendIndex;

    async fn to_document(self) -> Result<RecommendIndex> {
        let (difficulty_correlation, category_correlation) = self.correlations().await?;

        Ok(RecommendIndex {
            problem_id: self.data.problem_id.unwrap(),
            difficulty_correlation,
            category_correlation,
            difficulty: self.data.difficulty,
            is_experimental: self.data.is_experimental.unwrap_or(false),
            solved_count: self.data.solved_count.unwrap_or(0.0),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct RecommendIndex {
    pub problem_id: String,
    pub difficulty_correlation: Option<String>,
    pub category_correlation: Option<String>,
    pub difficulty: Option<i32>,
    pub is_experimental: bool,
    pub solved_count: f64,
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
            WITH "solved_counts" AS (
                SELECT
                    "problem_id",
                    COUNT(1) AS "solved_count"
                FROM
                    "submissions"
                WHERE
                    "result" = 'AC'
                GROUP BY
                    "problem_id"
            ),
            "denominators" AS (
                SELECT
                    MAX("solved_count") AS "denominator"
                FROM
                    "solved_counts"
                WHERE
                    "solved_count" > 0
            )
            SELECT
                "problems"."problem_id" AS "problem_id",
                "contests"."category" AS "category",
                "difficulties"."difficulty" AS "difficulty",
                "difficulties"."is_experimental" AS "is_experimental",
                CAST("solved_count" AS DOUBLE PRECISION) / (SELECT "denominator" FROM "denominators") AS "solved_count"
            FROM
                "problems"
                LEFT JOIN "difficulties" ON "problems"."problem_id" = "difficulties"."problem_id"
                LEFT JOIN "contests" ON "problems"."contest_id" = "contests"."contest_id"
                LEFT JOIN "solved_counts" ON "problems"."problem_id" = "solved_counts"."problem_id"
            WHERE
                "difficulty" IS NOT NULL
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
