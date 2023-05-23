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
    fn sort(&mut self, sort: impl ToString + Sync + Send) -> &mut Self;
    fn start(&mut self, start: u32) -> &mut Self;
    fn rows(&mut self, rows: u32) -> &mut Self;
    fn fq(&mut self, fq: impl ToString + Sync + Send) -> &mut Self;
    fn fl(&mut self, fl: impl ToString + Sync + Send) -> &mut Self;
    fn debug(&mut self) -> &mut Self;
    fn wt(&mut self, wt: impl ToString + Sync + Send) -> &mut Self;
    fn facet(&mut self, facet: impl FacetQueryParameter) -> &mut Self;
    fn op(&mut self, op: Operator) -> &mut Self;
    fn build(self) -> Vec<(String, String)>;
}

pub trait SolrLuceneQueryBuilder: SolrCommonQueryBuilder {
    fn q(&mut self, q: impl ToString + Sync + Send) -> &mut Self;
    fn df(&mut self, df: impl ToString + Sync + Send) -> &mut Self;
    fn sow(&mut self, sow: bool) -> &mut Self;
}

pub trait SolrDisMaxQueryBuilder: SolrCommonQueryBuilder {
    fn q(&mut self, q: impl ToString + Sync + Send) -> &mut Self;
    fn qf(&mut self, qf: impl ToString + Sync + Send) -> &mut Self;
    fn qs(&mut self, qs: u32) -> &mut Self;
    fn pf(&mut self, pf: impl ToString + Sync + Send) -> &mut Self;
    fn ps(&mut self, ps: u32) -> &mut Self;
    fn mm(&mut self, mm: impl ToString + Sync + Send) -> &mut Self;
    fn q_alt(&mut self, q: impl ToString + Sync + Send) -> &mut Self;
    fn tie(&mut self, tie: f64) -> &mut Self;
    fn bq(&mut self, bq: impl ToString + Sync + Send) -> &mut Self;
    fn bf(&mut self, bf: impl ToString + Sync + Send) -> &mut Self;
}

pub trait SolrEDismaxQueryBuilder: SolrDisMaxQueryBuilder {
    /// Add `sow` parameter.
    fn sow(&mut self, sow: bool) -> &mut Self;
    /// Add `boost` parameter.
    fn boost(&mut self, boost: impl ToString + Sync + Send) -> &mut Self;
    /// Add `lowercaseOperators` parameter.
    fn lowercase_operators(&mut self, flag: bool) -> &mut Self;
    /// Add `pf2` parameter.
    fn pf2(&mut self, pf: impl ToString + Sync + Send) -> &mut Self;
    /// Add `ps2` parameter.
    fn ps2(&mut self, ps: u32) -> &mut Self;
    /// Add `pf3` parameter.
    fn pf3(&mut self, pf: impl ToString + Sync + Send) -> &mut Self;
    /// Add `ps3` parameter.
    fn ps3(&mut self, ps: u32) -> &mut Self;
    /// Add `stopwords` parameter.
    fn stopwords(&mut self, flag: bool) -> &mut Self;
    /// Add `uf` parameter.
    fn uf(&mut self, uf: impl ToString + Sync + Send) -> &mut Self;
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

impl SolrQueryBuilder {
    pub fn new() -> Self {
        Self {
            params: vec![],
            facet_enable: false,
        }
    }
}

impl SolrCommonQueryBuilder for SolrQueryBuilder {
    fn sort(&mut self, sort: impl ToString + Sync + Send) -> &mut Self {
        let sort = sort.to_string();
        if !sort.is_empty() {
            self.params.push(("sort".to_string(), sort));
        }
        self
    }
    fn start(&mut self, start: u32) -> &mut Self {
        self.params.push(("start".to_string(), start.to_string()));
        self
    }
    fn rows(&mut self, rows: u32) -> &mut Self {
        self.params.push(("rows".to_string(), rows.to_string()));
        self
    }
    fn fq(&mut self, fq: impl ToString + Sync + Send) -> &mut Self {
        let fq = fq.to_string();
        if !fq.is_empty() {
            self.params.push(("fq".to_string(), fq));
        }
        self
    }
    fn fl(&mut self, fl: impl ToString + Sync + Send) -> &mut Self {
        let fl = fl.to_string();
        if !fl.is_empty() {
            self.params.push(("fl".to_string(), fl));
        }
        self
    }
    fn debug(&mut self) -> &mut Self {
        self.params.push(("debug".to_string(), "all".to_string()));
        self.params
            .push(("debug.explain.structured".to_string(), "true".to_string()));
        self
    }
    fn wt(&mut self, wt: impl ToString + Sync + Send) -> &mut Self {
        let wt = wt.to_string();
        if !wt.is_empty() {
            self.params.push(("wt".to_string(), wt));
        }
        self
    }
    fn facet(&mut self, facet: impl FacetQueryParameter) -> &mut Self {
        if !self.facet_enable {
            self.params.push(("facet".to_string(), "true".to_string()));
            self.facet_enable = true;
        }
        self.params.append(&mut facet.build());
        self
    }
    fn op(&mut self, op: Operator) -> &mut Self {
        self.params.push(("q.op".to_string(), op.to_string()));
        self
    }
    fn build(self) -> Vec<(String, String)> {
        self.params
    }
}

impl SolrDisMaxQueryBuilder for SolrQueryBuilder {
    fn q(&mut self, q: impl ToString + Sync + Send) -> &mut Self {
        let q = q.to_string();
        if !q.is_empty() {
            self.params.push(("q".to_string(), q));
        }
        self
    }
    fn qf(&mut self, qf: impl ToString + Sync + Send) -> &mut Self {
        let qf = qf.to_string();
        if !qf.is_empty() {
            self.params.push(("qf".to_string(), qf));
        }
        self
    }
    fn qs(&mut self, qs: u32) -> &mut Self {
        self.params.push(("qs".to_string(), qs.to_string()));
        self
    }
    fn pf(&mut self, pf: impl ToString + Sync + Send) -> &mut Self {
        let pf = pf.to_string();
        if !pf.is_empty() {
            self.params.push(("pf".to_string(), pf));
        }
        self
    }
    fn ps(&mut self, ps: u32) -> &mut Self {
        self.params.push(("ps".to_string(), ps.to_string()));
        self
    }
    fn mm(&mut self, mm: impl ToString + Sync + Send) -> &mut Self {
        let mm = mm.to_string();
        if !mm.is_empty() {
            self.params.push(("mm".to_string(), mm));
        }
        self
    }
    fn q_alt(&mut self, q: impl ToString + Sync + Send) -> &mut Self {
        let q = q.to_string();
        if !q.is_empty() {
            self.params.push(("q.alt".to_string(), q));
        }
        self
    }
    fn tie(&mut self, tie: f64) -> &mut Self {
        self.params.push(("tie".to_string(), tie.to_string()));
        self
    }
    fn bq(&mut self, bq: impl ToString + Sync + Send) -> &mut Self {
        let bq = bq.to_string();
        if !bq.is_empty() {
            self.params.push(("bq".to_string(), bq));
        }
        self
    }
    fn bf(&mut self, bf: impl ToString + Sync + Send) -> &mut Self {
        let bf = bf.to_string();
        if !bf.is_empty() {
            self.params.push(("bf".to_string(), bf));
        }
        self
    }
}

impl SolrEDismaxQueryBuilder for SolrQueryBuilder {
    fn sow(&mut self, sow: bool) -> &mut Self {
        self.params.push(("sow".to_string(), sow.to_string()));
        self
    }
    fn boost(&mut self, boost: impl ToString + Sync + Send) -> &mut Self {
        let boost = boost.to_string();
        if !boost.is_empty() {
            self.params.push(("boost".to_string(), boost.to_string()));
        }
        self
    }
    fn lowercase_operators(&mut self, flag: bool) -> &mut Self {
        self.params
            .push(("lowercaseOperators".to_string(), flag.to_string()));
        self
    }
    fn pf2(&mut self, pf: impl ToString + Sync + Send) -> &mut Self {
        let pf = pf.to_string();
        if !pf.is_empty() {
            self.params.push(("pf2".to_string(), pf.to_string()));
        }
        self
    }
    fn ps2(&mut self, ps: u32) -> &mut Self {
        self.params.push(("ps2".to_string(), ps.to_string()));
        self
    }
    fn pf3(&mut self, pf: impl ToString + Sync + Send) -> &mut Self {
        let pf = pf.to_string();
        if !pf.is_empty() {
            self.params.push(("pf3".to_string(), pf.to_string()));
        }
        self
    }
    fn ps3(&mut self, ps: u32) -> &mut Self {
        self.params.push(("ps3".to_string(), ps.to_string()));
        self
    }
    fn stopwords(&mut self, flag: bool) -> &mut Self {
        self.params
            .push(("stopwords".to_string(), flag.to_string()));
        self
    }
    fn uf(&mut self, uf: impl ToString + Sync + Send) -> &mut Self {
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
    prefix: Option<String>,
    contains: Option<String>,
    ignore_case: Option<bool>,
    sort: Option<FieldFacetSortOrder>,
    limit: Option<i32>,
    offset: Option<u32>,
    min_count: Option<u32>,
    missing: Option<bool>,
    method: Option<FieldFacetMethod>,
    exists: Option<bool>,
}

impl FieldFacetQueryParameter {
    pub fn new(field: impl ToString + Sync + Send) -> Self {
        Self {
            field: field.to_string(),
            prefix: None,
            contains: None,
            ignore_case: None,
            sort: None,
            limit: None,
            offset: None,
            min_count: None,
            missing: None,
            method: None,
            exists: None,
        }
    }

    pub fn prefix(&mut self, prefix: impl ToString + Sync + Send) -> &mut Self {
        let prefix = prefix.to_string();
        if !prefix.is_empty() {
            self.prefix = Some(prefix);
        }
        self
    }

    pub fn contains(&mut self, contains: impl ToString + Sync + Send) -> &mut Self {
        let contains = contains.to_string();
        if !contains.is_empty() {
            self.contains = Some(contains);
        }
        self
    }

    pub fn ignore_case(&mut self, ignore_case: bool) -> &mut Self {
        self.ignore_case = Some(ignore_case);
        self
    }

    pub fn sort(&mut self, sort: FieldFacetSortOrder) -> &mut Self {
        self.sort = Some(sort);
        self
    }

    pub fn limit(&mut self, limit: i32) -> &mut Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(&mut self, offset: u32) -> &mut Self {
        self.offset = Some(offset);
        self
    }

    pub fn min_count(&mut self, min_count: u32) -> &mut Self {
        self.min_count = Some(min_count);
        self
    }

    pub fn missing(&mut self, missing: bool) -> &mut Self {
        self.missing = Some(missing);
        self
    }

    pub fn method(&mut self, method: FieldFacetMethod) -> &mut Self {
        self.method = Some(method);
        self
    }

    pub fn exists(&mut self, exists: bool) -> &mut Self {
        self.exists = Some(exists);
        self
    }
}

impl FacetQueryParameter for FieldFacetQueryParameter {
    fn build(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();
        params.push((String::from("facet.field"), self.field.clone()));
        if let Some(prefix) = &self.prefix {
            params.push((format!("f.{}.facet.prefix", self.field), prefix.clone()));
        }

        if let Some(contains) = &self.contains {
            params.push((
                format!("f.{}.facet.contains", self.field),
                contains.to_string(),
            ));
        }

        if let Some(ignore_case) = &self.ignore_case {
            params.push((
                format!("f.{}.facet.contains.ignoreCase", self.field),
                ignore_case.to_string(),
            ));
        }

        if let Some(sort) = &self.sort {
            params.push((format!("f.{}.facet.sort", self.field), sort.to_string()));
        }

        if let Some(limit) = &self.limit {
            params.push((format!("f.{}.facet.limit", self.field), limit.to_string()));
        }

        if let Some(offset) = &self.offset {
            params.push((format!("f.{}.facet.offset", self.field), offset.to_string()));
        }

        if let Some(min_count) = &self.min_count {
            params.push((
                format!("f.{}.facet.mincount", self.field),
                min_count.to_string(),
            ));
        }

        if let Some(missing) = &self.missing {
            params.push((
                format!("f.{}.facet.missing", self.field),
                missing.to_string(),
            ));
        }

        if let Some(method) = &self.method {
            params.push((format!("f.{}.facet.method", self.field), method.to_string()));
        }

        if let Some(exists) = &self.exists {
            params.push((format!("f.{}.facet.exists", self.field), exists.to_string()));
        }

        params
    }
}

pub enum RangeFacetOtherOptions {
    Before,
    After,
    Between,
    All,
    None,
}

impl fmt::Display for RangeFacetOtherOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RangeFacetOtherOptions::Before => write!(f, "before"),
            RangeFacetOtherOptions::After => write!(f, "after"),
            RangeFacetOtherOptions::Between => write!(f, "between"),
            RangeFacetOtherOptions::All => write!(f, "all"),
            RangeFacetOtherOptions::None => write!(f, "none"),
        }
    }
}

pub enum RangeFacetIncludeOptions {
    Lower,
    Upper,
    Edge,
    Outer,
    All,
}

impl fmt::Display for RangeFacetIncludeOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RangeFacetIncludeOptions::Lower => write!(f, "lower"),
            RangeFacetIncludeOptions::Upper => write!(f, "upper"),
            RangeFacetIncludeOptions::Edge => write!(f, "edge"),
            RangeFacetIncludeOptions::Outer => write!(f, "outer"),
            RangeFacetIncludeOptions::All => write!(f, "all"),
        }
    }
}

pub struct RangeFacetQueryParameter {
    field: String,
    start: String,
    end: String,
    gap: String,
    hardend: Option<bool>,
    other: Option<RangeFacetOtherOptions>,
    include: Option<RangeFacetIncludeOptions>,
}

impl RangeFacetQueryParameter {
    pub fn new(
        field: impl ToString + Sync + Send,
        start: impl ToString + Sync + Send,
        end: impl ToString + Sync + Send,
        gap: impl ToString + Sync + Send,
    ) -> Self {
        Self {
            field: field.to_string(),
            start: start.to_string(),
            end: end.to_string(),
            gap: gap.to_string(),
            hardend: None,
            other: None,
            include: None,
        }
    }

    pub fn hardend(&mut self, hardend: bool) -> &mut Self {
        self.hardend = Some(hardend);
        self
    }

    pub fn other(&mut self, other: RangeFacetOtherOptions) -> &mut Self {
        self.other = Some(other);
        self
    }

    pub fn include(&mut self, include: RangeFacetIncludeOptions) -> &mut Self {
        self.include = Some(include);
        self
    }
}

impl FacetQueryParameter for RangeFacetQueryParameter {
    fn build(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();

        params.push((String::from("facet.range"), self.field.clone()));
        params.push((
            format!("f.{}.facet.range.start", self.field),
            self.start.clone(),
        ));
        params.push((
            format!("f.{}.facet.range.end", self.field),
            self.end.clone(),
        ));
        params.push((
            format!("f.{}.facet.range.gap", self.field),
            self.gap.clone(),
        ));
        if let Some(hardend) = self.hardend {
            params.push((
                format!("f.{}.facet.hardend", self.field),
                hardend.to_string(),
            ))
        }
        if let Some(other) = &self.other {
            params.push((
                format!("f.{}.facet.range.other", self.field),
                other.to_string(),
            ))
        }

        if let Some(include) = &self.include {
            params.push((
                format!("f.{}.facet.range.include", self.field),
                include.to_string(),
            ))
        }

        params
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use itertools::{sorted, Itertools};

    #[test]
    fn test_with_no_params() {
        let builder = SolrQueryBuilder::new();
        assert!(builder.build().is_empty());
    }

    #[test]
    fn test_common_params() {
        let mut builder = SolrQueryBuilder::new();
        builder
            .start(10)
            .rows(20)
            .fq("name:alice")
            .fq("{!collapse field=grade}")
            .fl("id,name,grade");
        let expected = vec![
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

    #[test]
    fn test_with_facets() {
        let mut field_facet = FieldFacetQueryParameter::new("category");
        field_facet
            .prefix("A")
            .contains("like")
            .ignore_case(true)
            .sort(FieldFacetSortOrder::Count)
            .limit(100)
            .offset(0)
            .min_count(1)
            .missing(false)
            .method(FieldFacetMethod::Fc)
            .exists(false);
        let mut range_facet = RangeFacetQueryParameter::new("difficulty", 0, 2000, 400);
        range_facet
            .include(RangeFacetIncludeOptions::Lower)
            .other(RangeFacetOtherOptions::All);

        let mut builder = SolrQueryBuilder::new();
        builder.facet(field_facet).facet(range_facet);

        let expected = sorted(
            vec![
                ("facet", "true"),
                ("facet.field", "category"),
                ("f.category.facet.prefix", "A"),
                ("f.category.facet.contains", "like"),
                ("f.category.facet.contains.ignoreCase", "true"),
                ("f.category.facet.sort", "count"),
                ("f.category.facet.limit", "100"),
                ("f.category.facet.offset", "0"),
                ("f.category.facet.mincount", "1"),
                ("f.category.facet.missing", "false"),
                ("f.category.facet.method", "fc"),
                ("f.category.facet.exists", "false"),
                ("facet.range", "difficulty"),
                ("f.difficulty.facet.range.start", "0"),
                ("f.difficulty.facet.range.end", "2000"),
                ("f.difficulty.facet.range.gap", "400"),
                ("f.difficulty.facet.range.other", "all"),
                ("f.difficulty.facet.range.include", "lower"),
            ]
            .iter()
            .map(|p| (p.0.to_string(), p.1.to_string())),
        )
        .collect_vec();

        assert_eq!(sorted(builder.build().into_iter()).collect_vec(), expected);
    }
}
