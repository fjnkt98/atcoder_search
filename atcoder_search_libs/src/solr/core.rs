use crate::solr::model::*;
use async_trait::async_trait;
use hyper::{
    self, client::HttpConnector, http::uri::InvalidUri, Body, Client, Method, Request, Uri,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use thiserror::Error;
use url::{self, Url};

type Result<T> = std::result::Result<T, SolrCoreError>;

#[derive(Debug, Error)]
pub enum SolrCoreError {
    #[error("failed to request to solr core")]
    RequestError(#[from] hyper::Error),
    #[error("failed to build request")]
    RequestBuildError(#[from] hyper::http::Error),
    #[error("failed to deserialize JSON data")]
    DeserializeError(#[from] serde_json::Error),
    #[error("invalid Solr url given")]
    InvalidUrlError(#[from] url::ParseError),
    #[error("invalid uri error")]
    UriParseError(#[from] InvalidUri),
    #[error("core not found")]
    CoreNotFoundError(String),
}

#[async_trait]
pub trait SolrCore {
    async fn ping(&self) -> Result<SolrPingResponse>;
    async fn status(&self) -> Result<SolrCoreStatus>;
    async fn reload(&self) -> Result<SolrSimpleResponse>;
    async fn select<D>(
        &self,
        params: &[(impl ToString + Sync, impl ToString + Sync)],
    ) -> Result<SolrSelectResponse<D>>
    where
        D: Serialize + DeserializeOwned;
    async fn post(&self, body: Vec<u8>) -> Result<SolrSimpleResponse>;
    async fn commit(&self) -> Result<()>;
    async fn optimize(&self) -> Result<()>;
    async fn rollback(&self) -> Result<()>;
    async fn truncate(&self) -> Result<()>;
}

pub struct StandaloneSolrCore {
    name: String,
    admin_url: String,
    ping_url: String,
    post_url: String,
    select_url: String,
    client: Client<HttpConnector>,
}

impl StandaloneSolrCore {
    pub fn new(name: &str, solr_url: &str) -> Result<Self> {
        let mut solr_url = Url::parse(solr_url).map_err(|e| SolrCoreError::InvalidUrlError(e))?;
        solr_url.set_path("");
        let base_url = String::from(solr_url.as_str());
        let admin_url = format!("{}solr/admin/cores", base_url);
        let ping_url = format!("{}solr/{}/admin/ping", base_url, name);
        let post_url = format!("{}solr/{}/update", base_url, name);
        let select_url = format!("{}solr/{}/select", base_url, name);

        let client = Client::new();
        Ok(StandaloneSolrCore {
            name: String::from(name),
            admin_url: admin_url,
            ping_url: ping_url,
            post_url: post_url,
            select_url: select_url,
            client: client,
        })
    }
}

#[async_trait]
impl SolrCore for StandaloneSolrCore {
    async fn ping(&self) -> Result<SolrPingResponse> {
        let uri = self
            .ping_url
            .parse::<Uri>()
            .map_err(|e| SolrCoreError::UriParseError(e))?;
        let res = self
            .client
            .get(uri)
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;
        let body = hyper::body::to_bytes(res.into_body())
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;

        let result: SolrPingResponse =
            serde_json::from_slice(&body).map_err(|e| SolrCoreError::DeserializeError(e))?;

        Ok(result)
    }

    async fn status(&self) -> Result<SolrCoreStatus> {
        let url = Url::parse_with_params(
            &self.admin_url,
            &[("action", "STATUS"), ("core", &self.name)],
        )
        .map_err(|e| SolrCoreError::InvalidUrlError(e))?;
        let uri = url
            .as_str()
            .parse::<Uri>()
            .map_err(|e| SolrCoreError::UriParseError(e))?;
        let res = self
            .client
            .get(uri)
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;
        let body = hyper::body::to_bytes(res.into_body())
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;
        let core_list: SolrCoreList =
            serde_json::from_slice(&body).map_err(|e| SolrCoreError::DeserializeError(e))?;

        let result = core_list
            .status
            .and_then(|status| status.get(&self.name).cloned())
            .ok_or(SolrCoreError::CoreNotFoundError(String::from(
                "core not found",
            )))?;
        Ok(result)
    }

    async fn reload(&self) -> Result<SolrSimpleResponse> {
        let url = Url::parse_with_params(
            &self.admin_url,
            &[("action", "RELOAD"), ("core", &self.name)],
        )
        .map_err(|e| SolrCoreError::InvalidUrlError(e))?;
        let uri = url
            .as_str()
            .parse::<Uri>()
            .map_err(|e| SolrCoreError::UriParseError(e))?;
        let res = self
            .client
            .get(uri)
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;
        let body = hyper::body::to_bytes(res.into_body())
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;
        let result: SolrSimpleResponse =
            serde_json::from_slice(&body).map_err(|e| SolrCoreError::DeserializeError(e))?;

        Ok(result)
    }

    async fn select<D>(
        &self,
        params: &[(impl ToString + Sync, impl ToString + Sync)],
    ) -> Result<SolrSelectResponse<D>>
    where
        D: Serialize + DeserializeOwned,
    {
        let params: Vec<(String, String)> = params
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let url = Url::parse_with_params(&self.select_url, &params)
            .map_err(|e| SolrCoreError::InvalidUrlError(e))?;
        let uri = url
            .as_str()
            .parse::<Uri>()
            .map_err(|e| SolrCoreError::UriParseError(e))?;
        let res = self
            .client
            .get(uri)
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;
        let body = hyper::body::to_bytes(res.into_body())
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;
        let result: SolrSelectResponse<D> =
            serde_json::from_slice(&body).map_err(|e| SolrCoreError::DeserializeError(e))?;

        Ok(result)
    }

    async fn post(&self, body: Vec<u8>) -> Result<SolrSimpleResponse> {
        let req = Request::builder()
            .method(Method::POST)
            .uri(&self.post_url)
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .map_err(|e| SolrCoreError::RequestBuildError(e))?;

        let res = self
            .client
            .request(req)
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;
        let body = hyper::body::to_bytes(res.into_body())
            .await
            .map_err(|e| SolrCoreError::RequestError(e))?;
        let result: SolrSimpleResponse =
            serde_json::from_slice(&body).map_err(|e| SolrCoreError::DeserializeError(e))?;

        Ok(result)
    }

    async fn commit(&self) -> Result<()> {
        self.post(br#"{"commit": {}}"#.to_vec()).await?;
        Ok(())
    }

    async fn optimize(&self) -> Result<()> {
        self.post(br#"{"optimize": {}}"#.to_vec()).await?;
        Ok(())
    }

    async fn rollback(&self) -> Result<()> {
        self.post(br#"{"rollback": {}}"#.to_vec()).await?;
        Ok(())
    }

    async fn truncate(&self) -> Result<()> {
        self.post(br#"{"delete":{"query": "*:*"}}"#.to_vec())
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::{DateTime, Utc};
    use serde::Deserialize;
    use serde_json::{self, Value};

    /// Normal system test to get core status.
    ///
    /// Run this test with the Docker container started with the following command.
    ///
    /// ```ignore
    /// docker run --rm -d -p 8983:8983 solr:9.1.0 solr-precreate example
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_get_status() {
        let core = StandaloneSolrCore::new("example", "http://localhost:8983").unwrap();
        let status = core.status().await.unwrap();

        assert_eq!(status.name, String::from("example"));
    }

    /// Normal system test of reload of the core.
    ///
    /// The reload is considered successful if the time elapsed between the start of the reload
    /// and the start of the reloaded core is less than or equal to 1 second.
    ///
    /// Run this test with the Docker container started with the following command.
    ///
    /// ```ignore
    /// docker run --rm -d -p 8983:8983 solr:9.1.0 solr-precreate example
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_reload() {
        let core = StandaloneSolrCore::new("example", "http://localhost:8983").unwrap();

        let before = Utc::now();

        core.reload().await.unwrap();

        let status = core.status().await.unwrap();
        let after = status.start_time.replace("Z", "+00:00");
        let after = DateTime::parse_from_rfc3339(&after)
            .unwrap()
            .with_timezone(&Utc);

        assert!(before < after);

        let duration = (after - before).num_milliseconds();
        assert!(duration.abs() < 1000);
    }

    #[derive(Serialize, Deserialize)]
    struct Document {
        id: String,
    }

    /// Normal system test of the function to ping api.
    ///
    /// Run this test with the Docker container started with the following command.
    ///
    /// ```ignore
    /// docker run --rm -d -p 8983:8983 solr:9.1.0 solr-precreate example
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_ping() {
        let core = StandaloneSolrCore::new("example", "http://localhost:8983").unwrap();
        let response = core.ping().await.unwrap();

        assert_eq!(response.status, String::from("OK"));
    }

    /// Normal system test of the function to search documents.
    ///
    /// Run this test with the Docker container started with the following command.
    ///
    /// ```ignore
    /// docker run --rm -d -p 8983:8983 solr:9.1.0 solr-precreate example
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_select_in_normal() {
        let core = StandaloneSolrCore::new("example", "http://localhost:8983").unwrap();

        let params = vec![("q".to_string(), "*:*".to_string())];
        let response = core.select::<Document>(&params).await.unwrap();

        assert_eq!(response.header.status, 0);
    }

    /// Anomaly system test of the function to search documents.
    ///
    /// If nonexistent field was specified, select() method will return error.
    #[tokio::test]
    #[ignore]
    async fn test_select_in_non_normal() {
        let core = StandaloneSolrCore::new("example", "http://localhost:8983").unwrap();

        let params = vec![("q".to_string(), "text_hoge:*".to_string())];
        let response = core.select::<Document>(&params).await;

        assert!(response.is_err());
    }

    /// Normal system test of the function to analyze the word.
    ///
    /// Run this test with the Docker container started with the following command.
    ///
    /// ```ignore
    /// docker run --rm -d -p 8983:8983 solr:9.1.0 solr-precreate example
    /// ```
    // #[tokio::test]
    // #[ignore]
    // async fn test_analyze() {
    //     let core = StandaloneSolrCore::new("example", "http://localhost:8983");

    //     let word = "solr-client";
    //     let expected = vec![String::from("solr"), String::from("client")];

    //     let actual = core.analyze(word, "text_en", "index").await.unwrap();

    //     assert_eq!(expected, actual);
    // }

    /// Test scenario to test the behavior of a series of process: post documents to core, reload core, search for document, delete documents.
    ///
    /// Run this test with the Docker container started with the following command.
    ///
    /// ```ignore
    /// docker run --rm -d -p 8983:8983 solr:9.1.0 solr-precreate example
    /// ```
    #[tokio::test]
    #[ignore]
    async fn test_scenario() {
        let core = StandaloneSolrCore::new("example", "http://localhost:8983").unwrap();

        // Define schema for test with Schema API
        let client = hyper::Client::new();
        let req = Request::builder()
            .method(Method::POST)
            .uri(
                format!("{}/schema", "http://localhost:8983/solr/example")
                    .parse::<Uri>()
                    .unwrap(),
            )
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::json!(
                    {
                        "add-field": [
                            {
                                "name": "name",
                                "type": "string",
                                "indexed": true,
                                "stored": true,
                                "multiValued": false
                            },
                            {
                                "name": "gender",
                                "type": "string",
                                "indexed": true,
                                "stored": true,
                                "multiValued": false
                            }
                        ]
                    }
                )
                .to_string(),
            ))
            .unwrap();

        client.request(req).await.unwrap();

        // Documents for test
        let documents = serde_json::json!(
            [
                {
                    "id": "001",
                    "name": "alice",
                    "gender": "female"
                },
                {
                    "id": "002",
                    "name": "bob",
                    "gender": "male"
                },
                {
                    "id": "003",
                    "name": "charles",
                    "gender": "male"
                }
            ]
        )
        .to_string()
        .as_bytes()
        .to_vec();

        // Reload core (Only for operation check)
        core.reload().await.unwrap();

        // Post the documents to core.
        core.post(documents).await.unwrap();
        core.commit().await.unwrap();
        let status = core.status().await.unwrap();

        // Verify that 3 documents are registered.
        assert_eq!(status.index.num_docs, 3);

        // Test to search document
        let result = core
            .select::<Value>(&[("q", "name:alice"), ("fl", "id,name,gender")])
            .await
            .unwrap();
        assert_eq!(result.response.num_found, 1);
        assert_eq!(
            result.response.docs,
            vec![serde_json::json!({"id": "001", "name": "alice", "gender": "female"})]
        );

        // Delete all documents.
        core.truncate().await.unwrap();
        core.commit().await.unwrap();
        let status = core.status().await.unwrap();
        // Verify that no documents in index.
        assert_eq!(status.index.num_docs, 0);
    }
}
