pub mod problem;
pub mod user;

use atcoder_search_libs::solr::core::{SolrCore, StandaloneSolrCore};
use axum::{extract::Extension, http::StatusCode};
use std::sync::Arc;

pub async fn liveness(
    Extension(problem_core): Extension<Arc<StandaloneSolrCore>>,
    Extension(user_core): Extension<Arc<StandaloneSolrCore>>,
    // Extension(recommend_core): Extension<Arc<StandaloneSolrCore>>
) -> StatusCode {
    if let (Ok(_), Ok(_)) = (problem_core.ping().await, user_core.ping().await) {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub async fn readiness(
    Extension(problem_core): Extension<Arc<StandaloneSolrCore>>,
    Extension(user_core): Extension<Arc<StandaloneSolrCore>>,
    // Extension(recommend_core): Extension<Arc<StandaloneSolrCore>>
) -> StatusCode {
    let problem_is_ok = problem_core
        .status()
        .await
        .and_then(|status| Ok(status.index.num_docs != 0))
        .unwrap_or(false);
    let user_is_ok = user_core
        .status()
        .await
        .and_then(|status| Ok(status.index.num_docs != 0))
        .unwrap_or(false);
    // let recommend_is_ok = recommend_core.status().await.and_then(|status| Ok(status.index.num_docs == 0) ).unwrap_or(false);

    if [problem_is_ok, user_is_ok].iter().all(|i| *i) {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
