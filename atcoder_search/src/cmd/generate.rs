use crate::modules::problems::generator::ProblemDocumentGenerator;
use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use sqlx::{postgres::Postgres, Pool};
use std::{
    env,
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug, Args)]
pub struct GenerateArgs {
    domain: Domain,
    #[arg(long)]
    save_dir: Option<OsString>,
}

#[derive(Debug, Clone, ValueEnum)]
enum Domain {
    Problems,
    Users,
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Domain::Problems => write!(f, "problems"),
            Domain::Users => write!(f, "users"),
        }
    }
}

pub async fn run(args: GenerateArgs) -> Result<()> {
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

    let save_dir: PathBuf = match args.save_dir {
        Some(path) => PathBuf::from(path),
        None => match env::var("DOCUMENT_SAVE_DIRECTORY") {
            Ok(path) => {
                let save_dir = PathBuf::from(path).join(Path::new(&args.domain.to_string()));
                tracing::info!("Documents will be save at {}", save_dir.display());
                save_dir
            }
            Err(e) => {
                let message = format!("couldn't determine document save directory {:?}", e);
                tracing::error!(message);
                anyhow::bail!(message)
            }
        },
    };

    if !save_dir.exists() {
        tracing::warn!(
            "The directory {} doesn't exists, so attempt to create it",
            save_dir.display()
        );
        match tokio::fs::create_dir_all(&save_dir).await {
            Ok(_) => {
                tracing::info!(
                    "The directory {} was successfully created",
                    save_dir.display()
                );
            }
            Err(e) => {
                let message = format!(
                    "failed to create the directory {} cause {:?}",
                    save_dir.display(),
                    e
                );
                tracing::error!(message);
                anyhow::bail!(message)
            }
        };
    }

    match args.domain {
        Domain::Problems => {
            let generator = ProblemDocumentGenerator::new(&pool, &save_dir);
            generator.run().await
        }
        Domain::Users => {
            todo!();
        }
    }
}
