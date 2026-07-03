use std::sync::Arc;

use sim_kernel::{
    Args, ContentId, Cx, Datum, DatumStore, DefaultFactory, Expr, NoopEvalPolicy, Symbol, Value,
};

use crate::runtime::{
    browse::schema::{CARD_V2_FIELDS, HELP_FIELDS},
    install_core_runtime,
};

use super::support::table_value;

#[test]
fn root_browse_catalog_reaches_core_help_through_registry_surfaces() {
    let mut cx = test_cx();
    let root = call(&mut cx, Symbol::new("browse"), Vec::new());
    let root_expr = expr(&mut cx, &root);

    assert_eq!(table_keys(&root_expr), symbols(CARD_V2_FIELDS));
    assert_eq!(
        table_value(&root_expr, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("browse", "catalog")))
    );
    let see_also = table_value(&root_expr, &Symbol::new("see-also")).expect("see-also");
    assert_list_contains_symbol(see_also, Symbol::qualified("core", "functions"));
    assert_list_contains_symbol(see_also, Symbol::qualified("core", "tests"));

    let functions = call(&mut cx, Symbol::qualified("core", "functions"), Vec::new());
    let Expr::List(entries) = expr(&mut cx, &functions) else {
        panic!("core/functions should return a list");
    };
    assert!(entries.iter().any(|entry| {
        table_value(entry, &Symbol::new("subject"))
            == Some(&Expr::Symbol(Symbol::qualified("core", "help")))
    }));
}

#[test]
fn browse_subject_returns_card_v2_with_fixed_help_object() {
    let mut cx = test_cx();
    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let card = call(&mut cx, Symbol::qualified("core", "browse"), vec![subject]);
    let card_expr = expr(&mut cx, &card);

    assert_eq!(table_keys(&card_expr), symbols(CARD_V2_FIELDS));
    assert_eq!(
        table_value(&card_expr, &Symbol::new("subject")),
        Some(&Expr::Symbol(Symbol::qualified("core", "help")))
    );
    assert_eq!(
        table_value(&card_expr, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "function")))
    );

    let help = table_value(&card_expr, &Symbol::new("help")).expect("help");
    assert_eq!(table_keys(help), symbols(HELP_FIELDS));
    assert_eq!(
        table_value(help, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "function")))
    );

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let projected = call(
        &mut cx,
        Symbol::qualified("core", "help-object"),
        vec![subject],
    );
    assert_eq!(expr(&mut cx, &projected), *help);
}

#[test]
fn projection_functions_match_card_fields() {
    let mut cx = test_cx();
    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let card = call(&mut cx, Symbol::qualified("core", "browse"), vec![subject]);
    let card_expr = expr(&mut cx, &card);

    for (function, field) in [
        (Symbol::qualified("core", "args"), "args"),
        (Symbol::qualified("core", "result"), "result"),
        (Symbol::qualified("core", "tests"), "tests"),
        (Symbol::qualified("core", "coverage"), "coverage"),
        (Symbol::qualified("core", "facets"), "facets"),
    ] {
        let subject = string_value(&cx, "core/help");
        let projected = call(&mut cx, function, vec![subject]);
        assert_eq!(
            expr(&mut cx, &projected),
            *table_value(&card_expr, &Symbol::new(field)).expect(field)
        );
    }

    let subject = symbol_value(&cx, Symbol::qualified("core", "help"));
    let examples = call(
        &mut cx,
        Symbol::qualified("core", "examples"),
        vec![subject],
    );
    assert_eq!(expr(&mut cx, &examples), Expr::List(Vec::new()));
}

#[test]
fn legacy_tests_surface_keeps_zero_arg_arity() {
    let mut cx = test_cx();
    let all_tests = call(&mut cx, Symbol::qualified("core", "tests"), Vec::new());

    assert!(matches!(expr(&mut cx, &all_tests), Expr::List(_)));
}

#[test]
fn browse_accepts_content_ref_identity_expressions() {
    let mut cx = test_cx();
    let id = cx
        .datum_store_mut()
        .intern(Datum::String("content subject".to_owned()))
        .unwrap();
    let subject_expr = content_ref_expr(&id);
    let subject = cx.factory().expr(subject_expr.clone()).unwrap();
    let card = call(&mut cx, Symbol::qualified("core", "browse"), vec![subject]);
    let card_expr = expr(&mut cx, &card);

    assert_eq!(
        table_value(&card_expr, &Symbol::new("subject")),
        Some(&subject_expr)
    );
    assert_eq!(
        table_value(&card_expr, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("browse", "content")))
    );
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn call(cx: &mut Cx, symbol: Symbol, args: Vec<Value>) -> Value {
    cx.call_function(&symbol, Args::new(args))
        .unwrap_or_else(|err| panic!("{symbol} failed: {err}"))
}

fn symbol_value(cx: &Cx, symbol: Symbol) -> Value {
    cx.factory().symbol(symbol).unwrap()
}

fn string_value(cx: &Cx, value: &str) -> Value {
    cx.factory().string(value.to_owned()).unwrap()
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

fn assert_list_contains_symbol(expr: &Expr, expected: Symbol) {
    let Expr::List(items) = expr else {
        panic!("expected list");
    };
    assert!(
        items
            .iter()
            .any(|item| matches!(item, Expr::Symbol(symbol) if symbol == &expected))
    );
}

fn content_ref_expr(id: &ContentId) -> Expr {
    Expr::Extension {
        tag: Symbol::qualified("core", "ref"),
        payload: Box::new(Expr::Map(vec![
            (
                Expr::Symbol(Symbol::new("kind")),
                Expr::Symbol(Symbol::qualified("core", "content")),
            ),
            (
                Expr::Symbol(Symbol::new("algorithm")),
                Expr::Symbol(id.algorithm.clone()),
            ),
            (
                Expr::Symbol(Symbol::new("bytes")),
                Expr::Bytes(id.bytes.to_vec()),
            ),
        ])),
    }
}
