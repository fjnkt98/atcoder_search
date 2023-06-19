use crate::modules::problems::extractor::FullTextExtractor;
use anyhow::Result;
use async_trait::async_trait;
use atcoder_search_libs::{ExpandField, GenerateDocument, ReadRows, ToDocument};
use chrono::{DateTime, Local, TimeZone, Utc};
use once_cell::sync::Lazy;
use serde_json::Value;
use sqlx::{postgres::Postgres, FromRow, Pool};
use std::path::{Path, PathBuf};
use tokio::macros::support::Pin;
use tokio_stream::Stream;

static EXTRACTOR: Lazy<FullTextExtractor> = Lazy::new(|| FullTextExtractor::new());

#[derive(FromRow, Debug)]
pub struct Row {
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

impl ToDocument for Row {
    type Document = Value;

    fn to_document(self) -> Result<Value> {
        let (statement_ja, statement_en) = EXTRACTOR.extract(&self.html)?;
        let contest_url: String = format!("https://atcoder.jp/contests/{}", self.contest_id);

        let start_at = Local
            .timestamp_opt(self.start_at, 0)
            .earliest()
            .unwrap_or(DateTime::<Utc>::MIN_UTC.with_timezone(&Local));

        let document = IndexingDocument {
            problem_id: self.problem_id,
            problem_title: self.problem_title,
            problem_url: self.problem_url,
            contest_id: self.contest_id,
            contest_title: self.contest_title,
            contest_url,
            difficulty: self.difficulty,
            start_at: start_at,
            duration: self.duration,
            rate_change: self.rate_change,
            category: self.category,
            statement_ja: statement_ja,
            statement_en: statement_en,
        };

        Ok(document.expand())
    }
}

#[derive(ExpandField)]
pub struct IndexingDocument {
    pub problem_id: String,
    #[suffix(text_ja, text_en)]
    pub problem_title: String,
    pub problem_url: String,
    pub contest_id: String,
    #[suffix(text_ja, text_en)]
    pub contest_title: String,
    pub contest_url: String,
    pub difficulty: Option<i32>,
    pub start_at: DateTime<Local>,
    pub duration: i64,
    pub rate_change: String,
    pub category: String,
    #[suffix(text_ja, text_reading)]
    pub statement_ja: Vec<String>,
    #[suffix(text_en)]
    pub statement_en: Vec<String>,
}

pub struct ProblemDocumentGenerator<'a> {
    pool: &'a Pool<Postgres>,
    save_dir: PathBuf,
}

impl<'a> ProblemDocumentGenerator<'a> {
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

        match self.generate(&self.save_dir, 1000).await {
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
impl<'a> ReadRows<'a> for ProblemDocumentGenerator<'a> {
    type Row = Row;

    async fn read_rows(
        &'a self,
    ) -> Result<Pin<Box<dyn Stream<Item = std::result::Result<Self::Row, sqlx::Error>> + Send + 'a>>>
    {
        let stream = sqlx::query_as(
            "
            SELECT
                problems.problem_id AS problem_id,
                problems.title AS problem_title,
                problems.url AS problem_url,
                contests.contest_id AS contest_id,
                contests.title AS contest_title,
                problems.difficulty AS difficulty,
                contests.start_epoch_second AS start_at,
                contests.duration_second AS duration,
                contests.rate_change AS rate_change,
                contests.category AS category,
                problems.html AS html
            FROM
                problems
                JOIN contests ON problems.contest_id = contests.contest_id;
            ",
        )
        .fetch(self.pool);

        Ok(stream)
    }
}

#[async_trait]
impl<'a> GenerateDocument<'a> for ProblemDocumentGenerator<'a> {}
