use std::{collections::BTreeSet, sync::Arc};

use sim_kernel::{
    Args, Cx, DefaultFactory, EagerPolicy, Error, Expr, Symbol, Value, browse_internal_capability,
    browse_run_tests_capability,
};

use crate::runtime::{
    SimTest, TestExpected,
    browse::schema::{
        BROWSE_TEST_FIELDS, CARD_V2_FIELDS, COVERAGE_FIELDS, FACET_FIELDS, HELP_FIELDS,
        REDACTION_FIELDS, TEST_REPORT_FIELDS,
    },
    install_core_runtime,
};

use super::support::table_value;

#[test]
fn every_registered_browse_subject_yields_schema_valid_card_v2() {
    let mut cx = conformance_cx();
    let subjects = collect_registered_subjects(&mut cx);
    assert!(
        !subjects.is_empty(),
        "conformance subject set should not be empty"
    );

    for subject in subjects {
        let card = browse_symbol(&mut cx, subject.clone());
        let card_expr = expr(&mut cx, &card);
        assert_card_v2(&card_expr, &subject.to_string());
        assert_shape_accepts(&mut cx, Symbol::qualified("core", "Card"), card);
    }
}

#[test]
fn every_visible_card_edge_is_browsable() {
    let mut cx = conformance_cx();
    let subjects = collect_registered_subjects(&mut cx);

    for subject in subjects {
        let subject_value = symbol_value(&cx, subject.clone());
        let neighbors = try_call(
            &mut cx,
            Symbol::qualified("core", "browse-neighbors"),
            vec![subject_value],
        )
        .unwrap_or_else(|err| panic!("browse-neighbors for {subject} failed: {err}"));
        for edge in list_items(&expr(&mut cx, &neighbors)) {
            let value = value_from_expr(&cx, edge);
            let card = call(&mut cx, Symbol::qualified("core", "browse"), vec![value]);
            let card_expr = expr(&mut cx, &card);
            assert_card_v2(&card_expr, &format!("edge {edge:?} from {subject}"));
        }
    }
}

#[test]
fn capability_redaction_and_run_tests_gates_fail_closed() {
    let mut public = conformance_cx();
    register_truthy_test(&mut public);
    let server = browse_symbol(&mut public, Symbol::qualified("server", "server"));
    let server = expr(&mut public, &server);
    let metrics = facet_by_name(&server, Symbol::qualified("server", "metrics"));
    assert_redaction(table_value(metrics, &field("value")).expect("redaction"));

    let subject = symbol_value(&public, Symbol::qualified("core", "help"));
    let descriptions = call(
        &mut public,
        Symbol::qualified("core", "tests"),
        vec![subject.clone()],
    );
    assert!(!list_items(&expr(&mut public, &descriptions)).is_empty());
    let denied = try_call(
        &mut public,
        Symbol::qualified("core", "run-tests"),
        vec![subject],
    )
    .unwrap_err();
    assert!(matches!(
        denied,
        Error::CapabilityDenied { capability } if capability == browse_run_tests_capability()
    ));

    let mut internal = conformance_cx();
    internal.grant(browse_internal_capability());
    let server = browse_symbol(&mut internal, Symbol::qualified("server", "server"));
    let server = expr(&mut internal, &server);
    let metrics = facet_by_name(&server, Symbol::qualified("server", "metrics"));
    assert!(!is_redaction(
        table_value(metrics, &field("value")).expect("metrics")
    ));

    internal.grant(browse_run_tests_capability());
    register_truthy_test(&mut internal);
    let subject = symbol_value(&internal, Symbol::qualified("core", "help"));
    let reports = call(
        &mut internal,
        Symbol::qualified("core", "run-tests"),
        vec![subject],
    );
    for report in list_items(&expr(&mut internal, &reports)) {
        assert_eq!(
            table_keys(report),
            symbols(TEST_REPORT_FIELDS),
            "TestReport field order"
        );
    }
}

#[test]
fn root_graph_reaches_core_schema_codec_shape_and_test_subjects() {
    let mut cx = conformance_cx();
    assert_path_reaches(&mut cx, Symbol::qualified("core", "help"));
    assert_path_reaches(&mut cx, Symbol::qualified("core", "Card"));
    assert_path_reaches(&mut cx, Symbol::qualified("browse", "Help"));
    assert_path_reaches(&mut cx, Symbol::qualified("browse-example", "core-card"));

    #[cfg(feature = "codec-lisp")]
    assert_path_reaches(&mut cx, Symbol::qualified("codec", "lisp"));
}

fn conformance_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    install_enabled_codecs(&mut cx);
    cx
}

fn install_enabled_codecs(cx: &mut Cx) {
    #[cfg(feature = "codec-lisp")]
    {
        let symbol = Symbol::qualified("codec", "lisp");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib =
                crate::codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
            cx.load_lib(&lib).unwrap();
        }
    }

    #[cfg(feature = "codec-json")]
    {
        let symbol = Symbol::qualified("codec", "json");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib = crate::codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
            cx.load_lib(&lib).unwrap();
        }
    }

    #[cfg(feature = "codec-binary")]
    {
        let symbol = Symbol::qualified("codec", "binary");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib = crate::codec_binary::BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
            cx.load_lib(&lib).unwrap();
        }
    }

    #[cfg(feature = "codec-binary-base64")]
    {
        let symbol = Symbol::qualified("codec", "binary-base64");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib = crate::codec_binary_base64::BinaryBase64CodecLib::new(
                cx.registry_mut().fresh_codec_id(),
            );
            cx.load_lib(&lib).unwrap();
        }
    }

    #[cfg(feature = "codec-algol")]
    {
        let symbol = Symbol::qualified("codec", "algol");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib = crate::codec_algol::AlgolCodecLib::new(cx.registry_mut().fresh_codec_id());
            cx.load_lib(&lib).unwrap();
        }
    }
}

fn collect_registered_subjects(cx: &mut Cx) -> BTreeSet<Symbol> {
    let mut subjects = BTreeSet::new();
    subjects.insert(Symbol::qualified("browse", "catalog"));
    subjects.extend(
        cx.registry()
            .libs()
            .iter()
            .map(|lib| lib.manifest.id.clone()),
    );
    subjects.extend(cx.registry().classes().keys().cloned());
    subjects.extend(cx.registry().functions().keys().cloned());
    subjects.extend(cx.registry().macros().keys().cloned());
    subjects.extend(cx.registry().shapes().keys().cloned());
    subjects.extend(cx.registry().codecs().keys().cloned());
    subjects.extend(cx.registry().number_domains().keys().cloned());
    subjects.extend(cx.registry().tests().keys().cloned());

    let root_neighbors = call(
        cx,
        Symbol::qualified("core", "browse-neighbors"),
        Vec::new(),
    );
    add_symbols_from_list(cx, root_neighbors, &mut subjects);

    for surface in [
        Symbol::qualified("core", "libs"),
        Symbol::qualified("core", "classes"),
        Symbol::qualified("core", "functions"),
        Symbol::qualified("core", "macros"),
        Symbol::qualified("core", "shapes"),
        Symbol::qualified("core", "codecs"),
        Symbol::qualified("core", "number-domains"),
        Symbol::qualified("core", "eval-policies"),
        Symbol::qualified("core", "tests"),
    ] {
        let value = call(cx, surface, Vec::new());
        add_registry_surface_subjects(cx, value, &mut subjects);
    }

    subjects
}

fn add_symbols_from_list(cx: &mut Cx, value: Value, subjects: &mut BTreeSet<Symbol>) {
    for item in list_items(&expr(cx, &value)) {
        if let Expr::Symbol(symbol) = item {
            subjects.insert(symbol.clone());
        }
    }
}

fn add_registry_surface_subjects(cx: &mut Cx, value: Value, subjects: &mut BTreeSet<Symbol>) {
    for item in list_items(&expr(cx, &value)) {
        for field_name in ["subject", "id", "name"] {
            if let Some(Expr::Symbol(symbol)) = table_value(item, &field(field_name)) {
                subjects.insert(symbol.clone());
            }
        }
    }
}

fn assert_card_v2(card: &Expr, label: &str) {
    assert_eq!(table_keys(card), symbols(CARD_V2_FIELDS), "{label}");
    assert_exact_table(
        table_value(card, &field("help")).expect("help"),
        HELP_FIELDS,
        "Help",
    );
    assert_non_nil(
        table_value(card, &field("args")).expect("args"),
        "args",
        label,
    );
    assert_non_nil(
        table_value(card, &field("result")).expect("result"),
        "result",
        label,
    );
    assert_list_field(card, "tests", label);
    assert_list_field(card, "ops", label);
    assert_list_field(card, "requires", label);
    assert_list_field(card, "see-also", label);
    assert_list_field(card, "facets", label);
    assert_list_field(card, "provenance", label);
    assert_eq!(
        table_keys(table_value(card, &field("coverage")).expect("coverage")),
        symbols(COVERAGE_FIELDS),
        "{label} Coverage field order"
    );
    assert!(matches!(
        table_value(card, &field("shape-known")),
        Some(Expr::Bool(_))
    ));
    assert!(matches!(
        table_value(card, &field("freshness")),
        Some(Expr::Symbol(symbol))
            if matches!(symbol.name.as_ref(), "unknown" | "fresh" | "stale" | "live")
    ));

    for test in list_items(table_value(card, &field("tests")).expect("tests")) {
        assert_exact_table(test, BROWSE_TEST_FIELDS, "Test");
    }
    for facet in list_items(table_value(card, &field("facets")).expect("facets")) {
        assert_exact_table(facet, FACET_FIELDS, "Facet");
        assert!(matches!(
            table_value(facet, &field("requires")),
            Some(Expr::List(_))
        ));
        assert!(matches!(
            table_value(facet, &field("evidence")),
            Some(Expr::List(_))
        ));
        if let Some(value) = table_value(facet, &field("value"))
            && is_redaction(value)
        {
            assert_redaction(value);
        }
    }
}

fn assert_shape_accepts(cx: &mut Cx, shape: Symbol, value: Value) {
    let shape = cx.resolve_shape(&shape).unwrap();
    let report = shape
        .object()
        .as_shape()
        .unwrap()
        .check_value(cx, value)
        .unwrap();
    assert!(report.accepted, "{:?}", report.diagnostics);
}

fn assert_path_reaches(cx: &mut Cx, target: Symbol) {
    let root = symbol_value(cx, Symbol::qualified("browse", "catalog"));
    let target_value = symbol_value(cx, target.clone());
    let path = call(
        cx,
        Symbol::qualified("core", "browse-path"),
        vec![root, target_value],
    );
    let path = expr(cx, &path);
    let Expr::List(items) = path else {
        panic!("path to {target} should be visible");
    };
    assert_eq!(items.last(), Some(&Expr::Symbol(target)));
}

fn register_truthy_test(cx: &mut Cx) {
    let test = SimTest::new(
        Symbol::qualified("test", "conformance-truthy"),
        Symbol::qualified("test", "runtime"),
        Expr::Bool(true),
        TestExpected::Truthy,
        vec![Symbol::qualified("core", "help")],
    );
    cx.registry_mut()
        .register_test(
            Symbol::qualified("test", "conformance-truthy"),
            Symbol::qualified("test", "runtime"),
            Arc::new(test),
            vec![Symbol::qualified("core", "help")],
        )
        .unwrap();
}

fn browse_symbol(cx: &mut Cx, symbol: Symbol) -> Value {
    let subject = symbol_value(cx, symbol);
    call(cx, Symbol::qualified("core", "browse"), vec![subject])
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

fn value_from_expr(cx: &Cx, expr: &Expr) -> Value {
    match expr {
        Expr::Symbol(symbol) => cx.factory().symbol(symbol.clone()).unwrap(),
        _ => cx.factory().expr(expr.clone()).unwrap(),
    }
}

fn expr(cx: &mut Cx, value: &Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

fn table_keys(expr: &Expr) -> Vec<Symbol> {
    let Expr::Map(entries) = expr else {
        panic!("expected table, got {expr:?}");
    };
    entries
        .iter()
        .map(|(key, _)| match key {
            Expr::Symbol(symbol) => symbol.clone(),
            other => panic!("expected symbol table key, got {other:?}"),
        })
        .collect()
}

fn assert_exact_table(expr: &Expr, fields: &[&str], label: &str) {
    assert_eq!(table_keys(expr), symbols(fields), "{label} field order");
}

fn assert_non_nil(expr: &Expr, field_name: &str, label: &str) {
    assert!(
        !matches!(expr, Expr::Nil),
        "{label} field {field_name} should be total"
    );
}

fn assert_list_field(card: &Expr, field_name: &str, label: &str) {
    assert!(
        matches!(table_value(card, &field(field_name)), Some(Expr::List(_))),
        "{label} field {field_name} should be a list"
    );
}

fn facet_by_name(card: &Expr, name: Symbol) -> &Expr {
    list_items(table_value(card, &field("facets")).expect("facets"))
        .iter()
        .find(|facet| table_value(facet, &field("name")) == Some(&Expr::Symbol(name.clone())))
        .unwrap_or_else(|| panic!("missing facet {name}"))
}

fn is_redaction(expr: &Expr) -> bool {
    matches!(expr, Expr::Map(_))
        && table_value(expr, &field("reason")).is_some()
        && table_value(expr, &field("requires")).is_some()
        && table_value(expr, &field("summary")).is_some()
}

fn assert_redaction(expr: &Expr) {
    assert_exact_table(expr, REDACTION_FIELDS, "Redaction");
    assert!(matches!(
        table_value(expr, &field("requires")),
        Some(Expr::List(_))
    ));
}

fn list_items(expr: &Expr) -> &[Expr] {
    let Expr::List(items) = expr else {
        panic!("expected list, got {expr:?}");
    };
    items
}

fn field(name: &str) -> Symbol {
    Symbol::new(name.to_owned())
}

fn symbols(fields: &[&str]) -> Vec<Symbol> {
    fields.iter().map(|field| Symbol::new(*field)).collect()
}
