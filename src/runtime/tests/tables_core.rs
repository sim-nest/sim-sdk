use sim_kernel::{
    Args, Cx, Error, Expr, Symbol, Table, Value, catalog::CatalogTable,
    config_table_impl_capability,
};

use super::support::eval_cx;

fn number_text(expr: Expr) -> String {
    match expr {
        Expr::Number(number) => number.canonical,
        other => panic!("expected number expression, found {other:?}"),
    }
}

fn number_value(cx: &mut Cx, text: &str) -> Value {
    cx.factory()
        .number_literal(Symbol::qualified("numbers", "f64"), text.to_owned())
        .unwrap()
}

fn expr(cx: &mut Cx, value: &Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

fn map_field<'a>(expr: &'a Expr, field: &str) -> &'a Expr {
    let Expr::Map(entries) = expr else {
        panic!("expected map expression");
    };
    entries
        .iter()
        .find_map(|(key, value)| match key {
            Expr::Symbol(symbol) if symbol == &Symbol::new(field) => Some(value),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing field {field}"))
}

#[test]
fn table_impl_switch_is_capability_gated() {
    let mut cx = eval_cx();
    let err = cx
        .call_function(
            &Symbol::new("table-impl"),
            Args::new(vec![cx.factory().symbol(Symbol::new("assoc")).unwrap()]),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        Error::CapabilityDenied { capability } if capability == config_table_impl_capability()
    ));

    cx.grant(config_table_impl_capability());
    let active = cx
        .call_function(
            &Symbol::new("table-impl"),
            Args::new(vec![cx.factory().symbol(Symbol::new("assoc")).unwrap()]),
        )
        .unwrap();
    assert_eq!(
        active.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("assoc"))
    );
}

#[test]
fn directory_ops_fail_cleanly_on_non_directory_tables() {
    let mut cx = eval_cx();
    let table = cx.new_table(Vec::new()).unwrap();
    let name = cx.factory().symbol(Symbol::new("sub")).unwrap();
    let err = cx
        .call_function(&Symbol::new("mkdir"), Args::new(vec![table, name]))
        .unwrap_err();
    assert!(matches!(
        err,
        Error::Eval(message) if message == "this table backend does not support directories"
    ));
}

#[test]
fn table_ops_fail_cleanly_on_non_table_values() {
    let mut cx = eval_cx();
    let number = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "f64"), "5".to_owned())
        .unwrap();
    let key = cx.factory().symbol(Symbol::new("x")).unwrap();

    let err = cx
        .call_function(&Symbol::new("get"), Args::new(vec![number, key]))
        .unwrap_err();
    match err {
        Error::Eval(message) => {
            assert!(message.contains("get expects a table and a key"));
        }
        other => panic!("expected Error::Eval, found {other:?}"),
    }
}

#[test]
fn directory_ops_fail_cleanly_on_non_table_values() {
    let mut cx = eval_cx();
    let number = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "f64"), "5".to_owned())
        .unwrap();
    let name = cx.factory().symbol(Symbol::new("sub")).unwrap();

    let err = cx
        .call_function(&Symbol::new("mkdir"), Args::new(vec![number, name]))
        .unwrap_err();
    match err {
        Error::Eval(message) => {
            assert!(message.contains("expected a directory table"));
        }
        other => panic!("expected Error::Eval, found {other:?}"),
    }
}

#[test]
fn table_len_dispatches_to_table_entries() {
    let mut cx = eval_cx();
    let one = number_value(&mut cx, "1");
    let table = cx.new_table(vec![(Symbol::new("a"), one)]).unwrap();
    let len = cx
        .call_function(&Symbol::new("len"), Args::new(vec![table]))
        .unwrap();
    assert_eq!(number_text(len.object().as_expr(&mut cx).unwrap()), "1");
}

#[test]
fn catalog_table_constructor_supports_table_mutation_ops() {
    let mut cx = eval_cx();
    let one = number_value(&mut cx, "1");
    let two = number_value(&mut cx, "2");
    let three = number_value(&mut cx, "3");
    let key_b = cx.factory().symbol(Symbol::new("b")).unwrap();
    let key_a = cx.factory().symbol(Symbol::new("a")).unwrap();
    let table = cx
        .call_function(
            &Symbol::qualified("table", "catalog"),
            Args::new(vec![key_b, two.clone(), key_a, one.clone()]),
        )
        .unwrap();
    let table_impl = table.object().as_table_impl().unwrap();

    assert_eq!(
        table_impl.backend_symbol(),
        Symbol::qualified("table", "catalog")
    );
    let a = table_impl.get(&mut cx, Symbol::new("a")).unwrap();
    assert_eq!(number_text(expr(&mut cx, &a)), "1");
    assert!(table_impl.has(&mut cx, Symbol::new("b")).unwrap());

    table_impl
        .set(&mut cx, Symbol::new("c"), three.clone())
        .unwrap();
    assert_eq!(table_impl.len(&mut cx).unwrap(), 3);
    let c = table_impl.del(&mut cx, Symbol::new("c")).unwrap();
    assert_eq!(number_text(expr(&mut cx, &c)), "3");

    table_impl.clear(&mut cx).unwrap();
    assert_eq!(table_impl.len(&mut cx).unwrap(), 0);
}

#[test]
fn catalog_table_missing_get_returns_nil() {
    let mut cx = eval_cx();
    let table = CatalogTable::new().unwrap();

    let missing = table.get(&mut cx, Symbol::new("missing")).unwrap();

    assert_eq!(expr(&mut cx, &missing), Expr::Nil);
}

#[test]
fn catalog_table_entries_are_sorted_by_key() {
    let mut cx = eval_cx();
    let table = CatalogTable::with_entries(vec![
        (Symbol::new("b"), number_value(&mut cx, "2")),
        (Symbol::new("a"), number_value(&mut cx, "1")),
    ])
    .unwrap();

    assert_eq!(
        table.keys(&mut cx).unwrap(),
        vec![Symbol::new("a"), Symbol::new("b")]
    );
    let entry_keys = table
        .entries(&mut cx)
        .unwrap()
        .into_iter()
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    assert_eq!(entry_keys, vec![Symbol::new("a"), Symbol::new("b")]);
}

#[test]
fn catalog_table_snapshot_contains_catalog_rows() {
    let mut cx = eval_cx();
    let table =
        CatalogTable::with_entries(vec![(Symbol::new("a"), number_value(&mut cx, "1"))]).unwrap();
    let snapshot = table.snapshot().unwrap();
    let entries_table = Symbol::qualified("table", "entries");
    let value_field = Symbol::new("value");

    assert!(snapshot.tables.contains_key(&entries_table));
    let rows = snapshot.rows(&entries_table).unwrap();
    let row = rows.get(&Symbol::new("a")).unwrap();
    assert_eq!(
        row.data.get(&Symbol::new("key")),
        Some(&Expr::Symbol(Symbol::new("a")))
    );
    let Expr::Extension { tag, payload } = row.data.get(&value_field).unwrap() else {
        panic!("expected unresolved live value marker");
    };
    assert_eq!(tag, &Symbol::qualified("catalog", "unresolved-live"));
    assert_eq!(
        map_field(payload, "table"),
        &Expr::Symbol(entries_table.clone())
    );
    assert_eq!(map_field(payload, "key"), &Expr::Symbol(Symbol::new("a")));
    assert_eq!(
        map_field(payload, "field"),
        &Expr::Symbol(value_field.clone())
    );
}

#[test]
fn catalog_backend_can_be_selected_without_changing_default() {
    let mut cx = eval_cx();

    assert_eq!(cx.table_registry().active(), "assoc");
    cx.grant(config_table_impl_capability());
    let backend = cx
        .call_function(
            &Symbol::new("table-impl"),
            Args::new(vec![
                cx.factory()
                    .symbol(Symbol::qualified("table", "catalog"))
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        expr(&mut cx, &backend),
        Expr::Symbol(Symbol::new("table/catalog"))
    );

    let table = cx.new_table(Vec::new()).unwrap();
    assert_eq!(
        table.object().as_table_impl().unwrap().backend_symbol(),
        Symbol::qualified("table", "catalog")
    );
}
