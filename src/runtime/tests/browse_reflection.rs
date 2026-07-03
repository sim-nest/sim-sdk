use std::sync::Arc;

use sim_kernel::{
    Args, Cx, DefaultFactory, Expr, NoopEvalPolicy, Symbol, Value, force_list_to_vec,
    registry_catalog_read_capability,
};

use crate::runtime::{
    browse::schema::{BrowseTestBuilder, CARD_V2_FIELDS, HELP_FIELDS, HelpBuilder},
    install_core_runtime,
};

use super::support::table_value;

const SCHEMA_SUBJECTS: &[(&str, &str, &str)] = &[
    ("core", "Card", "Card"),
    ("browse", "Help", "help document"),
    ("browse", "Test", "test or worked example"),
    ("browse", "Coverage", "coverage summary"),
    ("browse", "Facet", "extension payload"),
    ("browse", "Redaction", "hidden but known"),
    ("browse", "TestReport", "test run"),
];

const BROWSE_FUNCTIONS: &[(&str, &str)] = &[
    ("core", "browse"),
    ("core", "help"),
    ("core", "args"),
    ("core", "result"),
    ("core", "tests"),
    ("core", "examples"),
    ("core", "coverage"),
    ("core", "facets"),
    ("core", "help-object"),
    ("core", "browse-neighbors"),
    ("core", "browse-path"),
];

#[test]
fn browse_schema_subjects_have_cards_authored_help_and_examples() {
    let mut cx = test_cx();

    for (namespace, name, summary_fragment) in SCHEMA_SUBJECTS {
        let subject = Symbol::qualified(*namespace, *name);
        let card = browse(&mut cx, subject.clone());
        let card = expr(&mut cx, &card);
        assert_eq!(table_keys(&card), symbols(CARD_V2_FIELDS));
        assert_eq!(
            table_value(&card, &field("subject")),
            Some(&Expr::Symbol(subject.clone()))
        );
        assert!(
            shape_accepts_card(&mut cx, &card),
            "{subject} Card should satisfy core/Card"
        );

        let help = table_value(&card, &field("help")).expect("help");
        assert_eq!(table_keys(help), symbols(HELP_FIELDS));
        assert!(matches!(
            table_value(help, &field("summary")),
            Some(Expr::String(summary)) if summary.contains(summary_fragment)
        ));

        let subject_value = symbol_value(&cx, &subject);
        let examples = call(
            &mut cx,
            Symbol::qualified("core", "examples"),
            vec![subject_value],
        );
        let examples = expr(&mut cx, &examples);
        let examples = list_items(&examples);
        assert_eq!(
            examples.len(),
            1,
            "{subject} should have one schema example"
        );
        assert_eq!(
            table_value(&examples[0], &field("example")),
            Some(&Expr::Bool(true))
        );
        assert_list_contains_symbol(
            table_value(&examples[0], &field("subjects")).expect("subjects"),
            subject,
        );
    }
}

#[test]
fn browse_functions_and_projection_functions_are_browsable() {
    let mut cx = test_cx();

    for (namespace, name) in BROWSE_FUNCTIONS {
        let subject = Symbol::qualified(*namespace, *name);
        let card = browse(&mut cx, subject.clone());
        let card = expr(&mut cx, &card);

        assert_eq!(
            table_value(&card, &field("kind")),
            Some(&Expr::Symbol(Symbol::qualified("core", "function"))),
            "{subject} should be a function Card"
        );
        assert_eq!(
            table_value(&card, &field("shape-known")),
            Some(&Expr::Bool(true)),
            "{subject} should publish total call shapes"
        );

        let help = table_value(&card, &field("help")).expect("help");
        assert_eq!(
            table_value(help, &field("kind")),
            Some(&Expr::Symbol(Symbol::qualified("core", "function")))
        );
        assert!(matches!(
            table_value(help, &field("detail")),
            Some(Expr::String(detail)) if detail.contains(&subject.to_string())
        ));
    }
}

#[test]
fn registry_catalog_subject_returns_card_with_table_backed_facet() {
    let mut cx = test_cx();
    cx.grant(registry_catalog_read_capability());
    let subject = Symbol::qualified("registry", "catalog");
    let card = browse(&mut cx, subject.clone());
    let card_expr = expr(&mut cx, &card);

    assert_eq!(table_keys(&card_expr), symbols(CARD_V2_FIELDS));
    assert_eq!(
        table_value(&card_expr, &field("subject")),
        Some(&Expr::Symbol(subject.clone()))
    );
    assert!(
        shape_accepts_card(&mut cx, &card_expr),
        "{subject} Card should satisfy core/Card"
    );

    let help = table_value(&card_expr, &field("help")).expect("help");
    assert!(matches!(
        table_value(help, &field("summary")),
        Some(Expr::String(summary)) if summary.contains("registry catalog")
    ));

    let facets_value = value_field(&mut cx, &card, "facets");
    let facets = value_list(&mut cx, facets_value);
    let facet =
        facet_value_by_name(&mut cx, &facets, subject.clone()).expect("registry catalog facet");
    assert_eq!(
        value_field_expr(&mut cx, &facet, "kind"),
        Expr::Symbol(Symbol::new("catalog"))
    );
    assert_list_contains_symbol(
        &value_field_expr(&mut cx, &facet, "requires"),
        Symbol::qualified("capability", "registry.catalog.read"),
    );

    let catalog = value_field(&mut cx, &facet, "value");
    assert!(
        catalog.object().as_dir().is_some(),
        "catalog facet should expose a Dir view"
    );
    let table = catalog
        .object()
        .as_table_impl()
        .expect("catalog facet should expose a Table view");
    assert!(
        table.has(&mut cx, Symbol::new("registry")).unwrap(),
        "catalog root should expose registry namespace"
    );
    let registry_dir = table.get(&mut cx, Symbol::new("registry")).unwrap();
    let registry_table = registry_dir
        .object()
        .as_table_impl()
        .expect("registry namespace should be table-backed");
    assert!(
        registry_table.has(&mut cx, Symbol::new("exports")).unwrap(),
        "registry namespace should expose exports table"
    );
}

#[test]
fn schema_shapes_accept_complete_values_and_reject_malformed_tables() {
    let mut cx = test_cx();
    let card = browse(&mut cx, Symbol::qualified("core", "Card"));
    assert_shape_accepts(&mut cx, Symbol::qualified("core", "Card"), card);

    let help = HelpBuilder::new(symbol_value(&cx, &Symbol::qualified("core", "Card")))
        .build(&mut cx)
        .unwrap();
    assert_shape_accepts(&mut cx, Symbol::qualified("browse", "Help"), help);

    let test = BrowseTestBuilder::new(
        Symbol::qualified("browse-example", "core-card"),
        Symbol::new("core"),
    )
    .build(&mut cx)
    .unwrap();
    assert_shape_accepts(&mut cx, Symbol::qualified("browse", "Test"), test);

    for (shape, field_name) in [
        (Symbol::qualified("core", "Card"), "subject"),
        (Symbol::qualified("browse", "Help"), "subject"),
        (Symbol::qualified("browse", "Test"), "name"),
    ] {
        let malformed = cx
            .factory()
            .table(vec![(field(field_name), cx.factory().nil().unwrap())])
            .unwrap();
        assert!(
            !shape_accepts(&mut cx, shape.clone(), malformed),
            "{shape} should reject missing required fields"
        );
    }
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn browse(cx: &mut Cx, subject: Symbol) -> Value {
    let subject = symbol_value(cx, &subject);
    call(cx, Symbol::qualified("core", "browse"), vec![subject])
}

fn call(cx: &mut Cx, symbol: Symbol, args: Vec<Value>) -> Value {
    cx.call_function(&symbol, Args::new(args))
        .unwrap_or_else(|err| panic!("{symbol} failed: {err}"))
}

fn symbol_value(cx: &Cx, symbol: &Symbol) -> Value {
    cx.factory().symbol(symbol.clone()).unwrap()
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

fn list_items(expr: &Expr) -> &[Expr] {
    let Expr::List(items) = expr else {
        panic!("expected list");
    };
    items
}

fn symbols(fields: &[&str]) -> Vec<Symbol> {
    fields.iter().map(|field| Symbol::new(*field)).collect()
}

fn field(name: &str) -> Symbol {
    Symbol::new(name.to_owned())
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

fn value_list(cx: &mut Cx, value: Value) -> Vec<Value> {
    let Some(list) = value.object().as_list() else {
        panic!("expected list value");
    };
    force_list_to_vec(cx, list, "browse reflection list").unwrap()
}

fn facet_value_by_name(cx: &mut Cx, facets: &[Value], name: Symbol) -> Option<Value> {
    for facet in facets {
        if value_field_expr(cx, facet, "name") == Expr::Symbol(name.clone()) {
            return Some(facet.clone());
        }
    }
    None
}

fn value_field_expr(cx: &mut Cx, value: &Value, name: &str) -> Expr {
    let field_value = value_field(cx, value, name);
    expr(cx, &field_value)
}

fn value_field(cx: &mut Cx, value: &Value, name: &str) -> Value {
    let key = field(name);
    value_entries(cx, value)
        .into_iter()
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
        .unwrap_or_else(|| panic!("missing field {name}"))
}

fn value_entries(cx: &mut Cx, value: &Value) -> Vec<(Symbol, Value)> {
    if let Some(table) = value.object().as_table_impl() {
        return table.entries(cx).unwrap();
    }
    let table = value.object().as_table(cx).unwrap();
    table
        .object()
        .as_table_impl()
        .expect("expected table value")
        .entries(cx)
        .unwrap()
}

fn shape_accepts_card(cx: &mut Cx, expr: &Expr) -> bool {
    let value = cx.factory().expr(expr.clone()).unwrap();
    shape_accepts(cx, Symbol::qualified("core", "Card"), value)
}

fn assert_shape_accepts(cx: &mut Cx, shape: Symbol, value: Value) {
    assert!(
        shape_accepts(cx, shape.clone(), value),
        "{shape} should accept complete value"
    );
}

fn shape_accepts(cx: &mut Cx, shape: Symbol, value: Value) -> bool {
    let shape = cx.resolve_shape(&shape).unwrap();
    shape
        .object()
        .as_shape()
        .unwrap()
        .check_value(cx, value)
        .unwrap()
        .accepted
}
