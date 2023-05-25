use crate::modules::{
    crawler::{ContestCrawler, ProblemCrawler},
    migration::MIGRATOR,
};
use anyhow::{Context, Result};
use clap::Args;
use sqlx::{postgres::Postgres, Pool};
use std::env;
use tokio::time::Duration;

#[derive(Debug, Args)]
pub struct CrawlArgs {
    #[arg(long)]
    all: bool,
}

pub async fn run(args: CrawlArgs) -> Result<()> {
    let database_url: String = env::var("DATABASE_URL").with_context(|| {
        let message = "DATABASE_URL must be configured.";
        tracing::error!(message);
        message
    })?;

    let pool: Pool<Postgres> = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .with_context(|| {
            let message = "Failed to create database connection pool.";
            tracing::error!(message);
            message
        })?;

    MIGRATOR.run(&pool).await?;

    let crawler = ContestCrawler::new(&pool);
    crawler.run().await?;

    let crawler = ProblemCrawler::new(&pool);
    crawler.run(args.all, Duration::from_millis(1000)).await?;
    Ok(())
}
