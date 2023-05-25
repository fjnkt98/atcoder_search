use crate::modules::extractor::FullTextExtractor;
use anyhow::Result;
use atcoder_search_libs::ExpandField;
use chrono::{Local, SecondsFormat, TimeZone};
use futures::stream::FuturesUnordered;
use once_cell::sync::Lazy;
use serde_json::Value;
use sqlx::postgres::Postgres;
use sqlx::FromRow;
use sqlx::Pool;
use std::{
    fs::File,
    io::BufWriter,
    mem,
    path::{Path, PathBuf},
};
use tokio::macros::support::Pin;
use tokio_stream::{Stream, StreamExt};

static EXTRACTOR: Lazy<FullTextExtractor> = Lazy::new(|| FullTextExtractor::new());

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
    pub fn to_document(self) -> Result<IndexingDocument> {
        let (statement_ja, statement_en) = EXTRACTOR.extract(&self.html)?;
        let contest_url: String = format!("https://atcoder.jp/contests/{}", self.contest_id);

        let start_at = Local
            .timestamp_opt(self.start_at, 0)
            .earliest()
            .and_then(|start_at| Some(start_at.to_rfc3339_opts(SecondsFormat::Secs, true)))
            .unwrap_or(String::from("1970-01-01T00:00:00Z"));

        let document = IndexingDocument {
            problem_id: self.problem_id,
            problem_title: self.problem_title,
            problem_url: self.problem_url,
            contest_id: self.contest_id,
            contest_title: self.contest_title,
            contest_url: contest_url,
            difficulty: self.difficulty,
            start_at: start_at,
            duration: self.duration,
            rate_change: self.rate_change,
            category: self.category,
            statement_ja: statement_ja,
            statement_en: statement_en,
        };

        Ok(document)
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
    pub start_at: String,
    pub duration: i64,
    pub rate_change: String,
    pub category: String,
    pub statement_ja: Vec<String>,
    pub statement_en: Vec<String>,
}

pub struct RecordReader<'a> {
    pool: &'a Pool<Postgres>,
}

impl<'a> RecordReader<'a> {
    pub fn new(pool: &'a Pool<Postgres>) -> Self {
        RecordReader { pool: pool }
    }

    pub async fn read_rows(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = std::result::Result<Record, sqlx::Error>> + Send + 'a>>>
    {
        let stream = sqlx::query_as(
            "
            SELECT
                problems.problem_id id AS problem_id,
                problems.title AS problem_title,
                problems.url AS problem_url,
                contests.problem_id id AS contest_id,
                contests.title AS contest_title,
                problems.difficulty AS difficulty,
                contests.start_epoch_second AS start_at,
                contests.duration_second AS duration,
                contests.rate_change AS rate_change,
                contests.category AS category,
                problems.html AS html
            FROM
                problems
                JOIN contests ON problems.contest_id = contests.problem_id;
            ",
        )
        .fetch(self.pool);

        Ok(stream)
    }
}

pub struct DocumentGenerator<'a> {
    reader: RecordReader<'a>,
    save_dir: PathBuf,
}

impl<'a> DocumentGenerator<'a> {
    pub fn new(pool: &'a Pool<Postgres>, save_dir: &Path) -> Self {
        Self {
            reader: RecordReader::new(pool),
            save_dir: save_dir.to_path_buf(),
        }
    }

    pub async fn truncate(&self) -> Result<()> {
        let mut files = tokio::fs::read_dir(&self.save_dir).await?;

        tracing::info!(
            "start to delete existing file in {}",
            self.save_dir.display()
        );
        while let Ok(Some(entry)) = files.next_entry().await {
            let file = entry.path();
            if let Some(extension) = file.extension() {
                if extension == "json" {
                    tracing::info!("delete existing file {}", file.display());
                    tokio::fs::remove_file(file).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn generate(&self, chunk_size: usize) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(2 * chunk_size);

        let save_dir = self.save_dir.clone();
        let saver = tokio::task::spawn_blocking(move || {
            let mut suffix: u32 = 0;
            let mut documents: Vec<Value> = Vec::with_capacity(chunk_size);

            while let Some(document) = rx.blocking_recv() {
                suffix += 1;
                documents.push(document);

                if documents.len() >= chunk_size {
                    let filepath = save_dir.join(format!("doc-{}.json", suffix));

                    tracing::info!("Generate document file: {}", filepath.display());
                    let file = match File::create(filepath) {
                        Ok(file) => file,
                        Err(e) => {
                            let message = format!("failed to create file: {:?}", e);
                            tracing::error!(message);
                            panic!("{}", message);
                        }
                    };
                    let writer = BufWriter::new(file);
                    if let Err(e) = serde_json::to_writer_pretty(writer, &documents) {
                        let message = format!("failed to write document content: {:?}", e);
                        tracing::error!(message);
                        panic!("{}", message);
                    }

                    documents.clear();
                }
            }

            if !documents.is_empty() {
                let filepath = save_dir.join(format!("doc-{}.json", suffix));

                tracing::info!("Generate document file: {}", filepath.display());
                let file = match File::create(filepath) {
                    Ok(file) => file,
                    Err(e) => {
                        let message = format!("failed to create file: {:?}", e);
                        tracing::error!(message);
                        panic!("{}", message);
                    }
                };
                let writer = BufWriter::new(file);
                if let Err(e) = serde_json::to_writer_pretty(writer, &documents) {
                    let message = format!("failed to write document content: {:?}", e);
                    tracing::error!(message);
                    panic!("{}", message);
                }

                documents.clear();
            }
        });

        let mut record_stream = self.reader.read_rows().await?;
        let mut tasks = FuturesUnordered::new();
        while let Some(record) = tokio_stream::StreamExt::try_next(&mut record_stream).await? {
            let tx = tx.clone();
            let task = tokio::task::spawn(async move {
                let document = record.to_document().unwrap_or_else(|e| {
                    let message = format!(
                        "failed to convert from record into document cause: {}",
                        e.to_string()
                    );
                    tracing::error!(message);
                    panic!("{}", message);
                });
                let expanded = document.expand();

                tx.send(expanded)
                    .await
                    .expect("failed to send document to channel");
            });
            tasks.push(task);
        }
        mem::drop(tx);

        while let Some(task) = tasks.next().await {
            match task {
                Ok(()) => {}
                Err(e) => {
                    tracing::error!("An error occurred when generating document: {:?}", e);
                    saver.abort();
                    return Err(anyhow::anyhow!(e));
                }
            }
        }

        match saver.await {
            Ok(_) => {
                tracing::info!("All documents successfully saved.");
                Ok(())
            }
            Err(e) => {
                tracing::error!("An error occurred when saving the documents: {:?}", e);
                Err(anyhow::anyhow!(e))
            }
        }
    }
}
