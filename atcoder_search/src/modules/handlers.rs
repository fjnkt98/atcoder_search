use crate::modules::models::{
    request::{SearchQueryParameters, ValidatedSearchQueryParameters},
    response::{FacetCounts, ResponseDocument, SearchResultResponse, SearchResultStats},
};
use atcoder_search_libs::{
    solr::{
        core::{SolrCore, StandaloneSolrCore},
        model::SolrSelectResponse,
    },
    ToQueryParameter,
};
use axum::{extract::Extension, http::StatusCode, Json};
use std::sync::Arc;
use tokio::time::Instant;

type SearchResponse = (StatusCode, Json<SearchResultResponse>);

pub async fn search_with_qs(
    ValidatedSearchQueryParameters(params): ValidatedSearchQueryParameters<SearchQueryParameters>,
    Extension(core): Extension<Arc<StandaloneSolrCore>>,
) -> SearchResponse {
    let start_process = Instant::now();

    let response: SolrSelectResponse<ResponseDocument, FacetCounts> =
        match core.select(&params.to_query()).await {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("request failed cause: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(SearchResultResponse::error(&params, "unexpected error")),
                );
            }
        };

    let time: u32 = Instant::now().duration_since(start_process).as_millis() as u32;
    let total: u32 = response.response.num_found;
    let count: u32 = response.response.docs.len() as u32;
    let rows: u32 = params.limit.unwrap_or(20);
    let index: u32 = (response.response.start / rows) + 1;
    let pages: u32 = (total + rows - 1) / rows;

    tracing::info!(
        target: "querylog",
        "elapsed_time={} hits={} params={}",
        time, total, serde_json::to_string(&params).unwrap_or(String::from(""))
    );

    let stats = SearchResultStats {
        time,
        total,
        index,
        count,
        pages,
        params: serde_json::json!(params),
        facet: response.facets,
    };

    (
        StatusCode::OK,
        Json(SearchResultResponse {
            stats,
            items: response.response.docs,
            message: None,
        }),
    )
}

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
