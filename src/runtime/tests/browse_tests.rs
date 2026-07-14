use std::sync::Arc;

use sim_kernel::{
    Args, CapabilityName, Cx, DefaultFactory, EagerPolicy, Expr, Symbol, Test, Value,
};

use crate::runtime::{
    SimTest, TestExpected,
    browse::schema::{BROWSE_TEST_FIELDS, CARD_V2_FIELDS},
    browse_run_tests_capability, install_core_runtime,
};

use super::support::table_value;

#[test]
fn sim_test_description_uses_fixed_codec_aware_fields() {
    let mut cx = test_cx();
    let test = SimTest::new(
        Symbol::qualified("test", "value"),
        Symbol::qualified("test", "runtime"),
        Expr::Bool(true),
        TestExpected::Value(Expr::Bool(true)),
        vec![Symbol::qualified("core", "help")],
    )
    .as_example()
    .requiring(CapabilityName::new("demo"));

    let description = test.describe(&mut cx).unwrap();
    let description = expr(&mut cx, &description);

    assert_eq!(table_keys(&description), symbols(BROWSE_TEST_FIELDS));
    assert_eq!(
        table_value(&description, &Symbol::new("mode")),
        Some(&Expr::Symbol(Symbol::qualified("test", "value")))
    );
    assert_eq!(
        table_value(&description, &Symbol::new("expr-codec")),
        Some(&Expr::Symbol(Symbol::qualified("codec", "lisp")))
    );
    assert_eq!(
        table_value(&description, &Symbol::new("expected-codec")),
        Some(&Expr::Symbol(Symbol::qualified("codec", "lisp")))
    );
    assert_eq!(
        table_value(&description, &Symbol::new("example")),
        Some(&Expr::Bool(true))
    );
    assert_list_contains_symbol(
        table_value(&description, &Symbol::new("capabilities")).expect("capabilities"),
        Symbol::qualified("capability", "demo"),
    );
}

#[test]
fn subject_tests_and_examples_use_fixed_test_cards() {
    let mut cx = test_cx();
    register(&mut cx, example_test());
    register(
        &mut cx,
        SimTest::new(
            Symbol::qualified("test", "non-example"),
            Symbol::qualified("test", "runtime"),
            Expr::Bool(true),
            TestExpected::Truthy,
            vec![Symbol::qualified("core", "help")],
        ),
    );

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let tests = call(&mut cx, Symbol::qualified("core", "tests"), vec![subject]);
    let tests = expr(&mut cx, &tests);
    let test_items = list_items(&tests);
    assert_eq!(test_items.len(), 2);
    assert!(test_items.iter().all(|item| {
        table_value(item, &Symbol::new("expr-codec"))
            == Some(&Expr::Symbol(Symbol::qualified("codec", "lisp")))
    }));

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let examples = call(
        &mut cx,
        Symbol::qualified("core", "examples"),
        vec![subject],
    );
    let examples = expr(&mut cx, &examples);
    let example_items = list_items(&examples);
    assert_eq!(example_items.len(), 1);
    assert_eq!(
        table_value(&example_items[0], &Symbol::new("name")),
        Some(&Expr::Symbol(Symbol::qualified("test", "example")))
    );
    assert_eq!(
        table_value(&example_items[0], &Symbol::new("example")),
        Some(&Expr::Bool(true))
    );
}

#[test]
fn registered_test_is_browsable_and_roundtrip_metadata_is_visible() {
    let mut cx = test_cx();
    register(
        &mut cx,
        SimTest::new(
            Symbol::qualified("test", "roundtrip"),
            Symbol::qualified("test", "runtime"),
            Expr::String("stable".to_owned()),
            TestExpected::RoundTrip {
                codecs: vec![
                    Symbol::qualified("codec", "lisp"),
                    Symbol::qualified("codec", "json"),
                ],
            },
            vec![Symbol::qualified("core", "help")],
        ),
    );

    let tests = call(&mut cx, Symbol::qualified("core", "tests"), Vec::new());
    let tests = expr(&mut cx, &tests);
    let items = list_items(&tests);
    let roundtrip = test_by_name(items, Symbol::qualified("test", "roundtrip"));
    let codecs = table_value(roundtrip, &Symbol::new("codecs")).expect("codecs");
    assert_eq!(
        codecs,
        &Expr::List(vec![
            Expr::Symbol(Symbol::qualified("codec", "lisp")),
            Expr::Symbol(Symbol::qualified("codec", "json")),
        ])
    );

    let subject = symbol_value(&cx, Symbol::qualified("test", "roundtrip"));
    let card = call(&mut cx, Symbol::qualified("core", "browse"), vec![subject]);
    let card = expr(&mut cx, &card);
    assert_eq!(table_keys(&card), symbols(CARD_V2_FIELDS));
    assert_eq!(
        table_value(&card, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "test")))
    );
}

#[cfg(feature = "codec-lisp")]
#[test]
fn value_tests_roundtrip_expr_and_expected_through_recorded_codecs() {
    let mut cx = test_cx();
    install_lisp_codec(&mut cx);
    cx.grant(browse_run_tests_capability());
    let test = SimTest::new(
        Symbol::qualified("test", "codec-value"),
        Symbol::qualified("test", "runtime"),
        Expr::Bool(true),
        TestExpected::Value(Expr::Bool(true)),
        vec![Symbol::qualified("core", "help")],
    );

    let report = test.run(&mut cx).unwrap();
    assert!(report.passed, "{:?}", report.detail);
}

#[test]
fn run_tests_requires_capability_but_test_descriptions_stay_visible() {
    let mut cx = test_cx();
    register(&mut cx, example_test());

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let tests = call(&mut cx, Symbol::qualified("core", "tests"), vec![subject]);
    assert_eq!(list_items(&expr(&mut cx, &tests)).len(), 1);

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let err = try_call(
        &mut cx,
        Symbol::qualified("core", "run-tests"),
        vec![subject],
    )
    .unwrap_err();
    assert!(matches!(
        err,
        sim_kernel::Error::CapabilityDenied { capability }
            if capability == browse_run_tests_capability()
    ));
}

#[cfg(feature = "codec-lisp")]
#[test]
fn run_tests_returns_effect_reports_and_updates_coverage() {
    let mut cx = test_cx();
    install_lisp_codec(&mut cx);
    cx.grant(browse_run_tests_capability());
    register(&mut cx, example_test());
    register(
        &mut cx,
        SimTest::new(
            Symbol::qualified("test", "failing"),
            Symbol::qualified("test", "runtime"),
            Expr::Bool(false),
            TestExpected::Truthy,
            vec![Symbol::qualified("core", "help")],
        ),
    );

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let before = call(
        &mut cx,
        Symbol::qualified("core", "coverage"),
        vec![subject],
    );
    let before = expr(&mut cx, &before);
    assert_eq!(int_field(&before, "tests"), Some(2));
    assert_eq!(int_field(&before, "examples"), Some(1));
    assert_eq!(int_field(&before, "runnable"), Some(2));
    assert_eq!(
        table_value(&before, &Symbol::new("passed")),
        Some(&Expr::Nil)
    );

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let reports = call(
        &mut cx,
        Symbol::qualified("core", "run-tests"),
        vec![subject],
    );
    let reports = expr(&mut cx, &reports);
    let reports = list_items(&reports);
    assert_eq!(reports.len(), 2);
    assert!(reports.iter().all(|report| {
        !list_items(table_value(report, &Symbol::new("events")).expect("events")).is_empty()
            && !matches!(
                table_value(report, &Symbol::new("effect")),
                Some(Expr::Nil) | None
            )
            && table_value(report, &Symbol::new("mode"))
                == Some(&Expr::Symbol(Symbol::qualified("test", "truthy")))
    }));

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let after = call(
        &mut cx,
        Symbol::qualified("core", "coverage"),
        vec![subject],
    );
    let after = expr(&mut cx, &after);
    assert_eq!(int_field(&after, "passed"), Some(1));
    assert_eq!(int_field(&after, "failed"), Some(1));
    assert_eq!(int_field(&after, "skipped"), Some(0));
    assert!(!matches!(
        table_value(&after, &Symbol::new("last-run")),
        Some(Expr::Nil) | None
    ));
}

#[test]
fn run_tests_skips_tests_with_missing_test_capabilities() {
    let mut cx = test_cx();
    cx.grant(browse_run_tests_capability());
    register(
        &mut cx,
        SimTest::new(
            Symbol::qualified("test", "needs-demo"),
            Symbol::qualified("test", "runtime"),
            Expr::Bool(true),
            TestExpected::Truthy,
            vec![Symbol::qualified("core", "help")],
        )
        .requiring(CapabilityName::new("demo")),
    );

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let reports = call(
        &mut cx,
        Symbol::qualified("core", "run-tests"),
        vec![subject],
    );
    let reports = expr(&mut cx, &reports);
    let reports = list_items(&reports);
    assert_eq!(reports.len(), 1);
    assert_eq!(
        table_value(&reports[0], &Symbol::new("passed")),
        Some(&Expr::Bool(false))
    );
    assert!(matches!(
        table_value(&reports[0], &Symbol::new("detail")),
        Some(Expr::String(detail)) if detail.contains("missing capability demo")
    ));

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let coverage = call(
        &mut cx,
        Symbol::qualified("core", "coverage"),
        vec![subject],
    );
    let coverage = expr(&mut cx, &coverage);
    assert_eq!(int_field(&coverage, "runnable"), Some(0));
    assert_eq!(int_field(&coverage, "skipped"), Some(1));
}

#[cfg(feature = "codec-lisp")]
fn install_lisp_codec(cx: &mut Cx) {
    let codec_id = cx.registry_mut().fresh_codec_id();
    let lib = crate::codec_lisp::LispCodecLib::new(codec_id).unwrap();
    cx.load_lib(&lib).unwrap();
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn register(cx: &mut Cx, test: SimTest) {
    cx.registry_mut()
        .register_test(
            test.name.clone(),
            test.lib.clone(),
            Arc::new(test.clone()),
            test.subjects.clone(),
        )
        .unwrap();
}

fn example_test() -> SimTest {
    SimTest::new(
        Symbol::qualified("test", "example"),
        Symbol::qualified("test", "runtime"),
        Expr::Bool(true),
        TestExpected::Truthy,
        vec![Symbol::qualified("core", "help")],
    )
    .as_example()
}

fn call(cx: &mut Cx, symbol: Symbol, args: Vec<Value>) -> Value {
    try_call(cx, symbol.clone(), args).unwrap_or_else(|err| panic!("{symbol} failed: {err}"))
}

fn try_call(cx: &mut Cx, symbol: Symbol, args: Vec<Value>) -> sim_kernel::Result<Value> {
    cx.call_function(&symbol, Args::new(args))
}

fn symbol_value(cx: &Cx, symbol: Symbol) -> Value {
    cx.factory().symbol(symbol).unwrap()
}

fn expr(cx: &mut Cx, value: &Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

fn table_keys(expr: &Expr) -> Vec<Symbol> {
    let Expr::Map(entries) = expr else {
        panic!("expected map");
    };
    entries
        .iter()
        .map(|(key, _)| match key {
            Expr::Symbol(symbol) => symbol.clone(),
            other => panic!("expected symbol key, got {other:?}"),
        })
        .collect()
}

fn symbols(fields: &[&str]) -> Vec<Symbol> {
    fields.iter().map(|field| Symbol::new(*field)).collect()
}

fn list_items(expr: &Expr) -> &[Expr] {
    let Expr::List(items) = expr else {
        panic!("expected list");
    };
    items
}

fn test_by_name(tests: &[Expr], expected: Symbol) -> &Expr {
    tests
        .iter()
        .find(|test| {
            table_value(test, &Symbol::new("name")) == Some(&Expr::Symbol(expected.clone()))
        })
        .unwrap_or_else(|| panic!("missing test {expected}"))
}

fn int_field(expr: &Expr, name: &str) -> Option<i64> {
    match table_value(expr, &Symbol::new(name)) {
        Some(Expr::Number(number)) => number.canonical.parse().ok(),
        _ => None,
    }
}

fn assert_list_contains_symbol(expr: &Expr, expected: Symbol) {
    assert!(
        list_items(expr)
            .iter()
            .any(|item| matches!(item, Expr::Symbol(symbol) if symbol == &expected))
    );
}
