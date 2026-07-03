use std::sync::Arc;

use sim_kernel::{Args, Cx, DefaultFactory, Expr, NoopEvalPolicy, Symbol, Value};

use crate::runtime::install_core_runtime;

use super::support::table_value;

const SEEDED_LISP_RECIPE: &str = "codec/lisp/01-basics/quote-symbol";

#[test]
fn cookbook_help_lists_seeded_recipe() {
    let mut cx = test_cx();
    install_lisp_codec(&mut cx);

    let subject = symbol_value(&cx, Symbol::qualified("codec", "lisp"));
    let help = call(&mut cx, Symbol::qualified("core", "help"), vec![subject]);
    let help = expr(&mut cx, &help);

    assert!(matches!(
        table_value(&help, &field("detail")),
        Some(Expr::String(detail))
            if detail.contains("Recipes:") && detail.contains(SEEDED_LISP_RECIPE)
    ));
    assert_list_contains_symbol(
        table_value(&help, &field("see-also")).expect("see-also"),
        recipe_symbol(),
    );
}

#[test]
fn cookbook_browse_tree_includes_book_chapter_and_recipe_nodes() {
    let mut cx = test_cx();

    let root = call(&mut cx, Symbol::qualified("core", "browse"), Vec::new());
    assert_list_contains_symbol(
        table_value(&expr(&mut cx, &root), &field("see-also")).expect("root see-also"),
        Symbol::qualified("cookbook", "catalog"),
    );

    let catalog = browse_symbol(&mut cx, Symbol::qualified("cookbook", "catalog"));
    assert_eq!(
        table_value(&catalog, &field("kind")),
        Some(&Expr::Symbol(Symbol::qualified("cookbook", "catalog")))
    );
    assert_list_contains_symbol(
        table_value(&catalog, &field("see-also")).expect("catalog see-also"),
        Symbol::qualified("cookbook/book", "codec/lisp"),
    );

    let book = browse_symbol(&mut cx, Symbol::qualified("cookbook/book", "codec/lisp"));
    assert_eq!(
        table_value(&book, &field("kind")),
        Some(&Expr::Symbol(Symbol::qualified("cookbook", "book")))
    );
    assert_list_contains_symbol(
        table_value(&book, &field("see-also")).expect("book see-also"),
        Symbol::qualified("cookbook/chapter", "codec/lisp/01-basics"),
    );

    let chapter = browse_symbol(
        &mut cx,
        Symbol::qualified("cookbook/chapter", "codec/lisp/01-basics"),
    );
    assert_eq!(
        table_value(&chapter, &field("kind")),
        Some(&Expr::Symbol(Symbol::qualified("cookbook", "chapter")))
    );
    assert_list_contains_symbol(
        table_value(&chapter, &field("see-also")).expect("chapter see-also"),
        recipe_symbol(),
    );

    let recipe = browse_symbol(&mut cx, recipe_symbol());
    assert_eq!(
        table_value(&recipe, &field("kind")),
        Some(&Expr::Symbol(Symbol::qualified("cookbook", "recipe")))
    );
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn install_lisp_codec(cx: &mut Cx) {
    let codec_id = cx.registry_mut().fresh_codec_id();
    let lisp = crate::codec_lisp::LispCodecLib::new(codec_id).unwrap();
    cx.load_lib(&lisp).unwrap();
}

fn browse_symbol(cx: &mut Cx, symbol: Symbol) -> Expr {
    let subject = symbol_value(cx, symbol);
    let card = call(cx, Symbol::qualified("core", "browse"), vec![subject]);
    expr(cx, &card)
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

fn recipe_symbol() -> Symbol {
    Symbol::qualified("cookbook/recipe", SEEDED_LISP_RECIPE)
}

fn field(name: &str) -> Symbol {
    Symbol::new(name)
}

fn assert_list_contains_symbol(expr: &Expr, expected: Symbol) {
    let Expr::List(items) = expr else {
        panic!("expected symbol list");
    };
    assert!(
        items
            .iter()
            .any(|item| matches!(item, Expr::Symbol(symbol) if symbol == &expected)),
        "{expected} missing from {items:?}"
    );
}
