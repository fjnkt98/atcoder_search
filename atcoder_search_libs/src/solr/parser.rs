use core::fmt;
use once_cell::sync::Lazy;
use regex::Regex;

/// Regex object for sanitizing the [Solr special characters](https://solr.apache.org/guide/solr/latest/query-guide/standard-query-parser.html#escaping-special-characters).
pub static SOLR_SPECIAL_CHARACTERS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\+|\-|&&|\|\||!|\(|\)|\{|\}|\[|\]|\^|"|\~|\*|\?|:|/|AND|OR)"#).unwrap()
});

pub fn sanitize(s: &str) -> String {
    SOLR_SPECIAL_CHARACTERS.replace_all(s, r"\$0").to_string()
}

pub trait SolrCommonQueryBuilder {
    fn sort(self, sort: impl ToString + Sync + Send) -> Self;
    fn start(self, start: u32) -> Self;
    fn rows(self, rows: u32) -> Self;
    fn fq(self, fq: impl ToString + Sync + Send) -> Self;
    fn fl(self, fl: impl ToString + Sync + Send) -> Self;
    fn debug(self) -> Self;
    fn wt(self, wt: impl ToString + Sync + Send) -> Self;
    fn facet(self, facet: &impl FacetQueryParameter) -> Self;
    fn op(self, op: Operator) -> Self;
    fn build(self) -> Vec<(String, String)>;
}

pub trait SolrLuceneQueryBuilder: SolrCommonQueryBuilder {
    fn q(self, q: impl ToString + Sync + Send) -> Self;
    fn df(self, df: impl ToString + Sync + Send) -> Self;
    fn sow(self, sow: bool) -> Self;
}

pub trait SolrDisMaxQueryBuilder: SolrCommonQueryBuilder {
    fn q(self, q: impl ToString + Sync + Send) -> Self;
    fn qf(self, qf: impl ToString + Sync + Send) -> Self;
    fn qs(self, qs: u32) -> Self;
    fn pf(self, pf: impl ToString + Sync + Send) -> Self;
    fn ps(self, ps: u32) -> Self;
    fn mm(self, mm: impl ToString + Sync + Send) -> Self;
    fn q_alt(self, q: impl ToString + Sync + Send) -> Self;
    fn tie(self, tie: f64) -> Self;
    fn bq(self, bq: impl ToString + Sync + Send) -> Self;
    fn bf(self, bf: impl ToString + Sync + Send) -> Self;
}

pub trait SolrEDismaxQueryBuilder: SolrDisMaxQueryBuilder {
    /// Add `sow` parameter.
    fn sow(self, sow: bool) -> Self;
    /// Add `boost` parameter.
    fn boost(self, boost: impl ToString + Sync + Send) -> Self;
    /// Add `lowercaseOperators` parameter.
    fn lowercase_operators(self, flag: bool) -> Self;
    /// Add `pf2` parameter.
    fn pf2(self, pf: impl ToString + Sync + Send) -> Self;
    /// Add `ps2` parameter.
    fn ps2(self, ps: u32) -> Self;
    /// Add `pf3` parameter.
    fn pf3(self, pf: impl ToString + Sync + Send) -> Self;
    /// Add `ps3` parameter.
    fn ps3(self, ps: u32) -> Self;
    /// Add `stopwords` parameter.
    fn stopwords(self, flag: bool) -> Self;
    /// Add `uf` parameter.
    fn uf(self, uf: impl ToString + Sync + Send) -> Self;
}

pub trait FacetQueryParameter {
    fn build(&self) -> Vec<(String, String)>;
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

pub struct SolrQueryBuilder {
    params: Vec<(String, String)>,
    facet_enable: bool,
}

impl SolrCommonQueryBuilder for SolrQueryBuilder {
    fn sort(mut self, sort: impl ToString + Sync + Send) -> Self {
        let sort = sort.to_string();
        if !sort.is_empty() {
            self.params.push(("sort".to_string(), sort));
        }
        self
    }
    fn start(mut self, start: u32) -> Self {
        let start = start.to_string();
        if !start.is_empty() {
            self.params.push(("start".to_string(), start));
        }
        self
    }
    fn rows(mut self, rows: u32) -> Self {
        let rows = rows.to_string();
        if !rows.is_empty() {
            self.params.push(("rows".to_string(), rows));
        }
        self
    }
    fn fq(mut self, fq: impl ToString + Sync + Send) -> Self {
        let fq = fq.to_string();
        if !fq.is_empty() {
            self.params.push(("fq".to_string(), fq));
        }
        self
    }
    fn fl(mut self, fl: impl ToString + Sync + Send) -> Self {
        let fl = fl.to_string();
        if !fl.is_empty() {
            self.params.push(("fl".to_string(), fl));
        }
        self
    }
    fn debug(mut self) -> Self {
        self.params.push(("debug".to_string(), "all".to_string()));
        self.params
            .push(("debug.explain.structured".to_string(), "true".to_string()));
        self
    }
    fn wt(mut self, wt: impl ToString + Sync + Send) -> Self {
        let wt = wt.to_string();
        if !wt.is_empty() {
            self.params.push(("wt".to_string(), wt));
        }
        self
    }
    fn facet(mut self, facet: &impl FacetQueryParameter) -> Self {
        if !self.facet_enable {
            self.params.push(("facet".to_string(), "true".to_string()));
            self.facet_enable = true;
        }
        self.params.append(&mut facet.build());
        self
    }
    fn op(mut self, op: Operator) -> Self {
        self.params.push(("q.op".to_string(), op.to_string()));
        self
    }
    fn build(self) -> Vec<(String, String)> {
        self.params
    }
}

impl SolrDisMaxQueryBuilder for SolrQueryBuilder {
    fn q(mut self, q: impl ToString + Sync + Send) -> Self {
        let q = q.to_string();
        if !q.is_empty() {
            self.params.push(("q".to_string(), q));
        }
        self
    }
    fn qf(mut self, qf: impl ToString + Sync + Send) -> Self {
        let qf = qf.to_string();
        if !qf.is_empty() {
            self.params.push(("qf".to_string(), qf));
        }
        self
    }
    fn qs(mut self, qs: u32) -> Self {
        self.params.push(("qs".to_string(), qs.to_string()));
        self
    }
    fn pf(mut self, pf: impl ToString + Sync + Send) -> Self {
        let pf = pf.to_string();
        if !pf.is_empty() {
            self.params.push(("pf".to_string(), pf));
        }
        self
    }
    fn ps(mut self, ps: u32) -> Self {
        self.params.push(("ps".to_string(), ps.to_string()));
        self
    }
    fn mm(mut self, mm: impl ToString + Sync + Send) -> Self {
        let mm = mm.to_string();
        if !mm.is_empty() {
            self.params.push(("mm".to_string(), mm));
        }
        self
    }
    fn q_alt(mut self, q: impl ToString + Sync + Send) -> Self {
        let q = q.to_string();
        if !q.is_empty() {
            self.params.push(("q.alt".to_string(), q));
        }
        self
    }
    fn tie(mut self, tie: f64) -> Self {
        self.params.push(("tie".to_string(), tie.to_string()));
        self
    }
    fn bq(mut self, bq: impl ToString + Sync + Send) -> Self {
        let bq = bq.to_string();
        if !bq.is_empty() {
            self.params.push(("bq".to_string(), bq));
        }
        self
    }
    fn bf(mut self, bf: impl ToString + Sync + Send) -> Self {
        let bf = bf.to_string();
        if !bf.is_empty() {
            self.params.push(("bf".to_string(), bf));
        }
        self
    }
}

impl SolrEDismaxQueryBuilder for SolrQueryBuilder {
    fn sow(mut self, sow: bool) -> Self {
        self.params.push(("sow".to_string(), sow.to_string()));
        self
    }
    fn boost(mut self, boost: impl ToString + Sync + Send) -> Self {
        let boost = boost.to_string();
        if !boost.is_empty() {
            self.params.push(("boost".to_string(), boost.to_string()));
        }
        self
    }
    fn lowercase_operators(mut self, flag: bool) -> Self {
        self.params
            .push(("lowercaseOperators".to_string(), flag.to_string()));
        self
    }
    fn pf2(mut self, pf: impl ToString + Sync + Send) -> Self {
        let pf = pf.to_string();
        if !pf.is_empty() {
            self.params.push(("pf2".to_string(), pf.to_string()));
        }
        self
    }
    fn ps2(mut self, ps: u32) -> Self {
        self.params.push(("ps2".to_string(), ps.to_string()));
        self
    }
    fn pf3(mut self, pf: impl ToString + Sync + Send) -> Self {
        let pf = pf.to_string();
        if !pf.is_empty() {
            self.params.push(("pf3".to_string(), pf.to_string()));
        }
        self
    }
    fn ps3(mut self, ps: u32) -> Self {
        self.params.push(("ps3".to_string(), ps.to_string()));
        self
    }
    fn stopwords(mut self, flag: bool) -> Self {
        self.params
            .push(("stopwords".to_string(), flag.to_string()));
        self
    }
    fn uf(mut self, uf: impl ToString + Sync + Send) -> Self {
        let uf = uf.to_string();
        if !uf.is_empty() {
            self.params.push(("uf3".to_string(), uf.to_string()));
        }
        self
    }
}

pub enum FieldFacetSortOrder {
    Index,
    Count,
}
impl fmt::Display for FieldFacetSortOrder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FieldFacetSortOrder::Index => write!(f, "index"),
            FieldFacetSortOrder::Count => write!(f, "count"),
        }
    }
}

pub enum FieldFacetMethod {
    Enum,
    Fc,
    Fcs,
}
impl fmt::Display for FieldFacetMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FieldFacetMethod::Enum => write!(f, "enum"),
            FieldFacetMethod::Fc => write!(f, "fc"),
            FieldFacetMethod::Fcs => write!(f, "fcs"),
        }
    }
}

pub struct FieldFacetQueryParameter {
    field: String,
    params: Vec<(String, String)>,
}

impl FieldFacetQueryParameter {
    pub fn new(field: impl ToString + Sync + Send) -> Self {
        Self {
            field: field.to_string(),
            params: Vec::new(),
        }
    }

    pub fn prefix(mut self, prefix: impl ToString + Sync + Send) -> Self {
        let prefix = prefix.to_string();
        if !prefix.is_empty() {
            self.params
                .push((format!("f.{}.facet.prefix", self.field), prefix));
        }
        self
    }

    pub fn contains(mut self, contains: impl ToString + Sync + Send) -> Self {
        let contains = contains.to_string();
        if !contains.is_empty() {
            self.params
                .push((format!("f.{}.facet.contains", self.field), contains));
        }
        self
    }

    pub fn ignore_case(mut self, ignore_case: bool) -> Self {
        self.params.push((
            format!("f.{}.facet.ignoreCase", self.field),
            ignore_case.to_string(),
        ));
        self
    }

    pub fn sort(mut self, sort: FieldFacetSortOrder) -> Self {
        self.params
            .push((format!("f.{}.facet.sort", self.field), sort.to_string()));
        self
    }

    pub fn limit(mut self, limit: i32) -> Self {
        self.params
            .push((format!("f.{}.facet.limit", self.field), limit.to_string()));
        self
    }

    pub fn offset(mut self, offset: u32) -> Self {
        self.params
            .push((format!("f.{}.facet.offset", self.field), offset.to_string()));
        self
    }

    pub fn min_count(mut self, min_count: u32) -> Self {
        self.params.push((
            format!("f.{}.facet.mincount", self.field),
            min_count.to_string(),
        ));
        self
    }

    pub fn missing(mut self, missing: bool) -> Self {
        self.params.push((
            format!("f.{}.facet.missing", self.field),
            missing.to_string(),
        ));
        self
    }

    pub fn method(mut self, method: FieldFacetMethod) -> Self {
        self.params
            .push((format!("f.{}.facet.method", self.field), method.to_string()));
        self
    }

    pub fn exists(mut self, exists: bool) -> Self {
        self.params
            .push((format!("f.{}.facet.exists", self.field), exists.to_string()));
        self
    }
}

impl FacetQueryParameter for FieldFacetQueryParameter {
    fn build(&self) -> Vec<(String, String)> {
        self.params.clone()
    }
}
