pub mod problem;
pub mod user;

use atcoder_search_libs::solr::core::{SolrCore, StandaloneSolrCore};
use axum::{extract::Extension, http::StatusCode};
use std::sync::Arc;

pub async fn liveness(Extension(core): Extension<Arc<StandaloneSolrCore>>) -> StatusCode {
    match core.ping().await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn readiness(Extension(core): Extension<Arc<StandaloneSolrCore>>) -> StatusCode {
    let status = match core.status().await {
        Ok(status) => status,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    if status.index.num_docs == 0 {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        StatusCode::OK
    }
}
