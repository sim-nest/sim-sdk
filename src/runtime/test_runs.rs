use sim_kernel::{
    CapabilityName, Cx, Datum, DatumStore, Effect, Error, Ref, Result, Symbol, TestReport, Value,
    card::ref_value, effect_abort_op_key, effect_ledger::EffectLedger, effect_resume_op_key,
    effect_test_run_kind,
};

use super::browse::schema::TestReportBuilder;
use super::browse_run_tests_capability;

pub(crate) fn run_effect_backed<F>(
    cx: &mut Cx,
    name: Symbol,
    mode: Symbol,
    capabilities: &[CapabilityName],
    run_inner: F,
) -> Result<TestReport>
where
    F: FnOnce(&mut Cx) -> Result<TestReport>,
{
    cx.require(&browse_run_tests_capability())?;

    let input = test_run_input(cx, &name, &mode)?;
    let mut requires = vec![browse_run_tests_capability()];
    requires.extend(capabilities.iter().cloned());
    let mut effect = Effect::new(
        effect_test_run_kind(),
        Ref::Symbol(name.clone()),
        input,
        Ref::Symbol(Symbol::qualified("browse", "TestReport")),
        effect_resume_op_key(),
        effect_abort_op_key(),
    )
    .with_requirements(requires);
    effect.ensure_replay_key(None)?;

    let effect_ref = effect.id.clone();
    record_requested(cx, effect)?;

    let mut events = vec![test_event_value(cx, "start", &name, &mode, None, None)?];
    let mut report = match missing_capability(cx, capabilities) {
        Some(capability) => {
            let detail = format!("skipped: missing capability {capability}");
            events.push(test_event_value(
                cx,
                "skip",
                &name,
                &mode,
                Some(false),
                Some(&detail),
            )?);
            TestReport::skipped(name.clone(), Some(detail))
        }
        None => match run_inner(cx) {
            Ok(report) => {
                let event = if report.skipped {
                    "skip"
                } else if report.passed {
                    "pass"
                } else {
                    "fail"
                };
                events.push(test_event_value(
                    cx,
                    event,
                    &name,
                    &mode,
                    Some(report.passed),
                    report.detail.as_deref(),
                )?);
                report
            }
            Err(err) => {
                let error = error_ref(cx, &err)?;
                record_failed(cx, effect_ref, error)?;
                return Err(err);
            }
        },
    };

    report.mode = mode;
    report.effect = Some(ref_value(cx, &effect_ref)?);
    report.events = events;

    let result = report_record_ref(cx, &report)?;
    record_resolved(cx, effect_ref, result)?;
    Ok(report)
}

pub(crate) fn test_report_value(cx: &mut Cx, report: TestReport) -> Result<Value> {
    let mut builder = TestReportBuilder::new(report.name);
    builder.passed = report.passed;
    builder.mode = report.mode;
    builder.detail = report.detail;
    builder.events = report.events;
    builder.effect = report.effect;
    builder.shape_report = report.shape_report;
    builder.build(cx)
}

fn missing_capability(cx: &Cx, capabilities: &[CapabilityName]) -> Option<CapabilityName> {
    capabilities
        .iter()
        .find(|capability| !cx.capabilities().contains(capability))
        .cloned()
}

fn record_requested(cx: &mut Cx, effect: Effect) -> Result<()> {
    with_effect_ledger(cx, |cx, ledger| {
        ledger.record_requested(cx.datum_store_mut(), effect)?;
        Ok(())
    })
}

fn record_resolved(cx: &mut Cx, effect: Ref, result: Ref) -> Result<()> {
    with_effect_ledger(cx, |cx, ledger| {
        ledger.record_resolved(cx.datum_store_mut(), effect, result)?;
        Ok(())
    })
}

fn record_failed(cx: &mut Cx, effect: Ref, error: Ref) -> Result<()> {
    with_effect_ledger(cx, |cx, ledger| {
        ledger.record_failed(cx.datum_store_mut(), effect, error)?;
        Ok(())
    })
}

fn with_effect_ledger<T>(
    cx: &mut Cx,
    f: impl FnOnce(&mut Cx, &mut EffectLedger) -> Result<T>,
) -> Result<T> {
    let mut ledger = std::mem::take(cx.effect_ledger_mut());
    let result = f(cx, &mut ledger);
    *cx.effect_ledger_mut() = ledger;
    result
}

fn test_run_input(cx: &mut Cx, name: &Symbol, mode: &Symbol) -> Result<Ref> {
    intern_ref(
        cx,
        Datum::Node {
            tag: Symbol::qualified("browse", "TestRun"),
            fields: vec![
                (Symbol::new("name"), Datum::Symbol(name.clone())),
                (Symbol::new("mode"), Datum::Symbol(mode.clone())),
            ],
        },
    )
}

fn test_event_value(
    cx: &mut Cx,
    kind: &'static str,
    name: &Symbol,
    mode: &Symbol,
    passed: Option<bool>,
    detail: Option<&str>,
) -> Result<Value> {
    let mut fields = vec![
        (
            Symbol::new("kind"),
            Datum::Symbol(Symbol::qualified("test", kind)),
        ),
        (Symbol::new("name"), Datum::Symbol(name.clone())),
        (Symbol::new("mode"), Datum::Symbol(mode.clone())),
    ];
    if let Some(passed) = passed {
        fields.push((Symbol::new("passed"), Datum::Bool(passed)));
    }
    if let Some(detail) = detail {
        fields.push((Symbol::new("detail"), Datum::String(detail.to_owned())));
    }
    let reference = intern_ref(
        cx,
        Datum::Node {
            tag: Symbol::qualified("browse", "TestEvent"),
            fields,
        },
    )?;
    ref_value(cx, &reference)
}

fn report_record_ref(cx: &mut Cx, report: &TestReport) -> Result<Ref> {
    let mut fields = vec![
        (Symbol::new("name"), Datum::Symbol(report.name.clone())),
        (Symbol::new("passed"), Datum::Bool(report.passed)),
        (Symbol::new("mode"), Datum::Symbol(report.mode.clone())),
        (Symbol::new("status"), Datum::Symbol(report_status(report))),
    ];
    if let Some(detail) = &report.detail {
        fields.push((Symbol::new("detail"), Datum::String(detail.clone())));
    }
    intern_ref(
        cx,
        Datum::Node {
            tag: Symbol::qualified("browse", "TestReport"),
            fields,
        },
    )
}

fn report_status(report: &TestReport) -> Symbol {
    if report.skipped {
        Symbol::qualified("test", "skip")
    } else if report.passed {
        Symbol::qualified("test", "pass")
    } else {
        Symbol::qualified("test", "fail")
    }
}

fn error_ref(cx: &mut Cx, err: &Error) -> Result<Ref> {
    intern_ref(cx, Datum::String(err.to_string()))
}

fn intern_ref(cx: &mut Cx, datum: Datum) -> Result<Ref> {
    cx.datum_store_mut().intern(datum).map(Ref::Content)
}
