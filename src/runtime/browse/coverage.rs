use std::collections::{BTreeMap, BTreeSet};

use sim_kernel::{
    CapabilityName, Cx, Datum, DatumStore, Ref, Result, Symbol, Value, card::ref_value,
    force_list_to_vec,
};

use super::super::browse_run_tests_capability;
use super::super::test_runs::test_run_effect_kind;
use super::schema::CoverageBuilder;

pub(super) fn coverage_from_tests(cx: &mut Cx, tests: Value) -> Result<Value> {
    let tests = test_infos(cx, tests)?;
    let summary = RunSummary::latest(cx, &tests);
    let last_run = match &summary.last_run {
        Some(reference) => Some(ref_value(cx, reference)?),
        None => None,
    };

    CoverageBuilder {
        tests: tests.len(),
        examples: tests.iter().filter(|test| test.example).count(),
        runnable: tests.iter().filter(|test| test.runnable(cx)).count(),
        passed: summary.passed,
        failed: summary.failed,
        skipped: summary.skipped,
        last_run,
        stale: false,
    }
    .build(cx)
}

#[derive(Clone)]
struct TestInfo {
    name: Symbol,
    example: bool,
    capabilities: Vec<CapabilityName>,
}

impl TestInfo {
    fn runnable(&self, cx: &Cx) -> bool {
        cx.capabilities().contains(&browse_run_tests_capability())
            && self
                .capabilities
                .iter()
                .all(|capability| cx.capabilities().contains(capability))
    }
}

#[derive(Default)]
struct RunSummary {
    passed: Option<usize>,
    failed: Option<usize>,
    skipped: Option<usize>,
    last_run: Option<Ref>,
}

impl RunSummary {
    fn latest(cx: &Cx, tests: &[TestInfo]) -> Self {
        let names = tests
            .iter()
            .map(|test| test.name.clone())
            .collect::<BTreeSet<_>>();
        let mut latest = BTreeMap::<Symbol, RunStatus>::new();
        let mut last_run = None;

        for record in cx.effect_ledger().records() {
            let Some(effect) = cx.effect_ledger().effect(&record.effect) else {
                continue;
            };
            if effect.kind != test_run_effect_kind() {
                continue;
            }
            let Ref::Symbol(name) = &effect.subject else {
                continue;
            };
            if !names.contains(name) {
                continue;
            }
            let status = report_status(cx, record.result.as_ref())
                .or_else(|| record.aborted.then_some(RunStatus::Failed));
            let Some(status) = status else {
                continue;
            };
            latest.insert(name.clone(), status);
            if let Some(resolved) = &record.resolved_event {
                last_run = Some(resolved.clone());
            }
        }

        if latest.is_empty() {
            return Self::default();
        }

        Self {
            passed: Some(
                latest
                    .values()
                    .filter(|status| matches!(status, RunStatus::Passed))
                    .count(),
            ),
            failed: Some(
                latest
                    .values()
                    .filter(|status| matches!(status, RunStatus::Failed))
                    .count(),
            ),
            skipped: Some(
                latest
                    .values()
                    .filter(|status| matches!(status, RunStatus::Skipped))
                    .count(),
            ),
            last_run,
        }
    }
}

#[derive(Clone, Copy)]
enum RunStatus {
    Passed,
    Failed,
    Skipped,
}

fn test_infos(cx: &mut Cx, tests: Value) -> Result<Vec<TestInfo>> {
    let Some(list) = tests.object().as_list() else {
        return Ok(Vec::new());
    };
    force_list_to_vec(cx, list, "browse coverage tests")?
        .into_iter()
        .filter_map(|test| match test.object().as_expr(cx) {
            Ok(expr) => test_info(&expr).map(Ok),
            Err(err) => Some(Err(err)),
        })
        .collect()
}

fn test_info(expr: &sim_kernel::Expr) -> Option<TestInfo> {
    Some(TestInfo {
        name: symbol_field(expr, "name")?.clone(),
        example: bool_field(expr, "example").unwrap_or(false),
        capabilities: capabilities_field(expr, "capabilities"),
    })
}

fn report_status(cx: &Cx, result: Option<&Ref>) -> Option<RunStatus> {
    let Ref::Content(id) = result? else {
        return None;
    };
    let datum = cx.datum_store().get(id).ok().flatten()?;
    let Datum::Node { tag, fields } = datum else {
        return None;
    };
    if tag != &Symbol::qualified("browse", "TestReport") {
        return None;
    }
    let Datum::Symbol(status) = datum_field(fields.as_slice(), "status")? else {
        return None;
    };
    if status == &Symbol::qualified("test", "pass") {
        Some(RunStatus::Passed)
    } else if status == &Symbol::qualified("test", "skip") {
        Some(RunStatus::Skipped)
    } else if status == &Symbol::qualified("test", "fail") {
        Some(RunStatus::Failed)
    } else {
        None
    }
}

fn capabilities_field(expr: &sim_kernel::Expr, name: &str) -> Vec<CapabilityName> {
    let Some(sim_kernel::Expr::List(items)) = expr_field(expr, name) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| match item {
            sim_kernel::Expr::Symbol(symbol)
                if symbol.namespace.as_deref() == Some("capability") =>
            {
                Some(CapabilityName::new(symbol.name.to_string()))
            }
            sim_kernel::Expr::String(text) => Some(CapabilityName::new(text.clone())),
            _ => None,
        })
        .collect()
}

fn symbol_field<'a>(expr: &'a sim_kernel::Expr, name: &str) -> Option<&'a Symbol> {
    match expr_field(expr, name) {
        Some(sim_kernel::Expr::Symbol(symbol)) => Some(symbol),
        _ => None,
    }
}

use super::fields::{bool_field, expr_field};

fn datum_field<'a>(fields: &'a [(Symbol, Datum)], name: &str) -> Option<&'a Datum> {
    let key = Symbol::new(name.to_owned());
    fields
        .iter()
        .find_map(|(field, value)| (field == &key).then_some(value))
}
