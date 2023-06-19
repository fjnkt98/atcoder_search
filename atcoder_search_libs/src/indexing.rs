use crate::solr::core::SolrCore;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use serde::Serialize;
use serde_json::Value;
use std::{
    ffi::OsString,
    fmt::Debug,
    fs::File,
    io::BufWriter,
    mem,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
use tokio_stream::{Stream, StreamExt};

pub trait ExpandField {
    fn expand(&self) -> Value;
}

#[async_trait]
pub trait ReadRows<'a> {
    type Row: Debug + ToDocument + Send + Sync + 'static;
    async fn read_rows(
        &'a self,
    ) -> Result<Pin<Box<dyn Stream<Item = std::result::Result<Self::Row, sqlx::Error>> + Send + 'a>>>;
}

pub trait ToDocument {
    type Document: Debug + Serialize + Send + Sync + 'static;

    fn to_document(self) -> Result<Self::Document>;
}

#[async_trait]
pub trait PostDocument {
    async fn post_documents<C>(&self, core: C, save_dir: &Path, optimize: bool) -> Result<()>
    where
        C: SolrCore + Sync + Send + 'static,
    {
        let core = Arc::new(core);
        let mut files = tokio::fs::read_dir(save_dir).await?;

        let mut tasks: FuturesUnordered<JoinHandle<()>> = FuturesUnordered::new();
        while let Ok(Some(entry)) = files.next_entry().await {
            if entry
                .file_type()
                .await
                .and_then(|file_type| Ok(file_type.is_dir()))
                .unwrap_or(false)
            {
                continue;
            }
            let file = entry.path();
            if file.extension() != Some(OsString::from("json").as_ref()) {
                continue;
            }

            let core = core.clone();
            let task = tokio::spawn(async move {
                let filename = file.display();
                let file = match tokio::fs::File::open(&file).await {
                    Ok(file) => file,
                    Err(e) => {
                        let message = format!("failed to open the file {} cause {:?}", filename, e);
                        tracing::error!(message);
                        panic!("{}", message);
                    }
                };

                let size = file
                    .metadata()
                    .await
                    .and_then(|metadata| Ok(metadata.len()))
                    .unwrap_or(0);

                match core.post(file).await {
                    Ok(_) => {
                        tracing::info!("Post the file: {}, size: {} kB", filename, size / 1024)
                    }
                    Err(e) => {
                        let message = format!("failed to post document: {:?}", e);
                        tracing::error!(message);
                        panic!("{}", message)
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

        if optimize {
            core.optimize().await?;
        } else {
            core.commit().await?;
        }

        Ok(())
    }
}

#[async_trait]
pub trait GenerateDocument<'a>: ReadRows<'a> {
    async fn generate(&'a self, save_dir: &Path, chunk_size: usize) -> Result<()> {
        let (tx, mut rx): (
            Sender<<<Self as ReadRows>::Row as ToDocument>::Document>,
            Receiver<<<Self as ReadRows>::Row as ToDocument>::Document>,
        ) = tokio::sync::mpsc::channel(2 * chunk_size);

        let save_dir: PathBuf = save_dir.to_owned();
        let saver = tokio::task::spawn_blocking(move || {
            let mut suffix: u32 = 0;
            let mut documents: Vec<<<Self as ReadRows>::Row as ToDocument>::Document> =
                Vec::with_capacity(chunk_size);

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

        let mut stream = self.read_rows().await?;
        let mut tasks: FuturesUnordered<JoinHandle<()>> = FuturesUnordered::new();
        while let Some(row) = StreamExt::try_next(&mut stream).await? {
            let tx = tx.clone();
            let task = tokio::task::spawn(async move {
                let document = match row.to_document() {
                    Ok(document) => document,
                    Err(e) => {
                        let message = format!(
                            "failed to convert from row into document cause: {}",
                            e.to_string()
                        );
                        tracing::error!(message);
                        panic!("{}", message);
                    }
                };

                tx.send(document)
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
                    tracing::error!("an error occurred when generating document: {:?}", e);
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
                tracing::error!("an error occurred when saving the documents: {:?}", e);
                Err(anyhow::anyhow!(e))
            }
        }
    }
}
