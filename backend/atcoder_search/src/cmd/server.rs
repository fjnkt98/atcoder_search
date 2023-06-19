use crate::modules::handlers::{liveness, readiness, search_with_qs};
use anyhow::{Context, Result};
use atcoder_search_libs::solr::core::{SolrCore, StandaloneSolrCore};
use axum::{extract::Extension, routing, Router, Server};
use clap::Args;
use std::{env, net::SocketAddr, sync::Arc};

#[derive(Debug, Args)]
pub struct ServerArgs {
    #[arg(long)]
    port: Option<u16>,
}

pub async fn run(args: ServerArgs) -> Result<()> {
    let solr_host = env::var("SOLR_HOST").unwrap_or_else(|_| {
        tracing::warn!("SOLR_HOST environment variable is not set. Default value `http://localhost:8983` will be used.");
        String::from("http://localhost:8983")
    });
    let core_name = env::var("CORE_NAME").with_context(|| {
        let message = "SOLR_HOST environment variable must be set";
        tracing::error!(message);
        format!("{}", message)
    })?;

    tracing::info!("Connect to Solr core {}", core_name);
    let core = StandaloneSolrCore::new(&core_name, &solr_host).with_context(|| {
        let message = "couldn't create Solr core instance. check your Solr instance status and value of SOLR_HOST environment variable.";
        tracing::error!(message);
        format!("{}", message)
    })?;

    core.ping().await.with_context(|| {
        let message = format!("core {} is not available", core_name);
        tracing::error!(message);
        message
    })?;
    let app = create_router(core);
    let port = match args.port {
        Some(port) => port,
        None => {
            tracing::warn!("API server will be launched at default port number 8000");
            8000u16
        }
    };
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Server start at port {}", port);
    Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Failed to bind server.");

    Ok(())
}

fn create_router(core: impl SolrCore + Sync + Send + 'static) -> Router {
    // let origin = env::var("FRONTEND_ORIGIN_URL").unwrap_or(String::from("http://localhost:8000"));
    // let service = routing::get_service(ServeDir::new("assets"))
    //     .handle_error(|e| async move { (StatusCode::NOT_FOUND, format!("file not found: {}", e)) });

    Router::new()
        .route("/api/search", routing::get(search_with_qs))
        // .nest_service("/", service)
        .route("/api/liveness", routing::get(liveness))
        .route("/api/readiness", routing::get(readiness))
        .layer(Extension(Arc::new(core)))
    // .layer(
    //     CorsLayer::new()
    //         .allow_origin(AllowOrigin::exact(origin.parse().unwrap()))
    //         .allow_methods(Any)
    //         .allow_headers(vec![CONTENT_TYPE]),
    // )
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler.");
    };

    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("SIGINT signal received, starting graceful shutdown.");
}
