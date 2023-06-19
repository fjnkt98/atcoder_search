use crate::{
    cmd::TargetDomain,
    modules::{
        migration::MIGRATOR,
        problems::crawler::{ContestCrawler, ProblemCrawler},
        users::crawler::UserCrawler,
    },
};
use anyhow::{Context, Result};
use clap::Args;
use sqlx::{postgres::Postgres, Pool};
use std::env;
use tokio::time::Duration;

#[derive(Debug, Args)]
pub struct CrawlArgs {
    domain: TargetDomain,
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

    match args.domain {
        TargetDomain::Problems => {
            let crawler = ContestCrawler::new(&pool);
            crawler.run().await?;

            let crawler = ProblemCrawler::new(&pool);
            crawler.run(args.all, Duration::from_millis(1000)).await?;
            Ok(())
        }
        TargetDomain::Users => {
            let crawler = UserCrawler::new(&pool);
            crawler.crawl().await?;

            Ok(())
        }
        _ => {
            todo!();
        }
    }
}
