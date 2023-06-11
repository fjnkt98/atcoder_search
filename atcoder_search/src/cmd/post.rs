use anyhow::{Context, Result};
use atcoder_search_libs::solr::core::{SolrCore, StandaloneSolrCore};
use clap::Args;
use futures::stream::FuturesUnordered;
use std::{env, ffi::OsString, path::PathBuf, sync::Arc};
use tokio::fs::File;
use tokio_stream::StreamExt;

#[derive(Debug, Args)]
pub struct PostArgs {
    path: Option<OsString>,
}

pub async fn run(args: PostArgs) -> Result<()> {
    let save_dir: PathBuf = match args.path {
        Some(path) => PathBuf::from(path),
        None => match env::var("DOCUMENT_SAVE_DIRECTORY") {
            Ok(path) => PathBuf::from(path),
            Err(e) => anyhow::bail!(e.to_string()),
        },
    };
    let solr_host = env::var("SOLR_HOST").unwrap_or_else(|_| {
                tracing::info!("SOLR_HOST environment variable is not set. Default value `http://localhost:8983` will be used.");
                String::from("http://localhost:8983")
            });

    let core_name = env::var("CORE_NAME").with_context(|| {
        let message = "CORE_NAME must be configured";
        tracing::error!(message);
        message
    })?;

    let core = Arc::new(
        StandaloneSolrCore::new(&core_name, &solr_host).with_context(|| {
            let message = "Failed to create Solr core client";
            tracing::error!(message);
            message
        })?,
    );

    let mut files = tokio::fs::read_dir(&save_dir).await?;

    let mut tasks = FuturesUnordered::new();
    while let Ok(Some(entry)) = files.next_entry().await {
        let file = entry.path();
        if let Ok(filetype) = entry.file_type().await {
            if filetype.is_dir() {
                continue;
            }
        }
        if let Some(extension) = file.extension() {
            if extension != "json" {
                continue;
            }
        }

        let core = core.clone();

        let task = tokio::spawn(async move {
            let filename = file.display();
            let file: File = File::open(&file)
                .await
                .expect(&format!("failed to open file {}", filename));
            let size = file
                .metadata()
                .await
                .and_then(|metadata| Ok(metadata.len()))
                .unwrap_or(0);

            match core.post(file).await {
                Ok(_) => {
                    tracing::info!("Post file: {}, size: {} kB", filename, size / 1024)
                }
                Err(e) => {
                    let message = format!("failed to post document: {:?}", e);
                    tracing::error!(message);
                    panic!("{}", message);
                }
            }
        });
        tasks.push(task);
    }

    while let Some(task) = tasks.next().await {
        if let Err(e) = task {
            core.rollback().await?;
            return Err(anyhow::anyhow!(e));
        }
    }

    core.optimize().await?;

    Ok(())
}
