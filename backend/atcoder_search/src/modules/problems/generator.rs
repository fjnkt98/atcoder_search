use crate::modules::{problems::extractor::FullTextExtractor, utils::rate_to_color};
use anyhow::Result;
use async_trait::async_trait;
use atcoder_search_libs::{ExpandField, GenerateDocument, ReadRows, ToDocument};
use chrono::{DateTime, Local, TimeZone, Utc};
use once_cell::sync::Lazy;
use serde_json::Value;
use sqlx::{postgres::Postgres, FromRow, Pool};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

static EXTRACTOR: Lazy<FullTextExtractor> = Lazy::new(|| FullTextExtractor::new());

#[derive(FromRow, Debug)]
pub struct Row {
    pub problem_id: String,
    pub problem_title: String,
    pub problem_url: String,
    pub contest_id: String,
    pub contest_title: String,
    pub start_at: i64,
    pub duration: i64,
    pub rate_change: String,
    pub category: String,
    pub html: String,
    pub difficulty: Option<i32>,
    pub is_experimental: Option<bool>,
}

#[async_trait]
impl ToDocument for Row {
    type Document = Value;

    async fn to_document(self) -> Result<Value> {
        let (statement_ja, statement_en) = EXTRACTOR.extract(&self.html)?;
        let contest_url: String = format!("https://atcoder.jp/contests/{}", self.contest_id);

        let start_at = Local
            .timestamp_opt(self.start_at, 0)
            .earliest()
            .unwrap_or(DateTime::<Utc>::MIN_UTC.with_timezone(&Local));

        let document = ProblemIndex {
            problem_id: self.problem_id,
            problem_title: self.problem_title,
            problem_url: self.problem_url,
            contest_id: self.contest_id,
            contest_title: self.contest_title,
            contest_url,
            color: self
                .difficulty
                .and_then(|difficulty| Some(rate_to_color(difficulty))),
            difficulty: self.difficulty,
            is_experimental: self.is_experimental.unwrap_or(false),
            start_at,
            duration: self.duration,
            rate_change: self.rate_change,
            category: self.category,
            statement_ja,
            statement_en,
        };

        Ok(document.expand())
    }
}

#[derive(ExpandField)]
pub struct ProblemIndex {
    pub problem_id: String,
    #[suffix(text_ja, text_en)]
    pub problem_title: String,
    pub problem_url: String,
    pub contest_id: String,
    #[suffix(text_ja, text_en)]
    pub contest_title: String,
    pub contest_url: String,
    pub color: Option<String>,
    pub difficulty: Option<i32>,
    pub is_experimental: bool,
    pub start_at: DateTime<Local>,
    pub duration: i64,
    pub rate_change: String,
    pub category: String,
    #[suffix(text_ja, text_reading)]
    pub statement_ja: Vec<String>,
    #[suffix(text_en)]
    pub statement_en: Vec<String>,
}

pub struct ProblemDocumentGenerator {
    pool: Pool<Postgres>,
    save_dir: PathBuf,
}

impl ProblemDocumentGenerator {
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
impl ReadRows for ProblemDocumentGenerator {
    type Row = Row;

    async fn read_rows(pool: Pool<Postgres>, tx: Sender<<Self as ReadRows>::Row>) -> Result<()> {
        let mut stream = sqlx::query_as!(
            Row,
            r#"
            SELECT
                "problems"."problem_id" AS "problem_id",
                "problems"."title" AS "problem_title",
                "problems"."url" AS "problem_url",
                "contests"."contest_id" AS "contest_id",
                "contests"."title" AS "contest_title",
                "contests"."start_epoch_second" AS "start_at",
                "contests"."duration_second" AS "duration",
                "contests"."rate_change" AS "rate_change",
                "contests"."category" AS "category",
                "problems"."html" AS "html",
                "difficulties"."difficulty" AS "difficulty",
                "difficulties"."is_experimental" AS "is_experimental"
            FROM
                "problems"
                JOIN "contests" ON "problems"."contest_id" = "contests"."contest_id"
                LEFT JOIN "difficulties" ON "problems"."problem_id" = "difficulties"."problem_id"
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
impl GenerateDocument for ProblemDocumentGenerator {
    type Reader = Self;
}
