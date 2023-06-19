use anyhow::{Context, Result};
use atcoder_search_libs::solr::core::StandaloneSolrCore;
use atcoder_search_libs::{DocumentUploader, PostDocument};
use clap::Args;
use std::{env, ffi::OsString, path::PathBuf};

#[derive(Debug, Args)]
pub struct PostArgs {
    path: Option<OsString>,
    #[arg(short, long)]
    optimize: bool,
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

    let core = StandaloneSolrCore::new(&core_name, &solr_host).with_context(|| {
        let message = "Failed to create Solr core client";
        tracing::error!(message);
        message
    })?;

    let uploader = DocumentUploader::new();
    uploader
        .post_documents(core, &save_dir, args.optimize)
        .await?;

    Ok(())
}
