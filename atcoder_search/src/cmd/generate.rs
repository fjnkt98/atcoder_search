use crate::modules::generator::DocumentGenerator;
use anyhow::{Context, Result};
use clap::Args;
use sqlx::{postgres::Postgres, Pool};
use std::{env, ffi::OsString, path::PathBuf};

#[derive(Debug, Args)]
pub struct GenerateArgs {
    path: Option<OsString>,
}

pub async fn run(args: GenerateArgs) -> Result<()> {
    let save_dir: PathBuf = match args.path {
        Some(path) => PathBuf::from(path),
        None => match env::var("DOCUMENT_SAVE_DIRECTORY") {
            Ok(path) => PathBuf::from(path),
            Err(e) => anyhow::bail!(e.to_string()),
        },
    };

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

    let generator = DocumentGenerator::new(&pool, &save_dir);
    match generator.truncate().await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("failed to delete existing document: {:?}", e);
            return Err(anyhow::anyhow!(e));
        }
    };

    match generator.generate(1000).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("failed to generate document: {:?}", e);
            return Err(anyhow::anyhow!(e));
        }
    };

    Ok(())
}
