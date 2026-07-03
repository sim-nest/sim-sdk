use std::{collections::BTreeSet, sync::Arc};

use sim_kernel::{Args, Cx, DefaultFactory, Expr, NoopEvalPolicy, Symbol, Value};

use crate::runtime::install_core_runtime;

use super::support::table_value;

#[test]
fn browse_neighbors_extracts_card_edges_and_leaf_cards() {
    let mut cx = test_cx();
    let core_help = symbol_value(&cx, Symbol::qualified("core", "help"));
    let neighbors = call(
        &mut cx,
        Symbol::qualified("core", "browse-neighbors"),
        vec![core_help],
    );
    let symbols = symbol_set(&expr(&mut cx, &neighbors));

    assert!(symbols.contains(&Symbol::qualified("core", "help")));
    assert!(symbols.contains(&Symbol::qualified("core/help", "args")));
    assert!(symbols.contains(&Symbol::qualified("core/help", "result")));
    assert!(symbols.contains(&Symbol::new("core")));
    assert!(symbols.contains(&Symbol::qualified("browse", "Help")));
    assert!(symbols.contains(&Symbol::qualified("capability", "browse.internal")));
    assert!(symbols.contains(&Symbol::qualified("core", "call.v1")));

    assert_card_kind(
        &mut cx,
        Symbol::qualified("capability", "browse.internal"),
        Symbol::qualified("browse", "capability"),
    );
    assert_card_kind(
        &mut cx,
        Symbol::qualified("core", "call.v1"),
        Symbol::qualified("browse", "op"),
    );
    assert_card_kind(
        &mut cx,
        Symbol::qualified("core/help", "args"),
        Symbol::qualified("core", "shape"),
    );
}

#[test]
fn browse_path_returns_ref_list_or_nil() {
    let mut cx = test_cx();
    let core_help = symbol_value(&cx, Symbol::qualified("core", "help"));
    let core_card = symbol_value(&cx, Symbol::qualified("core", "Card"));
    let path = call(
        &mut cx,
        Symbol::qualified("core", "browse-path"),
        vec![core_help, core_card],
    );
    let path = expr(&mut cx, &path);
    let Expr::List(items) = &path else {
        panic!("reachable path should be a list");
    };
    assert_eq!(
        items.first(),
        Some(&Expr::Symbol(Symbol::qualified("core", "help")))
    );
    assert_eq!(
        items.last(),
        Some(&Expr::Symbol(Symbol::qualified("core", "Card")))
    );
    assert!(items.iter().all(is_ref_expr));

    let core_help = symbol_value(&cx, Symbol::qualified("core", "help"));
    let missing_target = symbol_value(&cx, Symbol::qualified("external", "missing"));
    let missing = call(
        &mut cx,
        Symbol::qualified("core", "browse-path"),
        vec![core_help, missing_target],
    );
    assert_eq!(expr(&mut cx, &missing), Expr::Nil);
}

#[cfg(feature = "codec-lisp")]
#[test]
fn root_graph_path_reaches_function_shape_codec_test_and_capability() {
    let mut cx = test_cx();
    let codec_id = cx.registry_mut().fresh_codec_id();
    let lib = crate::codec_lisp::LispCodecLib::new(codec_id).unwrap();
    cx.load_lib(&lib).unwrap();

    for (target, kind) in [
        (
            Symbol::qualified("core", "help"),
            Symbol::qualified("core", "function"),
        ),
        (
            Symbol::qualified("browse", "Help"),
            Symbol::qualified("core", "shape"),
        ),
        (
            Symbol::qualified("codec", "lisp"),
            Symbol::qualified("core", "codec"),
        ),
        (
            Symbol::qualified("browse-example", "core-card"),
            Symbol::qualified("core", "test"),
        ),
        (
            Symbol::qualified("capability", "browse.internal"),
            Symbol::qualified("browse", "capability"),
        ),
    ] {
        let root = symbol_value(&cx, Symbol::qualified("browse", "catalog"));
        let target_value = symbol_value(&cx, target.clone());
        let path = call(
            &mut cx,
            Symbol::qualified("core", "browse-path"),
            vec![root, target_value],
        );
        assert_path_reaches(&mut cx, path, &target);
        assert_card_kind(&mut cx, target, kind);
    }
}

#[test]
fn zero_arg_browse_neighbors_starts_at_root_catalog() {
    let mut cx = test_cx();
    let neighbors = call(
        &mut cx,
        Symbol::qualified("core", "browse-neighbors"),
        Vec::new(),
    );
    let symbols = symbol_set(&expr(&mut cx, &neighbors));

    assert!(symbols.contains(&Symbol::qualified("core", "functions")));
    assert!(symbols.contains(&Symbol::qualified("core", "shapes")));
    assert!(symbols.contains(&Symbol::qualified("core", "tests")));
    assert!(symbols.contains(&Symbol::qualified("capability", "browse.internal")));
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

fn expr(cx: &mut Cx, value: &Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

fn symbol_set(expr: &Expr) -> BTreeSet<Symbol> {
    let Expr::List(items) = expr else {
        panic!("expected list");
    };
    items
        .iter()
        .filter_map(|item| match item {
            Expr::Symbol(symbol) => Some(symbol.clone()),
            _ => None,
        })
        .collect()
}

fn assert_card_kind(cx: &mut Cx, subject: Symbol, expected: Symbol) {
    let subject_value = symbol_value(cx, subject.clone());
    let card = call(cx, Symbol::qualified("core", "browse"), vec![subject_value]);
    let card = expr(cx, &card);
    assert_eq!(
        table_value(&card, &Symbol::new("kind")),
        Some(&Expr::Symbol(expected)),
        "{subject} should browse with the expected kind"
    );
}

fn assert_path_reaches(cx: &mut Cx, path: Value, target: &Symbol) {
    let path = expr(cx, &path);
    let Expr::List(items) = path else {
        panic!("path to {target} should be a list");
    };
    assert_eq!(items.last(), Some(&Expr::Symbol(target.clone())));
    assert!(items.iter().all(is_ref_expr));
}

fn is_ref_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Symbol(_) => true,
        Expr::Extension { tag, .. } => tag == &Symbol::qualified("core", "ref"),
        _ => false,
    }
}
