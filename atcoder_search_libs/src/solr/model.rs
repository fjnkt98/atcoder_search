use chrono::{DateTime, FixedOffset, Local, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{DeserializeAs, SerializeAs};
use std::{collections::BTreeMap, string::ToString};

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrResponseHeader {
    #[serde(alias = "zkConnected")]
    pub zk_connected: Option<Value>,
    pub status: u32,
    #[serde(alias = "QTime")]
    pub qtime: u32,
    pub params: Option<BTreeMap<String, Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrPingResponse {
    #[serde(alias = "responseHeader")]
    pub header: SolrResponseHeader,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrErrorInfo {
    pub metadata: Vec<String>,
    pub msg: String,
    pub code: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LuceneInfo {
    #[serde(alias = "solr-spec-version")]
    pub solr_spec_version: String,
    #[serde(alias = "solr-impl-version")]
    pub solr_impl_version: String,
    #[serde(alias = "lucene-spec-version")]
    pub lucene_spec_version: String,
    #[serde(alias = "lucene-impl-version")]
    pub lucene_impl_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrSystemInfo {
    #[serde(alias = "responseHeader")]
    pub header: SolrResponseHeader,
    pub mode: String,
    pub solr_home: String,
    pub core_root: String,
    pub lucene: LuceneInfo,
    pub jvm: Value,
    pub security: Value,
    pub system: Value,
    pub error: Option<SolrErrorInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SolrIndexInfo {
    #[serde(alias = "numDocs")]
    pub num_docs: u64,
    #[serde(alias = "maxDoc")]
    pub max_doc: u64,
    #[serde(alias = "deletedDocs")]
    pub deleted_docs: u64,
    pub version: u64,
    #[serde(alias = "segmentCount")]
    pub segment_count: u64,
    pub current: bool,
    #[serde(alias = "hasDeletions")]
    pub has_deletions: bool,
    pub directory: String,
    #[serde(alias = "segmentsFile")]
    pub segments_file: String,
    #[serde(alias = "segmentsFileSizeInBytes")]
    pub segments_file_size_in_bytes: u64,
    #[serde(alias = "userData")]
    pub user_data: Value,
    #[serde(alias = "sizeInBytes")]
    pub size_in_bytes: u64,
    pub size: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SolrCoreStatus {
    pub name: String,
    #[serde(alias = "instanceDir")]
    pub instance_dir: String,
    #[serde(alias = "dataDir")]
    pub data_dir: String,
    pub config: String,
    pub schema: String,
    #[serde(alias = "startTime")]
    pub start_time: String,
    pub uptime: u64,
    pub index: SolrIndexInfo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrCoreList {
    #[serde(alias = "responseHeader")]
    pub header: SolrResponseHeader,
    #[serde(alias = "initFailures")]
    pub init_failures: Value,
    pub status: Option<BTreeMap<String, SolrCoreStatus>>,
    pub error: Option<SolrErrorInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrSimpleResponse {
    #[serde(alias = "responseHeader")]
    pub header: SolrResponseHeader,
    pub error: Option<SolrErrorInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrSelectResponse<D, F> {
    #[serde(alias = "responseHeader")]
    pub header: SolrResponseHeader,
    pub response: SolrSelectBody<D>,
    pub facets: Option<F>,
    pub error: Option<SolrErrorInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrSelectBody<D> {
    #[serde(alias = "numFound")]
    pub num_found: u32,
    pub start: u32,
    #[serde(alias = "numFoundExact")]
    pub num_found_exact: bool,
    pub docs: Vec<D>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Bucket<T> {
    val: T,
    count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrTermFacetCount {
    buckets: Vec<Bucket<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrRangeFacetCount<T> {
    buckets: Vec<Bucket<T>>,
    before: Option<SolrRangeFacetCountInfo>,
    after: Option<SolrRangeFacetCountInfo>,
    between: Option<SolrRangeFacetCountInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrRangeFacetCountInfo {
    count: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrQueryFacetCount {
    buckets: Vec<Bucket<String>>,
}

/// Model of the `analysis` field in the response JSON of a request to `/solr/<CORE_NAME>/analysis/field`.
#[derive(Serialize, Deserialize, Debug)]
pub struct SolrAnalysisBody {
    pub field_types: BTreeMap<String, SolrAnalysisField>,
    pub field_names: BTreeMap<String, SolrAnalysisField>,
}

/// Model of the `field_types` or `field_names` field in the response JSON of a request to `/solr/<CORE_NAME>/analysis/field`.
#[derive(Serialize, Deserialize, Debug)]
pub struct SolrAnalysisField {
    pub index: Option<Vec<Value>>,
    pub query: Option<Vec<Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SolrAnalysisResponse {
    #[serde(alias = "responseHeader")]
    pub header: SolrResponseHeader,
    pub analysis: SolrAnalysisBody,
    pub error: Option<SolrErrorInfo>,
}

pub struct FromSolrDateTime;

impl SerializeAs<DateTime<FixedOffset>> for FromSolrDateTime {
    fn serialize_as<S>(source: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&source.with_timezone(&Utc).to_rfc3339())
    }
}

impl<'de> DeserializeAs<'de, DateTime<FixedOffset>> for FromSolrDateTime {
    fn deserialize_as<D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let timestamp = DateTime::parse_from_rfc3339(&value.replace("Z", "+00:00"))
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;
        Ok(timestamp)
    }
}

impl SerializeAs<DateTime<Utc>> for FromSolrDateTime {
    fn serialize_as<S>(source: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&source.to_rfc3339())
    }
}

impl<'de> DeserializeAs<'de, DateTime<Utc>> for FromSolrDateTime {
    fn deserialize_as<D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let timestamp = DateTime::parse_from_rfc3339(&value.replace("Z", "+00:00"))
            .map_err(|e| serde::de::Error::custom(e.to_string()))?
            .with_timezone(&Utc);

        Ok(timestamp)
    }
}

impl SerializeAs<DateTime<Local>> for FromSolrDateTime {
    fn serialize_as<S>(source: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&source.to_rfc3339())
    }
}

impl<'de> DeserializeAs<'de, DateTime<Local>> for FromSolrDateTime {
    fn deserialize_as<D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let timestamp = value
            .parse::<DateTime<FixedOffset>>()
            .map_err(|e| serde::de::Error::custom(e.to_string()))?
            .with_timezone(&Local);
        Ok(timestamp)
    }
}

pub struct IntoSolrDateTime;

impl SerializeAs<DateTime<FixedOffset>> for IntoSolrDateTime {
    fn serialize_as<S>(source: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(
            &source
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        )
    }
}

impl SerializeAs<DateTime<Utc>> for IntoSolrDateTime {
    fn serialize_as<S>(source: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&source.to_rfc3339_opts(SecondsFormat::Secs, true))
    }
}

impl SerializeAs<DateTime<Local>> for IntoSolrDateTime {
    fn serialize_as<S>(source: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(
            &source
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_with::serde_as;

    #[test]
    fn test_deserialize_response_header() {
        let raw = r#"
        {
            "status": 400,
            "QTime": 7,
            "params": {
                "facet.range": "difficulty",
                "q": "text_ja:高橋",
                "facet.field": "category",
                "f.difficulty.facet.end": "2000",
                "f.category.facet.mincount": "1",
                "f.difficulty.facet.start": "0",
                "facet": "true",
                "f.difficulty.gap": "800"
            }
        }
        "#;
        let header: SolrResponseHeader = serde_json::from_str(raw).unwrap();
        assert_eq!(header.status, 400);
        assert_eq!(header.qtime, 7);
    }

    #[test]
    fn test_deserialize_error_info() {
        let raw = r#"
        {
            "metadata": [
                "error-class",
                "org.apache.solr.common.SolrException",
                "root-error-class",
                "org.apache.solr.common.SolrException"
            ],
            "msg": "Missing required parameter: f.difficulty.facet.range.start (or default: facet.range.start)",
            "code": 400
        }
        "#;

        let error: SolrErrorInfo = serde_json::from_str(raw).unwrap();
        assert_eq!(error.msg, "Missing required parameter: f.difficulty.facet.range.start (or default: facet.range.start)".to_string());
        assert_eq!(error.code, 400);
    }

    #[test]
    fn test_deserialize_lucene_info() {
        let raw = r#"
        {
            "solr-spec-version": "9.1.0",
            "solr-impl-version": "9.1.0 aa4f3d98ab19c201e7f3c74cd14c99174148616d - ishan - 2022-11-11 13:00:47",
            "lucene-spec-version": "9.3.0",
            "lucene-impl-version": "9.3.0 d25cebcef7a80369f4dfb9285ca7360a810b75dc - ivera - 2022-07-25 12:30:23"
        }
        "#;

        let info: LuceneInfo = serde_json::from_str(raw).unwrap();
        assert_eq!(info.solr_spec_version, "9.1.0".to_string());
    }

    #[test]
    fn test_deserialize_solr_system_info() {
        let raw = r#"
        {
            "responseHeader": {
                "status": 0,
                "QTime": 17
            },
            "mode": "std",
            "solr_home": "/var/solr/data",
            "core_root": "/var/solr/data",
            "lucene": {
                "solr-spec-version": "9.1.0",
                "solr-impl-version": "9.1.0 aa4f3d98ab19c201e7f3c74cd14c99174148616d - ishan - 2022-11-11 13:00:47",
                "lucene-spec-version": "9.3.0",
                "lucene-impl-version": "9.3.0 d25cebcef7a80369f4dfb9285ca7360a810b75dc - ivera - 2022-07-25 12:30:23"
            },
            "jvm": {
                "version": "17.0.5 17.0.5+8",
                "name": "Eclipse Adoptium OpenJDK 64-Bit Server VM",
                "spec": {
                "vendor": "Oracle Corporation",
                "name": "Java Platform API Specification",
                "version": "17"
                },
                "jre": {
                "vendor": "Eclipse Adoptium",
                "version": "17.0.5"
                },
                "vm": {
                "vendor": "Eclipse Adoptium",
                "name": "OpenJDK 64-Bit Server VM",
                "version": "17.0.5+8"
                },
                "processors": 16,
                "memory": {
                "free": "410.9 MB",
                "total": "512 MB",
                "max": "512 MB",
                "used": "101.1 MB (%19.7)",
                "raw": {
                    "free": 430868656,
                    "total": 536870912,
                    "max": 536870912,
                    "used": 106002256,
                    "used%": 19.74445879459381
                }
                },
                "jmx": {
                "classpath": "start.jar",
                "commandLineArgs": [
                    "-Xms512m",
                    "-Xmx512m",
                    "-XX:+UseG1GC",
                    "-XX:+PerfDisableSharedMem",
                    "-XX:+ParallelRefProcEnabled",
                    "-XX:MaxGCPauseMillis=250",
                    "-XX:+UseLargePages",
                    "-XX:+AlwaysPreTouch",
                    "-XX:+ExplicitGCInvokesConcurrent",
                    "-Xlog:gc*:file=/var/solr/logs/solr_gc.log:time,uptime:filecount=9,filesize=20M",
                    "-Dsolr.jetty.inetaccess.includes=",
                    "-Dsolr.jetty.inetaccess.excludes=",
                    "-Dsolr.log.dir=/var/solr/logs",
                    "-Djetty.port=8983",
                    "-DSTOP.PORT=7983",
                    "-DSTOP.KEY=solrrocks",
                    "-Duser.timezone=UTC",
                    "-XX:-OmitStackTraceInFastThrow",
                    "-XX:OnOutOfMemoryError=/opt/solr/bin/oom_solr.sh 8983 /var/solr/logs",
                    "-Djetty.home=/opt/solr/server",
                    "-Dsolr.solr.home=/var/solr/data",
                    "-Dsolr.data.home=",
                    "-Dsolr.install.dir=/opt/solr",
                    "-Dsolr.default.confdir=/opt/solr/server/solr/configsets/_default/conf",
                    "-Dlog4j.configurationFile=/var/solr/log4j2.xml",
                    "-Dsolr.jetty.host=0.0.0.0",
                    "-Xss256k",
                    "-XX:CompileCommand=exclude,com.github.benmanes.caffeine.cache.BoundedLocalCache::put",
                    "-Djava.security.manager",
                    "-Djava.security.policy=/opt/solr/server/etc/security.policy",
                    "-Djava.security.properties=/opt/solr/server/etc/security.properties",
                    "-Dsolr.internal.network.permission=*",
                    "-DdisableAdminUI=false"
                ],
                "startTime": "2023-01-26T14:06:26.026Z",
                "upTimeMS": 47574
                }
            },
            "security": {},
            "system": {
                "name": "Linux",
                "arch": "amd64",
                "availableProcessors": 16,
                "systemLoadAverage": 1.88,
                "version": "5.15.0-58-generic",
                "committedVirtualMemorySize": 6041583616,
                "cpuLoad": 0.0625,
                "freeMemorySize": 153268224,
                "freePhysicalMemorySize": 153268224,
                "freeSwapSpaceSize": 8422940672,
                "processCpuLoad": 0.5,
                "processCpuTime": 11970000000,
                "systemCpuLoad": 0,
                "totalMemorySize": 7512129536,
                "totalPhysicalMemorySize": 7512129536,
                "totalSwapSpaceSize": 10737410048,
                "maxFileDescriptorCount": 1048576,
                "openFileDescriptorCount": 156
            }
            }
        "#;

        let info: SolrSystemInfo = serde_json::from_str(raw).unwrap();
        assert_eq!(info.header.qtime, 17);
    }

    #[test]
    fn test_deserialize_index_info() {
        let raw = r#"
        {
            "numDocs": 0,
            "maxDoc": 0,
            "deletedDocs": 0,
            "version": 2,
            "segmentCount": 0,
            "current": true,
            "hasDeletions": false,
            "directory": "org.apache.lucene.store.NRTCachingDirectory:NRTCachingDirectory(MMapDirectory@/var/solr/data/atcoder/data/index lockFactory=org.apache.lucene.store.NativeFSLockFactory@404f935c; maxCacheMB=48.0 maxMergeSizeMB=4.0)",
            "segmentsFile": "segments_1",
            "segmentsFileSizeInBytes": 69,
            "userData": {},
            "sizeInBytes": 69,
            "size": "69 bytes"
        }
        "#;
        let info: SolrIndexInfo = serde_json::from_str(raw).unwrap();
        assert_eq!(info.num_docs, 0);
    }

    #[test]
    fn test_deserialize_core_list() {
        let raw = r#"
        {
            "responseHeader": {
                "status": 0,
                "QTime": 1
            },
            "initFailures": {},
            "status": {
                "atcoder": {
                "name": "atcoder",
                "instanceDir": "/var/solr/data/atcoder",
                "dataDir": "/var/solr/data/atcoder/data/",
                "config": "solrconfig.xml",
                "schema": "schema.xml",
                "startTime": "2023-01-26T14:06:28.956Z",
                "uptime": 321775,
                "index": {
                    "numDocs": 0,
                    "maxDoc": 0,
                    "deletedDocs": 0,
                    "version": 2,
                    "segmentCount": 0,
                    "current": true,
                    "hasDeletions": false,
                    "directory": "org.apache.lucene.store.NRTCachingDirectory:NRTCachingDirectory(MMapDirectory@/var/solr/data/atcoder/data/index lockFactory=org.apache.lucene.store.NativeFSLockFactory@404f935c; maxCacheMB=48.0 maxMergeSizeMB=4.0)",
                    "segmentsFile": "segments_1",
                    "segmentsFileSizeInBytes": 69,
                    "userData": {},
                    "sizeInBytes": 69,
                    "size": "69 bytes"
                }
                }
            }
        }
        "#;
        let info: SolrCoreList = serde_json::from_str(raw).unwrap();

        assert_eq!(
            info.status
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<String>>(),
            vec![String::from("atcoder")]
        );
    }

    #[test]
    fn test_deserialize_simple_response() {
        let raw = r#"
        {
            "responseHeader": {
                "status": 0,
                "QTime": 181
            }
        }
        "#;

        let response: SolrSimpleResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(response.header.qtime, 181);
    }

    #[test]
    fn test_deserialize_simple_response_with_error() {
        let raw = r#"
        {
            "responseHeader": {
                "status": 400,
                "QTime": 0
            },
            "error": {
                "metadata": [
                "error-class",
                "org.apache.solr.common.SolrException",
                "root-error-class",
                "org.apache.solr.common.SolrException"
                ],
                "msg": "No such core: hoge",
                "code": 400
            }
        }
        "#;

        let response: SolrSimpleResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(response.error.unwrap().code, 400);
    }

    #[allow(dead_code)]
    #[serde_as]
    #[derive(Deserialize)]
    struct Document {
        problem_id: String,
        problem_title: String,
        problem_url: String,
        contest_id: String,
        contest_title: String,
        contest_url: String,
        difficulty: i64,
        #[serde_as(as = "FromSolrDateTime")]
        start_at: DateTime<FixedOffset>,
        duration: i64,
        rate_change: String,
        category: String,
    }

    #[test]
    fn test_deserialize_select_body() {
        let raw = r#"
        {
            "numFound": 5650,
            "start": 0,
            "numFoundExact": true,
            "docs": [
                {
                    "problem_id": "APG4b_a",
                    "problem_title": "A. 1.00.はじめに",
                    "problem_url": "https://atcoder.jp/contests/APG4b/tasks/APG4b_a",
                    "contest_id": "APG4b",
                    "contest_title": "C++入門 AtCoder Programming Guide for beginners (APG4b)",
                    "contest_url": "https://atcoder.jp/contests/APG4b",
                    "difficulty": 0,
                    "start_at": "1970-01-01T00:00:00Z",
                    "duration": -1141367296,
                    "rate_change": "-",
                    "category": "Other Contests",
                    "_version_": 1756245857733181400
                }
            ]
        }
        "#;

        let body: SolrSelectBody<Document> = serde_json::from_str(raw).unwrap();
        assert_eq!(body.num_found, 5650);
    }

    #[test]
    fn test_deserialize_select_response() {
        let raw = r#"
        {
            "responseHeader": {
                "status": 0,
                "QTime": 27,
                "params": {}
            },
            "response": {
                "numFound": 0,
                "start": 0,
                "numFoundExact": true,
                "docs": []
            }
        }
        "#;
        let select: SolrSelectResponse<Document, ()> = serde_json::from_str(raw).unwrap();
        assert_eq!(select.response.num_found, 0);
    }
}
