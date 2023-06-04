use core::fmt;
use once_cell::sync::Lazy;
use regex::Regex;
use unicode_normalization::UnicodeNormalization;

/// Regex object for sanitizing the [Solr special characters](https://solr.apache.org/guide/solr/latest/query-guide/standard-query-parser.html#escaping-special-characters).
pub static SOLR_SPECIAL_CHARACTERS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\+|\-|&&|\|\||!|\(|\)|\{|\}|\[|\]|\^|"|\~|\*|\?|:|/|AND|OR)"#).unwrap()
});

pub fn sanitize(s: &str) -> String {
    SOLR_SPECIAL_CHARACTERS
        .replace_all(&s.nfkc().collect::<String>(), r"\$0")
        .to_string()
}

#[derive(Clone, PartialEq, Eq)]
pub enum Operator {
    AND,
    OR,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operator::AND => write!(f, "AND"),
            Operator::OR => write!(f, "OR"),
        }
    }
}

pub struct EDisMaxQueryBuilder {
    params: Vec<(&'static str, String)>,
}

impl EDisMaxQueryBuilder {
    pub fn new() -> Self {
        Self {
            params: vec![("defType", String::from("edismax"))],
        }
    }
    pub fn build(self) -> Vec<(String, String)> {
        self.params
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect()
    }
    pub fn sort(mut self, sort: impl ToString + Sync + Send) -> Self {
        let sort = sort.to_string();
        if !sort.is_empty() {
            self.params.push(("sort", sort));
        }
        self
    }
    pub fn start(mut self, start: u32) -> Self {
        self.params.push(("start", start.to_string()));
        self
    }
    pub fn rows(mut self, rows: u32) -> Self {
        self.params.push(("rows", rows.to_string()));
        self
    }
    pub fn fq(mut self, fq: impl ToString + Sync + Send) -> Self {
        let fq = fq.to_string();
        if !fq.is_empty() {
            self.params.push(("fq", fq));
        }
        self
    }
    pub fn fl(mut self, fl: impl ToString + Sync + Send) -> Self {
        let fl = fl.to_string();
        if !fl.is_empty() {
            self.params.push(("fl", fl));
        }
        self
    }
    pub fn debug(mut self) -> Self {
        self.params.push(("debug", "all".to_string()));
        self.params
            .push(("debug.explain.structured", "true".to_string()));
        self
    }
    pub fn wt(mut self, wt: impl ToString + Sync + Send) -> Self {
        let wt = wt.to_string();
        if !wt.is_empty() {
            self.params.push(("wt", wt));
        }
        self
    }
    pub fn facet(mut self, facet: impl ToString + Sync + Send) -> Self {
        let facet = facet.to_string();
        if !facet.is_empty() {
            self.params.push(("json.facet", facet.to_string()));
        }
        self
    }
    pub fn op(mut self, op: Operator) -> Self {
        self.params.push(("q.op", op.to_string()));
        self
    }
    pub fn df(mut self, df: impl ToString + Sync + Send) -> Self {
        let df = df.to_string();
        if !df.is_empty() {
            self.params.push(("df", df));
        }
        self
    }
    pub fn q(mut self, q: impl ToString + Sync + Send) -> Self {
        let q = q.to_string();
        if !q.is_empty() {
            self.params.push(("q", q));
        }
        self
    }
    pub fn qf(mut self, qf: impl ToString + Sync + Send) -> Self {
        let qf = qf.to_string();
        if !qf.is_empty() {
            self.params.push(("qf", qf));
        }
        self
    }
    pub fn qs(mut self, qs: u32) -> Self {
        self.params.push(("qs", qs.to_string()));
        self
    }
    pub fn pf(mut self, pf: impl ToString + Sync + Send) -> Self {
        let pf = pf.to_string();
        if !pf.is_empty() {
            self.params.push(("pf", pf));
        }
        self
    }
    pub fn ps(mut self, ps: u32) -> Self {
        self.params.push(("ps", ps.to_string()));
        self
    }
    pub fn mm(mut self, mm: impl ToString + Sync + Send) -> Self {
        let mm = mm.to_string();
        if !mm.is_empty() {
            self.params.push(("mm", mm));
        }
        self
    }
    pub fn q_alt(mut self, q: impl ToString + Sync + Send) -> Self {
        let q = q.to_string();
        if !q.is_empty() {
            self.params.push(("q.alt", q));
        }
        self
    }
    pub fn tie(mut self, tie: f64) -> Self {
        self.params.push(("tie", tie.to_string()));
        self
    }
    pub fn bq(mut self, bq: impl ToString + Sync + Send) -> Self {
        let bq = bq.to_string();
        if !bq.is_empty() {
            self.params.push(("bq", bq));
        }
        self
    }
    pub fn bf(mut self, bf: impl ToString + Sync + Send) -> Self {
        let bf = bf.to_string();
        if !bf.is_empty() {
            self.params.push(("bf", bf));
        }
        self
    }
    pub fn sow(mut self, sow: bool) -> Self {
        self.params.push(("sow", sow.to_string()));
        self
    }
    pub fn boost(mut self, boost: impl ToString + Sync + Send) -> Self {
        let boost = boost.to_string();
        if !boost.is_empty() {
            self.params.push(("boost", boost.to_string()));
        }
        self
    }
    pub fn lowercase_operators(mut self, flag: bool) -> Self {
        self.params.push(("lowercaseOperators", flag.to_string()));
        self
    }
    pub fn pf2(mut self, pf: impl ToString + Sync + Send) -> Self {
        let pf = pf.to_string();
        if !pf.is_empty() {
            self.params.push(("pf2", pf.to_string()));
        }
        self
    }
    pub fn ps2(mut self, ps: u32) -> Self {
        self.params.push(("ps2", ps.to_string()));
        self
    }
    pub fn pf3(mut self, pf: impl ToString + Sync + Send) -> Self {
        let pf = pf.to_string();
        if !pf.is_empty() {
            self.params.push(("pf3", pf.to_string()));
        }
        self
    }
    pub fn ps3(mut self, ps: u32) -> Self {
        self.params.push(("ps3", ps.to_string()));
        self
    }
    pub fn stopwords(mut self, flag: bool) -> Self {
        self.params.push(("stopwords", flag.to_string()));
        self
    }
    pub fn uf(mut self, uf: impl ToString + Sync + Send) -> Self {
        let uf = uf.to_string();
        if !uf.is_empty() {
            self.params.push(("uf3", uf.to_string()));
        }
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn test_with_no_params() {
        let builder = EDisMaxQueryBuilder::new();
        assert_eq!(
            builder.build(),
            vec![(String::from("defType"), String::from("edismax"))]
        );
    }

    #[test]
    fn test_common_params() {
        let builder = EDisMaxQueryBuilder::new()
            .start(10)
            .rows(20)
            .fq("name:alice")
            .fq("{!collapse field=grade}")
            .fl("id,name,grade");
        let expected = vec![
            ("defType", "edismax"),
            ("start", "10"),
            ("rows", "20"),
            ("fq", "name:alice"),
            ("fq", "{!collapse field=grade}"),
            ("fl", "id,name,grade"),
        ]
        .iter()
        .map(|param| (param.0.to_string(), param.1.to_string()))
        .collect_vec();
        assert_eq!(builder.build(), expected);
    }
}
