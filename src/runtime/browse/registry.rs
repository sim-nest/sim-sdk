use std::{collections::BTreeSet, sync::Arc};

use sim_kernel::{ClaimPattern, Cx, Demand, Error, FunctionId, Ref, Result, Symbol, Value};
use sim_shape::{AnyShape, Bindings, CaptureShape, ListShape};

use crate::functions::{FunctionCase, FunctionObject};

use super::super::browse_run_tests_capability;
use super::super::{help::build_help, test_runs::test_report_value};
use super::schema::card_v2_from_card_v1;
use super::test_values;

fn zero_arg_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
    implementation: fn(&mut Cx, &sim_kernel::PreparedArgs, Bindings) -> Result<Value>,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "all"),
            args: Arc::new(ListShape::new(Vec::new())),
            result: Some(Arc::new(AnyShape)),
            demand: Vec::new(),
            priority: 10,
            implementation,
        }],
    )
}

fn one_symbol_arg_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
    implementation: fn(&mut Cx, &sim_kernel::PreparedArgs, Bindings) -> Result<Value>,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "one"),
            args: Arc::new(ListShape::new(vec![Arc::new(CaptureShape::new(
                Symbol::new("subject"),
                Arc::new(AnyShape),
            ))])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![Demand::Value],
            priority: 10,
            implementation,
        }],
    )
}

pub(crate) fn classes_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    zero_arg_function(case_id, function_id, symbol, classes_impl)
}

pub(crate) fn functions_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    zero_arg_function(case_id, function_id, symbol, functions_impl)
}

pub(crate) fn macros_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    zero_arg_function(case_id, function_id, symbol, macros_impl)
}

pub(crate) fn shapes_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    zero_arg_function(case_id, function_id, symbol, shapes_impl)
}

pub(crate) fn codecs_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    zero_arg_function(case_id, function_id, symbol, codecs_impl)
}

pub(crate) fn number_domains_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    zero_arg_function(case_id, function_id, symbol, number_domains_impl)
}

pub(crate) fn eval_policies_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    zero_arg_function(case_id, function_id, symbol, eval_policies_impl)
}

pub(crate) fn lib_tests_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    one_symbol_arg_function(case_id, function_id, symbol, lib_tests_impl)
}

pub(crate) fn run_tests_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    one_symbol_arg_function(case_id, function_id, symbol, run_tests_impl)
}

pub(crate) fn help_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    one_symbol_arg_function(case_id, function_id, symbol, help_impl)
}

pub(super) fn registry_catalog_subject() -> Symbol {
    Symbol::qualified("registry", "catalog")
}

pub(super) fn registry_catalog_card(cx: &mut Cx) -> Result<Value> {
    let subject = Ref::Symbol(registry_catalog_subject());
    let fallback = cx.factory().table(vec![
        (
            field_symbol("kind"),
            cx.factory()
                .symbol(Symbol::qualified("registry", "catalog"))?,
        ),
        (
            field_symbol("help"),
            cx.factory()
                .string("read-only registry catalog Dir view".to_owned())?,
        ),
        (
            field_symbol("args"),
            cx.factory().symbol(Symbol::qualified("core", "Any"))?,
        ),
        (
            field_symbol("result"),
            cx.factory().symbol(Symbol::qualified("core", "Card"))?,
        ),
        (field_symbol("tests"), cx.factory().list(Vec::new())?),
        (field_symbol("ops"), cx.factory().list(Vec::new())?),
        (field_symbol("requires"), cx.factory().list(Vec::new())?),
        (
            field_symbol("see-also"),
            cx.factory().list(vec![
                cx.factory()
                    .symbol(Symbol::qualified("browse", "catalog"))?,
            ])?,
        ),
        (field_symbol("shape-known"), cx.factory().bool(true)?),
    ])?;
    let card_v1 = sim_kernel::card::card_for_ref_with_fallback(
        cx,
        subject.clone(),
        Some(fallback),
        Some(Symbol::qualified("registry", "catalog")),
    )?;
    card_v2_from_card_v1(cx, subject, card_v1)
}

fn classes_impl(
    cx: &mut Cx,
    _prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let symbols = export_symbols_for_kind(cx, "class", cx.registry().classes().keys().cloned())?;
    registry_values(cx, symbols, "class", |cx, symbol| cx.resolve_class(symbol))
}

fn functions_impl(
    cx: &mut Cx,
    _prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let symbols =
        export_symbols_for_kind(cx, "function", cx.registry().functions().keys().cloned())?;
    registry_values(cx, symbols, "function", |cx, symbol| {
        cx.resolve_function(symbol)
    })
}

fn macros_impl(
    cx: &mut Cx,
    _prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let symbols = export_symbols_for_kind(cx, "macro", cx.registry().macros().keys().cloned())?;
    registry_values(cx, symbols, "macro", |cx, symbol| cx.resolve_macro(symbol))
}

fn shapes_impl(
    cx: &mut Cx,
    _prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let symbols = export_symbols_for_kind(cx, "shape", cx.registry().shapes().keys().cloned())?;
    registry_values(cx, symbols, "shape", |cx, symbol| cx.resolve_shape(symbol))
}

fn codecs_impl(
    cx: &mut Cx,
    _prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let symbols = export_symbols_for_kind(cx, "codec", cx.registry().codecs().keys().cloned())?;
    registry_values(cx, symbols, "codec", |cx, symbol| cx.resolve_codec(symbol))
}

fn number_domains_impl(
    cx: &mut Cx,
    _prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let symbols = export_symbols_for_kind(
        cx,
        "number-domain",
        cx.registry().number_domains().keys().cloned(),
    )?;
    registry_values(cx, symbols, "number-domain", |cx, symbol| {
        cx.resolve_number_domain(symbol)
    })
}

pub(super) fn tests_impl(
    cx: &mut Cx,
    _prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let tests = test_values::all_test_cards(cx)?;
    cx.factory().list(tests)
}

fn eval_policies_impl(
    cx: &mut Cx,
    _prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let entries = vec![
        eval_policy_value(cx, "eager", cx.eval_policy_name() == "eager")?,
        eval_policy_value(cx, "lazy", cx.eval_policy_name() == "lazy")?,
        eval_policy_value(cx, "lazy-by-need", cx.eval_policy_name() == "lazy-by-need")?,
        eval_policy_value(
            cx,
            "strict-by-shape",
            cx.eval_policy_name() == "strict-by-shape",
        )?,
        eval_policy_value(cx, "hybrid", cx.eval_policy_name() == "hybrid")?,
    ];
    cx.factory().list(entries)
}

fn lib_tests_impl(
    cx: &mut Cx,
    prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let lib = prepared_symbol(prepared, cx, 0, "core/lib-tests expects one lib symbol")?;
    let tests = test_values::lib_test_cards(cx, &lib)?;
    cx.factory().list(tests)
}

fn run_tests_impl(
    cx: &mut Cx,
    prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    cx.require(&browse_run_tests_capability())?;
    let requested = prepared_symbol(prepared, cx, 0, "core/run-tests expects one test request")?;
    let tests = requested_tests(cx, &requested);

    let reports = tests
        .into_iter()
        .map(|test| {
            let report = test.run(cx)?;
            test_report_value(cx, report)
        })
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(reports)
}

fn requested_tests(cx: &Cx, requested: &Symbol) -> Vec<std::sync::Arc<dyn sim_kernel::Test>> {
    if requested == &Symbol::new(":all") || requested == &Symbol::new("all") {
        return cx
            .registry()
            .tests()
            .values()
            .map(|registered| registered.test.clone())
            .collect();
    }

    if let Some(test) = cx.registry().test_by_symbol(requested) {
        return vec![test.clone()];
    }

    if let Some(symbols) = cx.registry().tests_for_lib(requested) {
        return symbols
            .iter()
            .filter_map(|symbol| cx.registry().test_by_symbol(symbol).cloned())
            .collect();
    }

    cx.registry()
        .tests()
        .values()
        .filter(|test| test.subjects.iter().any(|subject| subject == requested))
        .map(|registered| registered.test.clone())
        .collect()
}

fn help_impl(
    cx: &mut Cx,
    prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let subject = prepared_symbol(prepared, cx, 0, "core/help expects one symbol or string")?;
    let help = build_help(cx, &subject)?;
    help.publish_claims(cx)?;
    let fallback = help.fallback_value(cx)?;
    sim_kernel::card::card_for_ref_with_fallback(cx, Ref::Symbol(subject), Some(fallback), None)
}

fn registry_values(
    cx: &mut Cx,
    symbols: Vec<Symbol>,
    kind: &'static str,
    resolve: impl Fn(&mut Cx, &Symbol) -> Result<Value>,
) -> Result<Value> {
    let values = symbols
        .into_iter()
        .map(|symbol| {
            let value = match resolve(cx, &symbol) {
                Ok(value) => Some(value),
                Err(Error::UnknownSymbol { .. }) => None,
                Err(err) => return Err(err),
            };
            sim_kernel::card::card_for_ref_with_fallback(
                cx,
                Ref::Symbol(symbol),
                value,
                Some(core_kind(kind)),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

fn export_symbols_for_kind(
    cx: &Cx,
    kind: &'static str,
    fallback: impl IntoIterator<Item = Symbol>,
) -> Result<Vec<Symbol>> {
    let mut symbols = claimed_symbols_for_kind(cx, kind)?;
    symbols.extend(fallback);
    Ok(symbols.into_iter().collect())
}

fn claimed_symbols_for_kind(cx: &Cx, kind: &'static str) -> Result<BTreeSet<Symbol>> {
    let claims = cx.query_facts(ClaimPattern {
        subject: None,
        predicate: Some(sim_kernel::card::card_kind_predicate()),
        object: Some(Ref::Symbol(core_kind(kind))),
        include_revoked: false,
    })?;
    Ok(claims
        .into_iter()
        .filter_map(|claim| match claim.subject {
            Ref::Symbol(symbol) => Some(symbol),
            _ => None,
        })
        .collect())
}

fn core_kind(name: &'static str) -> Symbol {
    Symbol::qualified("core", name)
}

fn eval_policy_value(cx: &mut Cx, name: &str, current: bool) -> Result<Value> {
    cx.factory().table(vec![
        (
            Symbol::new("id"),
            cx.factory()
                .symbol(Symbol::qualified("core", name.to_owned()))?,
        ),
        (Symbol::new("name"), cx.factory().string(name.to_owned())?),
        (Symbol::new("current"), cx.factory().bool(current)?),
    ])
}

fn prepared_symbol(
    prepared: &sim_kernel::PreparedArgs,
    cx: &mut Cx,
    index: usize,
    message: &str,
) -> Result<Symbol> {
    let Some(value) = prepared.get(index) else {
        return Err(Error::Eval(message.to_owned()));
    };
    let expr = value.object().as_expr(cx)?;
    Ok(match expr {
        sim_kernel::Expr::Symbol(symbol) => symbol,
        sim_kernel::Expr::String(text) => parse_symbol_text(&text),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "symbol",
                found: "non-symbol",
            });
        }
    })
}

fn parse_symbol_text(value: &str) -> Symbol {
    match value.split_once('/') {
        Some((namespace, name)) if !namespace.is_empty() && !name.is_empty() => {
            Symbol::qualified(namespace.to_owned(), name.to_owned())
        }
        _ => Symbol::new(value.to_owned()),
    }
}

use super::fields::key as field_symbol;
