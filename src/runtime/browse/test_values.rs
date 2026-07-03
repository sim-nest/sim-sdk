use sim_kernel::{Cx, Ref, Result, Symbol, Value, library::RegisteredTest};

pub(super) fn all_test_cards(cx: &mut Cx) -> Result<Vec<Value>> {
    let tests = cx.registry().tests().values().cloned().collect::<Vec<_>>();
    tests
        .iter()
        .map(|test| fixed_test_value(cx, test))
        .collect()
}

pub(super) fn lib_test_cards(cx: &mut Cx, lib: &Symbol) -> Result<Vec<Value>> {
    let tests = cx
        .registry()
        .tests_for_lib(lib)
        .unwrap_or(&[])
        .iter()
        .filter_map(|symbol| cx.registry().registered_test(symbol).cloned())
        .collect::<Vec<_>>();
    tests
        .iter()
        .map(|test| fixed_test_value(cx, test))
        .collect()
}

pub(super) fn test_card_v1_for_symbol(cx: &mut Cx, symbol: &Symbol) -> Result<Option<Value>> {
    let Some(test) = cx.registry().registered_test(symbol).cloned() else {
        return Ok(None);
    };
    test_card_v1(cx, &test).map(Some)
}

pub(super) fn subject_tests_fallback(cx: &mut Cx, subject: &Symbol) -> Result<Option<Value>> {
    let tests = subject_test_cards(cx, subject)?;
    if tests.is_empty() {
        return Ok(None);
    }
    let tests = cx.factory().list(tests)?;
    cx.factory()
        .table(vec![(field_symbol("tests"), tests)])
        .map(Some)
}

fn subject_test_cards(cx: &mut Cx, subject: &Symbol) -> Result<Vec<Value>> {
    let tests = cx
        .registry()
        .tests()
        .values()
        .filter(|test| test.subjects.iter().any(|candidate| candidate == subject))
        .cloned()
        .collect::<Vec<_>>();
    tests
        .iter()
        .map(|test| fixed_test_value(cx, test))
        .collect()
}

fn test_card_v1(cx: &mut Cx, test: &RegisteredTest) -> Result<Value> {
    let fallback = fixed_test_value(cx, test)?;
    sim_kernel::card::card_for_ref_with_fallback(
        cx,
        Ref::Symbol(test.symbol.clone()),
        Some(fallback),
        Some(Symbol::qualified("core", "test")),
    )
}

fn fixed_test_value(cx: &mut Cx, test: &RegisteredTest) -> Result<Value> {
    let described = test.test.describe(cx)?;
    let entries = table_entries(cx, described)?;
    let subjects = test
        .subjects
        .iter()
        .cloned()
        .map(|subject| cx.factory().symbol(subject))
        .collect::<Result<Vec<_>>>()?;

    let fields = vec![
        (
            field_symbol("name"),
            field_or_symbol(cx, &entries, "name", test.symbol.clone())?,
        ),
        (
            field_symbol("subjects"),
            field_or_list(cx, &entries, "subjects", subjects)?,
        ),
        (
            field_symbol("lib"),
            field_or_symbol(cx, &entries, "lib", test.lib.clone())?,
        ),
        (
            field_symbol("mode"),
            field_or_symbol(cx, &entries, "mode", Symbol::qualified("test", "unknown"))?,
        ),
        (field_symbol("expr"), field_or_nil(cx, &entries, "expr")?),
        (
            field_symbol("expr-codec"),
            field_or_symbol(cx, &entries, "expr-codec", default_test_codec())?,
        ),
        (
            field_symbol("expected"),
            field_or_nil(cx, &entries, "expected")?,
        ),
        (
            field_symbol("expected-codec"),
            field_or_nil(cx, &entries, "expected-codec")?,
        ),
        (
            field_symbol("expected-error"),
            field_or_nil(cx, &entries, "expected-error")?,
        ),
        (
            field_symbol("codecs"),
            field_or_list(cx, &entries, "codecs", Vec::new())?,
        ),
        (
            field_symbol("example"),
            field_or_bool(cx, &entries, "example", false)?,
        ),
        (
            field_symbol("capabilities"),
            field_or_list(cx, &entries, "capabilities", Vec::new())?,
        ),
    ];
    cx.factory().table(fields)
}

fn table_entries(cx: &mut Cx, value: Value) -> Result<Vec<(Symbol, Value)>> {
    if let Some(table) = value.object().as_table_impl() {
        return table.entries(cx);
    }
    let table = value.object().as_table(cx)?;
    match table.object().as_table_impl() {
        Some(table) => table.entries(cx),
        None => Ok(Vec::new()),
    }
}

fn field(entries: &[(Symbol, Value)], name: &str) -> Option<Value> {
    super::fields::value_field(entries, name).cloned()
}

fn field_or_symbol(
    cx: &mut Cx,
    entries: &[(Symbol, Value)],
    name: &str,
    default: Symbol,
) -> Result<Value> {
    match field(entries, name) {
        Some(value) => Ok(value),
        None => cx.factory().symbol(default),
    }
}

fn field_or_list(
    cx: &mut Cx,
    entries: &[(Symbol, Value)],
    name: &str,
    default: Vec<Value>,
) -> Result<Value> {
    match field(entries, name) {
        Some(value) => Ok(value),
        None => cx.factory().list(default),
    }
}

fn field_or_bool(
    cx: &mut Cx,
    entries: &[(Symbol, Value)],
    name: &str,
    default: bool,
) -> Result<Value> {
    match field(entries, name) {
        Some(value) => Ok(value),
        None => cx.factory().bool(default),
    }
}

fn field_or_nil(cx: &mut Cx, entries: &[(Symbol, Value)], name: &str) -> Result<Value> {
    match field(entries, name) {
        Some(value) => Ok(value),
        None => cx.factory().nil(),
    }
}

use super::fields::key as field_symbol;

fn default_test_codec() -> Symbol {
    Symbol::qualified("codec", "lisp")
}
