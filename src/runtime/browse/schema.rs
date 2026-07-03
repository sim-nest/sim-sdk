#![cfg_attr(not(test), allow(dead_code))]

use sim_kernel::{Cx, Ref, Result, Symbol, Value};

use super::{coverage, evidence, facets, help_normalize, predicates, surface_facets};

const CARD_V1_FIELDS: &[&str] = &[
    "subject",
    "kind",
    "help",
    "args",
    "result",
    "tests",
    "ops",
    "requires",
    "see-also",
    "shape-known",
];

pub(crate) const CARD_V2_FIELDS: &[&str] = &[
    "subject",
    "kind",
    "help",
    "args",
    "result",
    "tests",
    "ops",
    "requires",
    "see-also",
    "shape-known",
    "facets",
    "coverage",
    "provenance",
    "freshness",
];

pub(crate) const HELP_FIELDS: &[&str] = &[
    "subject",
    "kind",
    "summary",
    "detail",
    "exported-by",
    "stability",
    "capabilities",
    "demand",
    "see-also",
];

pub(crate) const BROWSE_TEST_FIELDS: &[&str] = &[
    "name",
    "subjects",
    "lib",
    "mode",
    "expr",
    "expr-codec",
    "expected",
    "expected-codec",
    "expected-error",
    "codecs",
    "example",
    "capabilities",
];

pub(crate) const COVERAGE_FIELDS: &[&str] = &[
    "tests", "examples", "runnable", "passed", "failed", "skipped", "last-run", "stale",
];

pub(crate) const FACET_FIELDS: &[&str] = &[
    "name",
    "version",
    "kind",
    "shape",
    "value",
    "requires",
    "visibility",
    "evidence",
];

pub(crate) const REDACTION_FIELDS: &[&str] = &["reason", "requires", "summary"];

pub(crate) const TEST_REPORT_FIELDS: &[&str] = &[
    "name",
    "passed",
    "mode",
    "detail",
    "events",
    "effect",
    "shape-report",
];

pub(crate) fn card_v2_for_ref(cx: &mut Cx, subject: Ref) -> Result<Value> {
    let card_v1 = sim_kernel::card::card_for_ref(cx, subject.clone())?;
    card_v2_from_card_v1(cx, subject, card_v1)
}

pub(crate) fn card_v2_from_card_v1(cx: &mut Cx, subject: Ref, card_v1: Value) -> Result<Value> {
    let defaults = sim_kernel::card::minimal_card(cx, subject.clone())?;
    let default_entries = table_entries(cx, defaults)?;
    let card_entries = table_entries(cx, card_v1)?;
    let mut spine = CARD_V1_FIELDS
        .iter()
        .map(|field| {
            let key = field_symbol(field);
            let value = find_field(&card_entries, field)
                .or_else(|| find_field(&default_entries, field))
                .cloned()
                .ok_or_else(|| {
                    sim_kernel::Error::HostError(format!("missing Card field {field}"))
                })?;
            Ok((key, value))
        })
        .collect::<Result<Vec<_>>>()?;
    if let Some(help) = evidence::claim_scalar(cx, &subject, predicates::help_doc_predicate())? {
        replace_field(&mut spine, "help", help);
    }
    help_normalize::normalize_help_field(cx, &subject, &mut spine)?;
    let mut builder = CardV2Builder::from_spine(spine);
    if let Some(tests) = find_field(&builder.spine, "tests").cloned() {
        builder.coverage = Some(coverage::coverage_from_tests(cx, tests)?);
    }
    builder.provenance = evidence::shape_provenance(cx, &subject)?;
    builder.facets = Some(facets_from_subject(
        cx,
        &subject,
        &builder.spine,
        builder.provenance.clone(),
    )?);
    builder.build(cx)
}

fn facets_from_subject(
    cx: &mut Cx,
    subject: &Ref,
    spine: &[(Symbol, Value)],
    provenance: Option<Value>,
) -> Result<Value> {
    let mut values = facets::facets_from_card_spine(cx, subject, spine, provenance)?;
    values.extend(surface_facets::surface_facets(cx, subject, spine)?);
    cx.factory().list(values)
}

pub(crate) struct CardV2Builder {
    spine: Vec<(Symbol, Value)>,
    facets: Option<Value>,
    coverage: Option<Value>,
    provenance: Option<Value>,
    freshness: Option<Value>,
}

impl CardV2Builder {
    pub(crate) fn from_spine(spine: Vec<(Symbol, Value)>) -> Self {
        Self {
            spine,
            facets: None,
            coverage: None,
            provenance: None,
            freshness: None,
        }
    }

    pub(crate) fn build(self, cx: &mut Cx) -> Result<Value> {
        let mut entries = self.spine;
        entries.push((
            field_symbol("facets"),
            self.facets.unwrap_or(empty_list(cx)?),
        ));
        entries.push((
            field_symbol("coverage"),
            self.coverage
                .unwrap_or(CoverageBuilder::default().build(cx)?),
        ));
        entries.push((
            field_symbol("provenance"),
            self.provenance.unwrap_or(empty_list(cx)?),
        ));
        entries.push((
            field_symbol("freshness"),
            self.freshness.unwrap_or(status_symbol(cx, "unknown")?),
        ));
        cx.factory().table(entries)
    }
}

pub(crate) struct HelpBuilder {
    pub(crate) subject: Value,
    pub(crate) kind: Symbol,
    pub(crate) summary: String,
    pub(crate) detail: String,
    pub(crate) exported_by: Option<Value>,
    pub(crate) stability: Symbol,
    pub(crate) capabilities: Vec<Value>,
    pub(crate) demand: Vec<Value>,
    pub(crate) see_also: Vec<Value>,
}

impl HelpBuilder {
    pub(crate) fn new(subject: Value) -> Self {
        Self {
            subject,
            kind: Symbol::qualified("core", "unknown"),
            summary: String::new(),
            detail: String::new(),
            exported_by: None,
            stability: Symbol::new("unknown"),
            capabilities: Vec::new(),
            demand: Vec::new(),
            see_also: Vec::new(),
        }
    }

    pub(crate) fn build(self, cx: &mut Cx) -> Result<Value> {
        cx.factory().table(vec![
            (field_symbol("subject"), self.subject),
            (field_symbol("kind"), cx.factory().symbol(self.kind)?),
            (field_symbol("summary"), cx.factory().string(self.summary)?),
            (field_symbol("detail"), cx.factory().string(self.detail)?),
            (
                field_symbol("exported-by"),
                self.exported_by.unwrap_or(cx.factory().nil()?),
            ),
            (
                field_symbol("stability"),
                cx.factory().symbol(self.stability)?,
            ),
            (
                field_symbol("capabilities"),
                cx.factory().list(self.capabilities)?,
            ),
            (field_symbol("demand"), cx.factory().list(self.demand)?),
            (field_symbol("see-also"), cx.factory().list(self.see_also)?),
        ])
    }
}

pub(crate) struct BrowseTestBuilder {
    name: Symbol,
    subjects: Vec<Value>,
    lib: Symbol,
    mode: Symbol,
    expr: Option<Value>,
    expr_codec: Symbol,
    expected: Option<Value>,
    expected_codec: Option<Value>,
    expected_error: Option<String>,
    codecs: Vec<Value>,
    example: bool,
    capabilities: Vec<Value>,
}

impl BrowseTestBuilder {
    pub(crate) fn new(name: Symbol, lib: Symbol) -> Self {
        Self {
            name,
            subjects: Vec::new(),
            lib,
            mode: Symbol::new("unknown"),
            expr: None,
            expr_codec: Symbol::qualified("codec", "lisp"),
            expected: None,
            expected_codec: None,
            expected_error: None,
            codecs: Vec::new(),
            example: false,
            capabilities: Vec::new(),
        }
    }

    pub(crate) fn build(self, cx: &mut Cx) -> Result<Value> {
        cx.factory().table(vec![
            (field_symbol("name"), cx.factory().symbol(self.name)?),
            (field_symbol("subjects"), cx.factory().list(self.subjects)?),
            (field_symbol("lib"), cx.factory().symbol(self.lib)?),
            (field_symbol("mode"), cx.factory().symbol(self.mode)?),
            (
                field_symbol("expr"),
                self.expr.unwrap_or(cx.factory().nil()?),
            ),
            (
                field_symbol("expr-codec"),
                cx.factory().symbol(self.expr_codec)?,
            ),
            (
                field_symbol("expected"),
                self.expected.unwrap_or(cx.factory().nil()?),
            ),
            (
                field_symbol("expected-codec"),
                self.expected_codec.unwrap_or(cx.factory().nil()?),
            ),
            (
                field_symbol("expected-error"),
                match self.expected_error {
                    Some(value) => cx.factory().string(value)?,
                    None => cx.factory().nil()?,
                },
            ),
            (field_symbol("codecs"), cx.factory().list(self.codecs)?),
            (field_symbol("example"), cx.factory().bool(self.example)?),
            (
                field_symbol("capabilities"),
                cx.factory().list(self.capabilities)?,
            ),
        ])
    }
}

#[derive(Default)]
pub(crate) struct CoverageBuilder {
    pub(crate) tests: usize,
    pub(crate) examples: usize,
    pub(crate) runnable: usize,
    pub(crate) passed: Option<usize>,
    pub(crate) failed: Option<usize>,
    pub(crate) skipped: Option<usize>,
    pub(crate) last_run: Option<Value>,
    pub(crate) stale: bool,
}

impl CoverageBuilder {
    pub(crate) fn build(self, cx: &mut Cx) -> Result<Value> {
        cx.factory().table(vec![
            (field_symbol("tests"), integer(cx, self.tests)?),
            (field_symbol("examples"), integer(cx, self.examples)?),
            (field_symbol("runnable"), integer(cx, self.runnable)?),
            (field_symbol("passed"), optional_integer(cx, self.passed)?),
            (field_symbol("failed"), optional_integer(cx, self.failed)?),
            (field_symbol("skipped"), optional_integer(cx, self.skipped)?),
            (
                field_symbol("last-run"),
                self.last_run.unwrap_or(cx.factory().nil()?),
            ),
            (field_symbol("stale"), cx.factory().bool(self.stale)?),
        ])
    }
}

pub(crate) struct FacetBuilder {
    pub(crate) name: Symbol,
    pub(crate) version: usize,
    pub(crate) kind: Symbol,
    pub(crate) shape: Symbol,
    pub(crate) value: Option<Value>,
    pub(crate) requires: Vec<Value>,
    pub(crate) visibility: Symbol,
    pub(crate) evidence: Vec<Value>,
}

impl FacetBuilder {
    pub(crate) fn new(name: Symbol) -> Self {
        Self {
            name,
            version: 1,
            kind: Symbol::new("custom"),
            shape: Symbol::qualified("core", "Any"),
            value: None,
            requires: Vec::new(),
            visibility: Symbol::new("public"),
            evidence: Vec::new(),
        }
    }

    pub(crate) fn build(self, cx: &mut Cx) -> Result<Value> {
        cx.factory().table(vec![
            (field_symbol("name"), cx.factory().symbol(self.name)?),
            (field_symbol("version"), integer(cx, self.version)?),
            (field_symbol("kind"), cx.factory().symbol(self.kind)?),
            (field_symbol("shape"), cx.factory().symbol(self.shape)?),
            (
                field_symbol("value"),
                self.value.unwrap_or(cx.factory().nil()?),
            ),
            (field_symbol("requires"), cx.factory().list(self.requires)?),
            (
                field_symbol("visibility"),
                cx.factory().symbol(self.visibility)?,
            ),
            (field_symbol("evidence"), cx.factory().list(self.evidence)?),
        ])
    }
}

pub(crate) struct RedactionBuilder {
    pub(crate) reason: Symbol,
    pub(crate) requires: Vec<Value>,
    pub(crate) summary: String,
}

impl RedactionBuilder {
    pub(crate) fn unavailable() -> Self {
        Self {
            reason: Symbol::new("unavailable"),
            requires: Vec::new(),
            summary: String::new(),
        }
    }

    pub(crate) fn build(self, cx: &mut Cx) -> Result<Value> {
        cx.factory().table(vec![
            (field_symbol("reason"), cx.factory().symbol(self.reason)?),
            (field_symbol("requires"), cx.factory().list(self.requires)?),
            (field_symbol("summary"), cx.factory().string(self.summary)?),
        ])
    }
}

pub(crate) struct TestReportBuilder {
    name: Symbol,
    pub(crate) passed: bool,
    pub(crate) mode: Symbol,
    pub(crate) detail: Option<String>,
    pub(crate) events: Vec<Value>,
    pub(crate) effect: Option<Value>,
    pub(crate) shape_report: Option<Value>,
}

impl TestReportBuilder {
    pub(crate) fn new(name: Symbol) -> Self {
        Self {
            name,
            passed: false,
            mode: Symbol::new("unknown"),
            detail: None,
            events: Vec::new(),
            effect: None,
            shape_report: None,
        }
    }

    pub(crate) fn build(self, cx: &mut Cx) -> Result<Value> {
        cx.factory().table(vec![
            (field_symbol("name"), cx.factory().symbol(self.name)?),
            (field_symbol("passed"), cx.factory().bool(self.passed)?),
            (field_symbol("mode"), cx.factory().symbol(self.mode)?),
            (
                field_symbol("detail"),
                match self.detail {
                    Some(value) => cx.factory().string(value)?,
                    None => cx.factory().nil()?,
                },
            ),
            (field_symbol("events"), cx.factory().list(self.events)?),
            (
                field_symbol("effect"),
                self.effect.unwrap_or(cx.factory().nil()?),
            ),
            (
                field_symbol("shape-report"),
                self.shape_report.unwrap_or(cx.factory().nil()?),
            ),
        ])
    }
}

fn table_entries(cx: &mut Cx, value: Value) -> Result<Vec<(Symbol, Value)>> {
    if let Some(table) = value.object().as_table_impl() {
        return table.entries(cx);
    }
    let table = value.object().as_table(cx)?;
    match table.object().as_table_impl() {
        Some(table) => table.entries(cx),
        None => Ok(Vec::new()),
    }
}

use super::fields::{key as field_symbol, value_field as find_field};

fn replace_field(entries: &mut [(Symbol, Value)], name: &str, value: Value) {
    let key = field_symbol(name);
    if let Some((_, slot)) = entries.iter_mut().find(|(field, _)| field == &key) {
        *slot = value;
    }
}

fn optional_integer(cx: &Cx, value: Option<usize>) -> Result<Value> {
    match value {
        Some(value) => integer(cx, value),
        None => cx.factory().nil(),
    }
}

fn integer(cx: &Cx, value: usize) -> Result<Value> {
    cx.factory()
        .number_literal(Symbol::qualified("numbers", "i64"), value.to_string())
}

fn status_symbol(cx: &mut Cx, name: &'static str) -> Result<Value> {
    cx.factory().symbol(Symbol::new(name))
}
fn empty_list(cx: &mut Cx) -> Result<Value> {
    cx.factory().list(Vec::new())
}
