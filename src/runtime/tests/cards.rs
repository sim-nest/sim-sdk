use std::{fs, path::Path, sync::Arc};

use sim_kernel::{
    Args, ClaimPattern, Datum, DatumStore, DefaultFactory, EagerPolicy, Expr, NoopEvalPolicy, Ref,
    Symbol, Value, card::Card, force_list_to_vec,
};

use crate::runtime::{SimTest, TestExpected, install_core_runtime};

use super::support::table_value;

#[test]
fn core_runtime_registers_card_class() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let card = Symbol::qualified("core", "Card");
    assert!(cx.registry().class_by_symbol(&card).is_some());
}

#[test]
fn registry_browse_surfaces_return_cards_for_functions() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let functions = cx
        .call_function(
            &Symbol::qualified("core", "functions"),
            Args::new(Vec::new()),
        )
        .unwrap();
    let functions_expr = functions.object().as_expr(&mut cx).unwrap();
    let Expr::List(entries) = functions_expr else {
        panic!("expected functions list");
    };
    let help = entries
        .iter()
        .find(|entry| {
            table_value(entry, &Symbol::new("subject"))
                == Some(&Expr::Symbol(Symbol::qualified("core", "help")))
        })
        .expect("core/help card");

    assert_eq!(
        table_value(help, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "function")))
    );
    assert_eq!(
        table_value(help, &Symbol::new("args")),
        Some(&Expr::Symbol(Symbol::qualified("core/help", "args")))
    );
    assert_eq!(
        table_value(help, &Symbol::new("result")),
        Some(&Expr::Symbol(Symbol::qualified("core/help", "result")))
    );
    assert_eq!(
        table_value(help, &Symbol::new("shape-known")),
        Some(&Expr::Bool(true))
    );
    assert_eq!(
        table_value(help, &Symbol::new("symbol")),
        Some(&Expr::String("core/help".to_owned()))
    );
}

#[test]
fn registry_browse_list_surfaces_return_card_objects() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);

    for surface in [
        Symbol::qualified("core", "classes"),
        Symbol::qualified("core", "functions"),
        Symbol::qualified("core", "shapes"),
    ] {
        assert_card_list(&mut cx, surface);
    }
}

#[test]
fn object_card_hook_is_absent_from_rust_sources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let needle = ["as", "_card"].concat();

    for dir in ["crates", "src"] {
        assert_no_source_contains(&root.join(dir), &needle);
    }
}

#[test]
fn loaded_exports_publish_registry_claims() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let subject = Symbol::qualified("core", "help");

    assert_has_claim(
        &cx,
        subject.clone(),
        Symbol::qualified("core", "kind"),
        Ref::Symbol(Symbol::qualified("core", "function")),
    );
    assert_has_claim(
        &cx,
        subject.clone(),
        Symbol::qualified("core", "exported-by"),
        Ref::Symbol(Symbol::new("core")),
    );
    assert_has_claim(
        &cx,
        subject.clone(),
        Symbol::qualified("core", "args"),
        Ref::Symbol(Symbol::qualified("core/help", "args")),
    );
    assert_has_claim(
        &cx,
        subject.clone(),
        Symbol::qualified("core", "result"),
        Ref::Symbol(Symbol::qualified("core/help", "result")),
    );
    assert_has_bool_claim(
        &cx,
        subject.clone(),
        Symbol::qualified("core", "shape-known"),
        true,
    );
    assert_has_claim(
        &cx,
        Symbol::qualified("core", "lambda"),
        Symbol::qualified("core", "args"),
        Ref::Symbol(Symbol::qualified("core", "Any")),
    );
    assert_has_claim(
        &cx,
        Symbol::qualified("core", "lambda"),
        Symbol::qualified("core", "result"),
        Ref::Symbol(Symbol::qualified("core", "Any")),
    );
    assert_has_bool_claim(
        &cx,
        Symbol::qualified("core", "lambda"),
        Symbol::qualified("core", "shape-known"),
        false,
    );

    let ops = claim_strings(&cx, subject, Symbol::qualified("core", "ops"));
    assert!(ops.iter().any(|op| op == "core/call.v1"));
}

#[test]
fn registry_browse_surfaces_fall_back_to_map_entries_without_claims() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let symbol = Symbol::qualified("test", "map-only");
    let value = cx
        .factory()
        .string("legacy function value".to_owned())
        .unwrap();
    cx.registry_mut()
        .register_function_value(symbol.clone(), value)
        .unwrap();

    let claims = cx
        .query_facts(ClaimPattern {
            subject: Some(Ref::Symbol(symbol.clone())),
            predicate: Some(Symbol::qualified("core", "kind")),
            object: None,
            include_revoked: false,
        })
        .unwrap();
    assert!(claims.is_empty());

    let functions = cx
        .call_function(
            &Symbol::qualified("core", "functions"),
            Args::new(Vec::new()),
        )
        .unwrap();
    let functions_expr = functions.object().as_expr(&mut cx).unwrap();
    let Expr::List(entries) = functions_expr else {
        panic!("expected functions list");
    };
    assert!(entries.iter().any(|entry| {
        table_value(entry, &Symbol::new("subject")) == Some(&Expr::Symbol(symbol.clone()))
    }));
}

#[test]
fn registry_browse_surfaces_return_cards_for_shapes_and_codecs() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let shapes = cx
        .call_function(&Symbol::qualified("core", "shapes"), Args::new(Vec::new()))
        .unwrap();
    let shapes_expr = shapes.object().as_expr(&mut cx).unwrap();
    let Expr::List(shape_entries) = shapes_expr else {
        panic!("expected shapes list");
    };
    let any = shape_entries
        .iter()
        .find(|entry| {
            table_value(entry, &Symbol::new("subject"))
                == Some(&Expr::Symbol(Symbol::qualified("core", "Any")))
        })
        .expect("core/Any card");

    assert_eq!(
        table_value(any, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "shape")))
    );

    let codecs = cx
        .call_function(&Symbol::qualified("core", "codecs"), Args::new(Vec::new()))
        .unwrap();
    let codecs_expr = codecs.object().as_expr(&mut cx).unwrap();
    let Expr::List(codec_entries) = codecs_expr else {
        panic!("expected codecs list");
    };
    if let Some(codec) = codec_entries.first() {
        assert_eq!(
            table_value(codec, &Symbol::new("kind")),
            Some(&Expr::Symbol(Symbol::qualified("core", "codec")))
        );
        assert!(table_value(codec, &Symbol::new("subject")).is_some());
    }
}

#[cfg(feature = "codec-lisp")]
#[test]
fn loaded_codec_exports_publish_registry_claims() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let codec_id = cx.registry_mut().fresh_codec_id();
    let lib = crate::codec_lisp::LispCodecLib::new(codec_id).unwrap();
    cx.load_lib(&lib).unwrap();
    let subject = Symbol::qualified("codec", "lisp");

    assert_has_claim(
        &cx,
        subject.clone(),
        Symbol::qualified("core", "kind"),
        Ref::Symbol(Symbol::qualified("core", "codec")),
    );
    assert_has_claim(
        &cx,
        subject,
        Symbol::qualified("core", "exported-by"),
        Ref::Symbol(Symbol::qualified("codec", "lisp")),
    );
}

#[test]
fn help_projection_publishes_authored_help_claims() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx.call_function(
        &Symbol::qualified("core", "help"),
        Args::new(vec![
            cx.factory()
                .symbol(Symbol::qualified("core", "help"))
                .unwrap(),
        ]),
    )
    .unwrap();

    let help = claim_strings(
        &cx,
        Symbol::qualified("core", "help"),
        Symbol::qualified("core", "help"),
    );
    let has_authored_help = help
        .iter()
        .any(|text| text.contains("returns and publishes"));
    assert!(has_authored_help);
}

#[test]
fn roundtrip_tests_are_card_visible_with_codec_ids() {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let test = SimTest::new(
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
    );
    cx.registry_mut()
        .register_test(
            Symbol::qualified("test", "roundtrip"),
            Symbol::qualified("test", "runtime"),
            Arc::new(test),
            vec![Symbol::qualified("core", "help")],
        )
        .unwrap();

    let tests = cx
        .call_function(&Symbol::qualified("core", "tests"), Args::new(Vec::new()))
        .unwrap();
    let tests_expr = tests.object().as_expr(&mut cx).unwrap();
    let Expr::List(tests) = tests_expr else {
        panic!("expected tests list");
    };
    let roundtrip = card_by_name(&tests, Symbol::qualified("test", "roundtrip"));
    let codecs = table_value(roundtrip, &Symbol::new("codecs")).expect("codec ids");

    assert_eq!(
        codecs,
        &Expr::List(vec![
            Expr::Symbol(Symbol::qualified("codec", "lisp")),
            Expr::Symbol(Symbol::qualified("codec", "json")),
        ])
    );
}

#[test]
fn card_v2_provenance_contains_shape_report_evidence() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let shape = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    let target = cx
        .resolve_function(&Symbol::qualified("core", "help"))
        .unwrap();
    let report = sim_kernel::shape_report::check_value_report(&mut cx, &shape, target).unwrap();
    assert!(report.accepted);

    let card = cx
        .call_function(
            &Symbol::qualified("core", "browse"),
            Args::new(vec![
                cx.factory()
                    .symbol(Symbol::qualified("core", "help"))
                    .unwrap(),
            ]),
        )
        .unwrap();
    let card_expr = card.object().as_expr(&mut cx).unwrap();
    let provenance = table_value(&card_expr, &Symbol::new("provenance")).expect("provenance");
    let Expr::List(items) = provenance else {
        panic!("provenance should be a list");
    };
    assert!(items.iter().any(is_shape_report_expr));
}

fn assert_has_claim(cx: &sim_kernel::Cx, subject: Symbol, predicate: Symbol, object: Ref) {
    let claims = cx
        .query_facts(ClaimPattern {
            subject: Some(Ref::Symbol(subject)),
            predicate: Some(predicate),
            object: Some(object),
            include_revoked: false,
        })
        .unwrap();
    assert_eq!(claims.len(), 1);
}

fn assert_has_bool_claim(cx: &sim_kernel::Cx, subject: Symbol, predicate: Symbol, expected: bool) {
    let claims = cx
        .query_facts(ClaimPattern {
            subject: Some(Ref::Symbol(subject)),
            predicate: Some(predicate),
            object: None,
            include_revoked: false,
        })
        .unwrap();
    assert!(claims.into_iter().any(|claim| {
        matches!(
            claim.object,
            Ref::Content(id)
                if matches!(cx.datum_store().get(&id).unwrap(), Some(Datum::Bool(value)) if *value == expected)
        )
    }));
}

fn claim_strings(cx: &sim_kernel::Cx, subject: Symbol, predicate: Symbol) -> Vec<String> {
    cx.query_facts(ClaimPattern {
        subject: Some(Ref::Symbol(subject)),
        predicate: Some(predicate),
        object: None,
        include_revoked: false,
    })
    .unwrap()
    .into_iter()
    .filter_map(|claim| match claim.object {
        Ref::Content(id) => match cx.datum_store().get(&id).unwrap() {
            Some(Datum::String(text)) => Some(text.clone()),
            _ => None,
        },
        _ => None,
    })
    .collect()
}

fn assert_card_list(cx: &mut sim_kernel::Cx, surface: Symbol) {
    let value = cx
        .call_function(&surface, Args::new(Vec::new()))
        .unwrap_or_else(|err| panic!("{surface} failed: {err}"));
    let Some(list) = value.object().as_list() else {
        panic!("{surface} should return a list");
    };
    let entries = force_list_to_vec(cx, list, "registry browse card list")
        .unwrap_or_else(|err| panic!("{surface} list failed: {err}"));

    assert!(!entries.is_empty(), "{surface} should not be empty");
    for entry in entries {
        assert_card_value(&surface, &entry);
    }
}

fn assert_card_value(surface: &Symbol, value: &Value) {
    assert!(
        value.object().downcast_ref::<Card>().is_some(),
        "{surface} returned a non-Card browse value"
    );
}

fn card_by_name(cards: &[Expr], expected: Symbol) -> &Expr {
    let expected_expr = Expr::Symbol(expected.clone());
    cards
        .iter()
        .find(|card| table_value(card, &Symbol::new("name")) == Some(&expected_expr))
        .unwrap_or_else(|| panic!("missing card {expected}"))
}

fn assert_no_source_contains(path: &Path, needle: &str) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap_or_else(|err| {
            panic!("failed to read source directory {}: {err}", path.display())
        }) {
            let entry = entry.unwrap_or_else(|err| {
                panic!(
                    "failed to read source directory entry {}: {err}",
                    path.display()
                )
            });
            assert_no_source_contains(&entry.path(), needle);
        }
        return;
    }

    if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
        return;
    }

    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read source file {}: {err}", path.display()));
    assert!(
        !source.contains(needle),
        "forbidden object card hook found in {}",
        path.display()
    );
}

fn is_shape_report_expr(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Extension { tag, .. } if tag == &Symbol::qualified("core", "ShapeReport")
    )
}
