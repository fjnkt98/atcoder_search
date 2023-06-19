use crate::cmd::TargetDomain;
use anyhow::{Context, Result};
use atcoder_search_libs::solr::core::{SolrCore, StandaloneSolrCore};
use atcoder_search_libs::{DocumentUploader, PostDocument};
use clap::Args;
use std::{env, ffi::OsString, path::PathBuf};

#[derive(Debug, Args)]
pub struct PostArgs {
    domain: TargetDomain,
    #[arg(long)]
    save_dir: Option<OsString>,
    #[arg(short, long)]
    optimize: bool,
}

pub async fn run(args: PostArgs) -> Result<()> {
    let save_dir: PathBuf = match args.save_dir {
        Some(save_dir) => PathBuf::from(save_dir),
        None => match env::var("DOCUMENT_SAVE_DIRECTORY") {
            Ok(path) => PathBuf::from(path).join(&args.domain.to_string()),
            Err(e) => {
                let message = format!("couldn't determine document save directory {:?}", e);
                tracing::error!(message);
                anyhow::bail!(message)
            }
        },
    };
    let solr_host = env::var("SOLR_HOST").unwrap_or_else(|_| {
                tracing::info!("SOLR_HOST environment variable is not set. Default value `http://localhost:8983` will be used.");
                String::from("http://localhost:8983")
            });

    let core_name_key = format!("{}_CORE_NAME", args.domain.to_string().to_uppercase());
    let core_name = match env::var(&core_name_key) {
        Ok(core_name) => core_name,
        Err(_) => {
            let message = format!("{} must be set", core_name_key);
            tracing::error!(message);
            anyhow::bail!(message)
        }
    };

    let core = StandaloneSolrCore::new(&core_name, &solr_host).with_context(|| {
        let message = "Failed to create Solr core client";
        tracing::error!(message);
        message
    })?;

    core.truncate().await?;
    let uploader = DocumentUploader::new();
    uploader
        .post_documents(core, &save_dir, args.optimize)
        .await?;

    Ok(())
}
