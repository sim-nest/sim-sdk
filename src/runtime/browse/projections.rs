use std::sync::Arc;

use sim_kernel::{
    CaseId, ContentId, Cx, Datum, DatumStore, Error, FunctionId, PreparedArgs, Ref, Result, Symbol,
    Value, force_list_to_vec, value_from_datum,
};
use sim_shape::{AnyShape, Bindings, CaptureShape, ListShape};

use crate::functions::{FunctionCase, FunctionObject};

use super::{
    graph::leaf_card_for_symbol,
    ref_parse::prepared_ref,
    registry::{registry_catalog_card, registry_catalog_subject, tests_impl},
    schema::{card_v2_for_ref, card_v2_from_card_v1},
    surface_cards::{root_surface_symbols, surface_card_for_symbol},
    test_values,
};
use crate::runtime::help::build_help;

type BrowseImpl = fn(&mut Cx, &PreparedArgs, Bindings) -> Result<Value>;

const ROOT_SURFACES: &[(&str, &str)] = &[
    ("core", "libs"),
    ("core", "classes"),
    ("core", "functions"),
    ("core", "macros"),
    ("core", "shapes"),
    ("core", "codecs"),
    ("core", "number-domains"),
    ("core", "eval-policies"),
    ("core", "tests"),
    ("core", "help"),
    ("core", "browse-neighbors"),
    ("core", "browse-path"),
    ("registry", "catalog"),
];

pub(crate) fn browse_function(
    root_case_id: CaseId,
    subject_case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![
            zero_case(root_case_id, &symbol, "root", browse_root_impl),
            subject_case(subject_case_id, &symbol, "subject", browse_subject_impl),
        ],
    )
}

pub(crate) fn tests_function(
    all_case_id: CaseId,
    subject_case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![
            zero_case(all_case_id, &symbol, "all", tests_impl),
            subject_case(subject_case_id, &symbol, "subject", tests_projection_impl),
        ],
    )
}

pub(crate) fn args_function(
    case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    projection_function(case_id, function_id, symbol, args_impl)
}

pub(crate) fn result_function(
    case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    projection_function(case_id, function_id, symbol, result_impl)
}

pub(crate) fn examples_function(
    case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    projection_function(case_id, function_id, symbol, examples_impl)
}

pub(crate) fn coverage_function(
    case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    projection_function(case_id, function_id, symbol, coverage_impl)
}

pub(crate) fn facets_function(
    case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    projection_function(case_id, function_id, symbol, facets_impl)
}

pub(crate) fn help_object_function(
    case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    projection_function(case_id, function_id, symbol, help_object_impl)
}

fn projection_function(
    case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
    implementation: BrowseImpl,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![subject_case(case_id, &symbol, "subject", implementation)],
    )
}

fn zero_case(
    case_id: CaseId,
    symbol: &Symbol,
    name: &'static str,
    implementation: BrowseImpl,
) -> FunctionCase {
    FunctionCase {
        id: case_id,
        name: Symbol::qualified(symbol.to_string(), name),
        args: Arc::new(ListShape::new(Vec::new())),
        result: Some(Arc::new(AnyShape)),
        demand: Vec::new(),
        priority: 10,
        implementation,
    }
}

fn subject_case(
    case_id: CaseId,
    symbol: &Symbol,
    name: &'static str,
    implementation: BrowseImpl,
) -> FunctionCase {
    FunctionCase {
        id: case_id,
        name: Symbol::qualified(symbol.to_string(), name),
        args: Arc::new(ListShape::new(vec![Arc::new(CaptureShape::new(
            Symbol::new("subject"),
            Arc::new(AnyShape),
        ))])),
        result: Some(Arc::new(AnyShape)),
        demand: vec![sim_kernel::Demand::Value],
        priority: 10,
        implementation,
    }
}

fn browse_root_impl(cx: &mut Cx, _prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    root_catalog_card(cx)
}

fn browse_subject_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let subject = prepared_ref(prepared, cx, 0, "core/browse expects zero or one subject")?;
    browse_card_for_ref(cx, subject)
}

fn args_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    projection_field(cx, prepared, "args", "core/args expects one subject")
}

fn result_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    projection_field(cx, prepared, "result", "core/result expects one subject")
}

fn tests_projection_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    projection_field(
        cx,
        prepared,
        "tests",
        "core/tests expects zero args or one subject",
    )
}

fn examples_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let tests = projection_field(cx, prepared, "tests", "core/examples expects one subject")?;
    examples_from_tests(cx, tests)
}

fn coverage_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    projection_field(
        cx,
        prepared,
        "coverage",
        "core/coverage expects one subject",
    )
}

fn facets_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    projection_field(cx, prepared, "facets", "core/facets expects one subject")
}

fn help_object_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    projection_field(cx, prepared, "help", "core/help-object expects one subject")
}

pub(super) fn root_catalog_symbol() -> Symbol {
    Symbol::qualified("browse", "catalog")
}

pub(super) fn root_catalog_card(cx: &mut Cx) -> Result<Value> {
    let subject = Ref::Symbol(root_catalog_symbol());
    let see_also = root_catalog_symbols()
        .into_iter()
        .map(|symbol| cx.factory().symbol(symbol))
        .collect::<Result<Vec<_>>>()?;
    let fallback = cx.factory().table(vec![
        (
            field_symbol("kind"),
            cx.factory()
                .symbol(Symbol::qualified("browse", "catalog"))?,
        ),
        (
            field_symbol("help"),
            cx.factory()
                .string("root browse catalog for installed runtime surfaces".to_owned())?,
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
        (field_symbol("see-also"), cx.factory().list(see_also)?),
        (field_symbol("shape-known"), cx.factory().bool(true)?),
    ])?;
    let card_v1 = sim_kernel::card::card_for_ref_with_fallback(
        cx,
        subject.clone(),
        Some(fallback),
        Some(Symbol::qualified("browse", "catalog")),
    )?;
    card_v2_from_card_v1(cx, subject, card_v1)
}

fn root_catalog_symbols() -> Vec<Symbol> {
    let mut symbols = ROOT_SURFACES
        .iter()
        .map(|(namespace, name)| Symbol::qualified(*namespace, *name))
        .collect::<Vec<_>>();
    symbols.extend(root_surface_symbols());
    #[cfg(feature = "cookbook")]
    symbols.extend(crate::runtime::cookbook_discovery::root_symbols());
    symbols
}

fn projection_field(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    field: &str,
    message: &str,
) -> Result<Value> {
    let subject = prepared_ref(prepared, cx, 0, message)?;
    let card = browse_card_for_ref(cx, subject)?;
    card_field(cx, card, field)
}

pub(super) fn browse_card_for_ref(cx: &mut Cx, subject: Ref) -> Result<Value> {
    if subject == Ref::Symbol(root_catalog_symbol()) {
        return root_catalog_card(cx);
    }
    if subject == Ref::Symbol(registry_catalog_subject()) {
        return registry_catalog_card(cx);
    }
    if let Ref::Content(id) = &subject {
        return content_card_for_id(cx, id.clone());
    }
    if let Ref::Symbol(symbol) = &subject {
        publish_help_doc_if_known(cx, symbol)?;
        if let Some(card_v1) = test_values::test_card_v1_for_symbol(cx, symbol)? {
            return card_v2_from_card_v1(cx, subject, card_v1);
        }
        if let Some(fallback) = test_values::subject_tests_fallback(cx, symbol)? {
            let card_v1 = sim_kernel::card::card_for_ref_with_fallback(
                cx,
                subject.clone(),
                Some(fallback),
                None,
            )?;
            return card_v2_from_card_v1(cx, subject, card_v1);
        }
        if let Some(card) = surface_card_for_symbol(cx, symbol)? {
            return Ok(card);
        }
        #[cfg(feature = "cookbook")]
        if let Some(card) = crate::runtime::cookbook_discovery::card_for_symbol(cx, symbol)? {
            return Ok(card);
        }
        if let Some(card) = leaf_card_for_symbol(cx, symbol)? {
            return Ok(card);
        }
    }
    card_v2_for_ref(cx, subject)
}

fn content_card_for_id(cx: &mut Cx, id: ContentId) -> Result<Value> {
    let subject = Ref::Content(id.clone());
    let (fallback, kind) = match cx.datum_store().get(&id)?.cloned() {
        Some(datum) => (
            content_ref_fallback(cx, &id, datum)?,
            Symbol::qualified("browse", "content"),
        ),
        None => (
            missing_content_ref_fallback(cx, &id)?,
            Symbol::qualified("browse", "missing-ref"),
        ),
    };
    let card_v1 = sim_kernel::card::card_for_ref_with_fallback(
        cx,
        subject.clone(),
        Some(fallback),
        Some(kind),
    )?;
    card_v2_from_card_v1(cx, subject, card_v1)
}

fn content_ref_fallback(cx: &mut Cx, id: &ContentId, datum: Datum) -> Result<Value> {
    let datum_kind = datum_kind(&datum);
    let value = value_from_datum(cx, datum)?;
    let mut entries = content_ref_base_entries(
        cx,
        id,
        Symbol::qualified("browse", "content"),
        "content-addressed Datum stored in DatumStore",
        true,
    )?;
    entries.push((field_symbol("datum-kind"), cx.factory().symbol(datum_kind)?));
    entries.push((field_symbol("value"), value));
    cx.factory().table(entries)
}

fn missing_content_ref_fallback(cx: &mut Cx, id: &ContentId) -> Result<Value> {
    let mut entries = content_ref_base_entries(
        cx,
        id,
        Symbol::qualified("browse", "missing-ref"),
        "content reference is not present in DatumStore",
        false,
    )?;
    entries.push((
        field_symbol("missing-ref"),
        sim_kernel::card::ref_value(cx, &Ref::Content(id.clone()))?,
    ));
    cx.factory().table(entries)
}

fn content_ref_base_entries(
    cx: &mut Cx,
    id: &ContentId,
    kind: Symbol,
    help: &str,
    shape_known: bool,
) -> Result<Vec<(Symbol, Value)>> {
    Ok(vec![
        (field_symbol("kind"), cx.factory().symbol(kind)?),
        (field_symbol("help"), cx.factory().string(help.to_owned())?),
        (
            field_symbol("args"),
            cx.factory().symbol(Symbol::qualified("core", "Any"))?,
        ),
        (
            field_symbol("result"),
            cx.factory().symbol(Symbol::qualified("core", "Any"))?,
        ),
        (field_symbol("tests"), cx.factory().list(Vec::new())?),
        (field_symbol("ops"), cx.factory().list(Vec::new())?),
        (field_symbol("requires"), cx.factory().list(Vec::new())?),
        (field_symbol("see-also"), cx.factory().list(Vec::new())?),
        (field_symbol("shape-known"), cx.factory().bool(shape_known)?),
        (
            field_symbol("algorithm"),
            cx.factory().symbol(id.algorithm.clone())?,
        ),
        (
            field_symbol("bytes"),
            cx.factory().bytes(id.bytes.to_vec())?,
        ),
    ])
}

fn datum_kind(datum: &Datum) -> Symbol {
    let name = match datum {
        Datum::Nil => "nil",
        Datum::Bool(_) => "bool",
        Datum::Number(_) => "number",
        Datum::Symbol(_) => "symbol",
        Datum::String(_) => "string",
        Datum::Bytes(_) => "bytes",
        Datum::List(_) => "list",
        Datum::Vector(_) => "vector",
        Datum::Map(_) => "map",
        Datum::Set(_) => "set",
        Datum::Node { .. } => "node",
    };
    Symbol::qualified("datum", name)
}

fn publish_help_doc_if_known(cx: &mut Cx, subject: &Symbol) -> Result<()> {
    match build_help(cx, subject) {
        Ok(help) => help.publish_claims(cx),
        Err(Error::UnknownSymbol { .. }) => Ok(()),
        Err(err) => Err(err),
    }
}

fn card_field(cx: &mut Cx, card: Value, field: &str) -> Result<Value> {
    let key = field_symbol(field);
    table_entries(cx, card)?
        .into_iter()
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
        .ok_or_else(|| Error::HostError(format!("missing Card field {field}")))
}

fn examples_from_tests(cx: &mut Cx, tests: Value) -> Result<Value> {
    let Some(list) = tests.object().as_list() else {
        return cx.factory().list(Vec::new());
    };
    let examples = force_list_to_vec(cx, list, "browse examples")?
        .into_iter()
        .filter_map(|test| match test.object().as_expr(cx) {
            Ok(expr) if expr_bool_field(&expr, "example") == Some(true) => Some(Ok(test)),
            Ok(_) => None,
            Err(err) => Some(Err(err)),
        })
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(examples)
}

use super::fields::{bool_field as expr_bool_field, key as field_symbol};

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
