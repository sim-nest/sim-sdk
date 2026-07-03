use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

use sim_kernel::{
    Args, CaseId, Cx, Demand, Expr, FunctionId, PreparedArgs, Ref, Result, Symbol, Value,
    card::ref_value, force_list_to_vec,
};
use sim_shape::{AnyShape, Bindings, CaptureShape, ListShape};

use crate::functions::{FunctionCase, FunctionObject};

use super::{
    projections::{browse_card_for_ref, root_catalog_symbol},
    ref_parse::{parse_symbol_text, prepared_ref, ref_from_expr},
    schema::card_v2_from_card_v1,
};

const MAX_PATH_NODES: usize = 1024;

type BrowseImpl = fn(&mut Cx, &PreparedArgs, Bindings) -> Result<Value>;

#[derive(Clone, Copy)]
enum RefMode {
    Generic,
    Capability,
    SymbolText,
}

pub(crate) fn browse_neighbors_function(
    root_case_id: CaseId,
    subject_case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![
            zero_case(root_case_id, &symbol, "root", neighbors_root_impl),
            subject_case(subject_case_id, &symbol, "subject", neighbors_subject_impl),
        ],
    )
}

pub(crate) fn browse_path_function(
    case_id: CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "path"),
            args: Arc::new(ListShape::new(vec![
                Arc::new(CaptureShape::new(Symbol::new("from"), Arc::new(AnyShape))),
                Arc::new(CaptureShape::new(Symbol::new("to"), Arc::new(AnyShape))),
            ])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![Demand::Value, Demand::Value],
            priority: 10,
            implementation: path_impl,
        }],
    )
}

pub(super) fn leaf_card_for_symbol(cx: &mut Cx, symbol: &Symbol) -> Result<Option<Value>> {
    let Some(kind) = leaf_kind(symbol) else {
        return Ok(None);
    };
    let subject = Ref::Symbol(symbol.clone());
    let fallback = leaf_fallback(cx, symbol, kind.clone())?;
    let card_v1 = sim_kernel::card::card_for_ref_with_fallback(
        cx,
        subject.clone(),
        Some(fallback),
        Some(kind),
    )?;
    card_v2_from_card_v1(cx, subject, card_v1).map(Some)
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
        demand: vec![Demand::Value],
        priority: 10,
        implementation,
    }
}

fn neighbors_root_impl(
    cx: &mut Cx,
    _prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    neighbors_value(cx, Ref::Symbol(root_catalog_symbol()))
}

fn neighbors_subject_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let subject = prepared_ref(
        prepared,
        cx,
        0,
        "browse-neighbors expects zero or one subject",
    )?;
    neighbors_value(cx, subject)
}

fn path_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let start = prepared_ref(prepared, cx, 0, "browse-path expects a start ref")?;
    let target = prepared_ref(prepared, cx, 1, "browse-path expects a target ref")?;
    let Some(path) = shortest_path(cx, start, target)? else {
        return cx.factory().nil();
    };
    let values = path
        .iter()
        .map(|reference| ref_value(cx, reference))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

fn neighbors_value(cx: &mut Cx, subject: Ref) -> Result<Value> {
    let values = neighbors_for_ref(cx, &subject)?
        .iter()
        .map(|reference| ref_value(cx, reference))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

fn shortest_path(cx: &mut Cx, start: Ref, target: Ref) -> Result<Option<Vec<Ref>>> {
    let mut parents = BTreeMap::<Ref, Option<Ref>>::new();
    let mut queue = VecDeque::from([start.clone()]);
    parents.insert(start.clone(), None);

    while let Some(current) = queue.pop_front() {
        if current == target {
            return Ok(Some(reconstruct_path(&parents, current)));
        }
        if parents.len() >= MAX_PATH_NODES {
            break;
        }

        for neighbor in neighbors_for_ref(cx, &current)? {
            if parents.contains_key(&neighbor) {
                continue;
            }
            parents.insert(neighbor.clone(), Some(current.clone()));
            if neighbor == target {
                return Ok(Some(reconstruct_path(&parents, neighbor)));
            }
            if allow_list_reason(&neighbor).is_none() {
                queue.push_back(neighbor);
            }
        }
    }
    Ok(None)
}

fn reconstruct_path(parents: &BTreeMap<Ref, Option<Ref>>, mut current: Ref) -> Vec<Ref> {
    let mut path = Vec::new();
    loop {
        path.push(current.clone());
        let Some(Some(parent)) = parents.get(&current) else {
            break;
        };
        current = parent.clone();
    }
    path.reverse();
    path
}

fn neighbors_for_ref(cx: &mut Cx, subject: &Ref) -> Result<Vec<Ref>> {
    let mut out = BTreeSet::new();
    let card = browse_card_for_ref(cx, subject.clone())?;
    let expr = card.object().as_expr(cx)?;
    collect_card_refs(cx, &expr, &mut out)?;
    collect_catalog_refs(cx, subject, &mut out)?;
    Ok(out.into_iter().collect())
}

fn collect_card_refs(cx: &mut Cx, expr: &Expr, out: &mut BTreeSet<Ref>) -> Result<()> {
    let Expr::Map(entries) = expr else {
        return Ok(());
    };
    add_field_refs(entries, "subject", RefMode::Generic, out)?;
    add_field_refs(entries, "args", RefMode::Generic, out)?;
    add_field_refs(entries, "result", RefMode::Generic, out)?;
    add_field_refs(entries, "ops", RefMode::SymbolText, out)?;
    add_field_refs(entries, "requires", RefMode::Capability, out)?;
    add_field_refs(entries, "see-also", RefMode::Generic, out)?;
    add_help_refs(entries, out)?;
    add_tests_refs(entries, out)?;
    add_facets_refs(entries, out)?;
    add_coverage_refs(entries, out)?;
    add_field_refs(entries, "provenance", RefMode::Generic, out)?;
    let _ = cx;
    Ok(())
}

fn collect_catalog_refs(cx: &mut Cx, subject: &Ref, out: &mut BTreeSet<Ref>) -> Result<()> {
    let Ref::Symbol(symbol) = subject else {
        return Ok(());
    };
    if !is_catalog_surface(symbol) {
        return Ok(());
    }
    let value = cx.call_function(symbol, Args::new(Vec::new()))?;
    let Some(list) = value.object().as_list() else {
        let expr = value.object().as_expr(cx)?;
        return collect_card_refs(cx, &expr, out);
    };
    for item in force_list_to_vec(cx, list, "browse graph catalog")? {
        let expr = item.object().as_expr(cx)?;
        add_catalog_item_refs(&expr, out)?;
    }
    Ok(())
}

fn add_catalog_item_refs(expr: &Expr, out: &mut BTreeSet<Ref>) -> Result<()> {
    let Expr::Map(entries) = expr else {
        return add_refs_from_expr(expr, RefMode::Generic, out);
    };
    add_field_refs(entries, "subject", RefMode::Generic, out)?;
    add_field_refs(entries, "id", RefMode::Generic, out)?;
    add_field_refs(entries, "name", RefMode::Generic, out)?;
    add_field_refs(entries, "subjects", RefMode::Generic, out)?;
    add_field_refs(entries, "capabilities", RefMode::Capability, out)
}

fn add_help_refs(entries: &[(Expr, Expr)], out: &mut BTreeSet<Ref>) -> Result<()> {
    let Some(help) = expr_field(entries, "help") else {
        return Ok(());
    };
    let Expr::Map(help_entries) = help else {
        return Ok(());
    };
    add_field_refs(help_entries, "exported-by", RefMode::Generic, out)?;
    add_field_refs(help_entries, "see-also", RefMode::Generic, out)?;
    add_field_refs(help_entries, "capabilities", RefMode::Capability, out)
}

fn add_tests_refs(entries: &[(Expr, Expr)], out: &mut BTreeSet<Ref>) -> Result<()> {
    let Some(tests) = expr_field(entries, "tests") else {
        return Ok(());
    };
    let Expr::List(items) = tests else {
        return add_refs_from_expr(tests, RefMode::Generic, out);
    };
    for test in items {
        add_test_refs(test, out)?;
    }
    Ok(())
}

fn add_test_refs(expr: &Expr, out: &mut BTreeSet<Ref>) -> Result<()> {
    let Expr::Map(entries) = expr else {
        return add_refs_from_expr(expr, RefMode::Generic, out);
    };
    add_field_refs(entries, "name", RefMode::Generic, out)?;
    add_field_refs(entries, "lib", RefMode::Generic, out)?;
    add_field_refs(entries, "subjects", RefMode::Generic, out)?;
    add_field_refs(entries, "capabilities", RefMode::Capability, out)
}

fn add_facets_refs(entries: &[(Expr, Expr)], out: &mut BTreeSet<Ref>) -> Result<()> {
    let Some(Expr::List(facets)) = expr_field(entries, "facets") else {
        return Ok(());
    };
    for facet in facets {
        let Expr::Map(facet_entries) = facet else {
            continue;
        };
        add_field_refs(facet_entries, "name", RefMode::Generic, out)?;
        add_field_refs(facet_entries, "shape", RefMode::Generic, out)?;
        add_field_refs(facet_entries, "requires", RefMode::Capability, out)?;
        add_field_refs(facet_entries, "evidence", RefMode::Generic, out)?;
        if let Some(value) = expr_field(facet_entries, "value") {
            add_redaction_refs(value, out)?;
        }
    }
    Ok(())
}

fn add_redaction_refs(expr: &Expr, out: &mut BTreeSet<Ref>) -> Result<()> {
    let Expr::Map(entries) = expr else {
        return Ok(());
    };
    if expr_field(entries, "reason").is_some() && expr_field(entries, "requires").is_some() {
        add_field_refs(entries, "requires", RefMode::Capability, out)?;
    }
    Ok(())
}

fn add_coverage_refs(entries: &[(Expr, Expr)], out: &mut BTreeSet<Ref>) -> Result<()> {
    let Some(Expr::Map(coverage)) = expr_field(entries, "coverage") else {
        return Ok(());
    };
    add_field_refs(coverage, "last-run", RefMode::Generic, out)
}

fn add_field_refs(
    entries: &[(Expr, Expr)],
    name: &str,
    mode: RefMode,
    out: &mut BTreeSet<Ref>,
) -> Result<()> {
    if let Some(expr) = expr_field(entries, name) {
        add_refs_from_expr(expr, mode, out)?;
    }
    Ok(())
}

fn add_refs_from_expr(expr: &Expr, mode: RefMode, out: &mut BTreeSet<Ref>) -> Result<()> {
    if let Some(reference) = ref_from_expr(expr)? {
        out.insert(reference);
        return Ok(());
    }
    match expr {
        Expr::Symbol(symbol) => {
            out.insert(Ref::Symbol(symbol.clone()));
        }
        Expr::String(text) if matches!(mode, RefMode::Capability) => {
            out.insert(Ref::Symbol(Symbol::qualified("capability", text.clone())));
        }
        Expr::String(text) if matches!(mode, RefMode::SymbolText) => {
            out.insert(Ref::Symbol(parse_symbol_text(text)));
        }
        Expr::List(items) => {
            for item in items {
                add_refs_from_expr(item, mode, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

use super::fields::expr_entry_field as expr_field;

fn is_catalog_surface(symbol: &Symbol) -> bool {
    matches!(
        (symbol.namespace.as_deref(), symbol.name.as_ref()),
        (Some("core"), "libs")
            | (Some("core"), "classes")
            | (Some("core"), "functions")
            | (Some("core"), "macros")
            | (Some("core"), "shapes")
            | (Some("core"), "codecs")
            | (Some("core"), "number-domains")
            | (Some("core"), "eval-policies")
            | (Some("core"), "tests")
    )
}

fn leaf_fallback(cx: &mut Cx, symbol: &Symbol, kind: Symbol) -> Result<Value> {
    cx.factory().table(vec![
        (field_symbol("kind"), cx.factory().symbol(kind)?),
        (
            field_symbol("help"),
            cx.factory().string(leaf_help(symbol))?,
        ),
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
        (field_symbol("shape-known"), cx.factory().bool(false)?),
    ])
}

fn leaf_help(symbol: &Symbol) -> String {
    match leaf_kind(symbol) {
        Some(kind) if kind == Symbol::qualified("browse", "capability") => {
            format!("capability leaf {}", symbol.name)
        }
        Some(kind) if kind == Symbol::qualified("browse", "op") => {
            format!("operation key leaf {symbol}")
        }
        Some(_) => format!("generated shape leaf {symbol}"),
        None => String::new(),
    }
}

fn leaf_kind(symbol: &Symbol) -> Option<Symbol> {
    if symbol.namespace.as_deref() == Some("capability") {
        return Some(Symbol::qualified("browse", "capability"));
    }
    if is_operation_key(symbol) {
        return Some(Symbol::qualified("browse", "op"));
    }
    if is_generated_shape_symbol(symbol) {
        return Some(Symbol::qualified("core", "shape"));
    }
    None
}

fn is_operation_key(symbol: &Symbol) -> bool {
    symbol.namespace.is_some()
        && symbol
            .name
            .rsplit_once(".v")
            .is_some_and(|(_, version)| version.chars().all(|item| item.is_ascii_digit()))
}

fn is_generated_shape_symbol(symbol: &Symbol) -> bool {
    matches!(symbol.name.as_ref(), "args" | "result" | "syntax-shape")
        && symbol
            .namespace
            .as_deref()
            .is_some_and(|namespace| namespace.contains('/'))
}

fn allow_list_reason(reference: &Ref) -> Option<&'static str> {
    match reference {
        Ref::Content(_) | Ref::Handle(_) | Ref::Coord(_) => Some("external runtime identity"),
        Ref::Symbol(_) => None,
    }
}

use super::fields::key as field_symbol;
