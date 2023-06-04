use crate::solr::model::*;
use async_trait::async_trait;
use hyper::header::CONTENT_TYPE;
use reqwest::{self, Body, Client, Url};
use serde::{de::DeserializeOwned, Serialize};
use serde_json;
use thiserror::Error;

type Result<T> = std::result::Result<T, SolrCoreError>;

#[derive(Debug, Error)]
pub enum SolrCoreError {
    #[error("failed to request to solr core")]
    RequestError(#[from] reqwest::Error),
    #[error("failed to deserialize JSON data")]
    DeserializeError(#[from] serde_json::Error),
    #[error("invalid Solr url given")]
    InvalidUrlError(#[from] url::ParseError),
    #[error("core not found")]
    CoreNotFoundError(String),
    #[error("{0}")]
    UnexpectedError(String),
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
    async fn post<T: Into<Body> + Send>(&self, body: T) -> Result<SolrSimpleResponse>;
    async fn commit(&self) -> Result<()>;
    async fn optimize(&self) -> Result<()>;
    async fn rollback(&self) -> Result<()>;
    async fn truncate(&self) -> Result<()>;
}

pub struct StandaloneSolrCore {
    name: String,
    admin_url: Url,
    ping_url: Url,
    post_url: Url,
    select_url: Url,
    client: Client,
}

impl StandaloneSolrCore {
    pub fn new(name: &str, solr_url: &str) -> Result<Self> {
        let mut solr_url = Url::parse(solr_url)?;
        solr_url.set_path("");
        let base_url = solr_url;
        let admin_url = base_url.join("solr/admin/cores")?;
        let ping_url = base_url.join(&format!("solr/{}/admin/ping", name))?;
        let post_url = base_url.join(&format!("solr/{}/update", name))?;
        let select_url = base_url.join(&format!("solr/{}/select", name))?;

        let client = Client::new();
        Ok(StandaloneSolrCore {
            name: String::from(name),
            admin_url,
            ping_url,
            post_url,
            select_url,
            client,
        })
    }
}

#[async_trait]
impl SolrCore for StandaloneSolrCore {
    async fn ping(&self) -> Result<SolrPingResponse> {
        let res = self.client.get(self.ping_url.clone()).send().await?;
        match res.error_for_status_ref() {
            Ok(_) => {
                let body: SolrPingResponse = res.json().await?;
                Ok(body)
            }
            Err(e) => {
                let body: SolrSimpleResponse = res.json().await?;
                let msg = body
                    .error
                    .and_then(|error| Some(error.msg))
                    .unwrap_or(String::default());
                Err(SolrCoreError::UnexpectedError(format!(
                    "unexpected error [{}] cause [{}]",
                    e.to_string(),
                    msg
                )))
            }
        }
    }

    async fn status(&self) -> Result<SolrCoreStatus> {
        let res = self
            .client
            .get(self.admin_url.clone())
            .query(&[("action", "STATUS"), ("core", &self.name)])
            .send()
            .await?;
        match res.error_for_status_ref() {
            Ok(_) => {
                let core_list: SolrCoreList = res.json().await?;
                let status = core_list
                    .status
                    .and_then(|status| status.get(&self.name).cloned())
                    .ok_or(SolrCoreError::CoreNotFoundError(String::from(
                        "core not found",
                    )))?;

                Ok(status)
            }
            Err(e) => {
                let body: SolrSimpleResponse = res.json().await?;
                let msg = body
                    .error
                    .and_then(|error| Some(error.msg))
                    .unwrap_or(String::default());
                Err(SolrCoreError::UnexpectedError(format!(
                    "unexpected error [{}] cause [{}]",
                    e.to_string(),
                    msg
                )))
            }
        }
    }

    async fn reload(&self) -> Result<SolrSimpleResponse> {
        let res = self
            .client
            .get(self.admin_url.clone())
            .query(&[("action", "RELOAD"), ("core", &self.name)])
            .send()
            .await?;
        match res.error_for_status_ref() {
            Ok(_) => {
                let body: SolrSimpleResponse = res.json().await?;
                Ok(body)
            }
            Err(e) => {
                let body: SolrSimpleResponse = res.json().await?;
                let msg = body
                    .error
                    .and_then(|error| Some(error.msg))
                    .unwrap_or(String::default());
                Err(SolrCoreError::UnexpectedError(format!(
                    "unexpected error [{}] cause [{}]",
                    e.to_string(),
                    msg
                )))
            }
        }
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
        let res = self
            .client
            .get(self.select_url.clone())
            .query(&params)
            .send()
            .await?;
        match res.error_for_status_ref() {
            Ok(_) => {
                let body: SolrSelectResponse<D> = res.json().await?;
                Ok(body)
            }
            Err(e) => {
                let body: SolrSimpleResponse = res.json().await?;
                let msg = body
                    .error
                    .and_then(|error| Some(error.msg))
                    .unwrap_or(String::default());
                Err(SolrCoreError::UnexpectedError(format!(
                    "unexpected error [{}] cause [{}]",
                    e.to_string(),
                    msg
                )))
            }
        }
    }

    async fn post<T: Into<Body> + Send>(&self, body: T) -> Result<SolrSimpleResponse> {
        let res = self
            .client
            .post(self.post_url.clone())
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await?;

        match res.error_for_status_ref() {
            Ok(_) => {
                let body: SolrSimpleResponse = res.json().await?;
                Ok(body)
            }
            Err(e) => {
                let body: SolrSimpleResponse = res.json().await?;
                let msg = body
                    .error
                    .and_then(|error| Some(error.msg))
                    .unwrap_or(String::default());
                Err(SolrCoreError::UnexpectedError(format!(
                    "unexpected error [{}] cause [{}]",
                    e.to_string(),
                    msg
                )))
            }
        }
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

    #[test]
    fn create_new_core() {
        let core = StandaloneSolrCore::new("example", "http://localhost:8983").unwrap();

        assert_eq!(
            core.admin_url,
            Url::parse("http://localhost:8983/solr/admin/cores").unwrap()
        );
        assert_eq!(
            core.ping_url,
            Url::parse("http://localhost:8983/solr/example/admin/ping").unwrap()
        );
        assert_eq!(
            core.post_url,
            Url::parse("http://localhost:8983/solr/example/update").unwrap()
        );
        assert_eq!(
            core.select_url,
            Url::parse("http://localhost:8983/solr/example/select").unwrap()
        );
    }

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
        let client = reqwest::Client::new();
        client
            .post(format!("http://localhost:8983/solr/example/schema"))
            .body(
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
            )
            .send()
            .await
            .unwrap();

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
