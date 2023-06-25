use crate::modules::handlers::{liveness, problem::search_problem, readiness, user::search_user};
use anyhow::Result;
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
    let problem_core_name = env::var("PROBLEMS_CORE_NAME").unwrap_or_else(|_| {
        tracing::warn!("PROBLEMS_CORE_NAME not set. name 'problems' will be used.");
        String::from("problems")
    });
    let user_core_name = env::var("USERS_CORE_NAME").unwrap_or_else(|_| {
        tracing::warn!("USERS_CORE_NAME not set. name 'problems' will be used.");
        String::from("users")
    });

    let problem_core = match StandaloneSolrCore::new(&problem_core_name, &solr_host) {
        Ok(core) => core,
        Err(_) => {
            let message = "couldn't create problems core instance. check your Solr instance status and value of SOLR_HOST environment variable.";
            tracing::error!(message);
            anyhow::bail!(message)
        }
    };
    let user_core = match StandaloneSolrCore::new(&user_core_name, &solr_host) {
        Ok(core) => core,
        Err(_) => {
            let message = "couldn't create users core instance. check your Solr instance status and value of SOLR_HOST environment variable.";
            tracing::error!(message);
            anyhow::bail!(message)
        }
    };

    let app = create_router(problem_core, user_core);
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

fn create_router(
    problem_core: impl SolrCore + Clone + Sync + Send + 'static,
    user_core: impl SolrCore + Clone + Sync + Send + 'static,
) -> Router {
    // let origin = env::var("FRONTEND_ORIGIN_URL").unwrap_or(String::from("http://localhost:8000"));
    // let service = routing::get_service(ServeDir::new("assets"))
    //     .handle_error(|e| async move { (StatusCode::NOT_FOUND, format!("file not found: {}", e)) });
    let problem_core = Arc::new(problem_core);
    let user_core = Arc::new(user_core);

    let problem_routes = Router::new()
        .route("/problem", routing::get(search_problem))
        .layer(Extension(problem_core.clone()));
    let user_routes = Router::new()
        .route("/user", routing::get(search_user))
        .layer(Extension(user_core.clone()));
    let search_routes = Router::new()
        .nest("/search", problem_routes)
        .nest("/search", user_routes);
    let liveness_routes = Router::new()
        .route("/liveness", routing::get(liveness))
        .layer(Extension(problem_core.clone()));
    let readiness_routes = Router::new()
        .route("/readiness", routing::get(readiness))
        .layer(Extension(problem_core.clone()));

    Router::new()
        .nest("/api", search_routes)
        // .nest_service("/", service)
        .nest("/api", liveness_routes)
        .nest("/api", readiness_routes)
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
