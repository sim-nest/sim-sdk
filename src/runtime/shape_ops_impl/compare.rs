use std::sync::Arc;

use sim_kernel::{Cx, Error, Expr, PreparedArgs, Result, Symbol, Value};
use sim_shape::{
    Bindings, ShapeProbe, ShapeRelation, ShapeRelationKind, VennShapeSet, relate_shapes,
};

use crate::shapes::{shape_ref_arc, shape_ref_as_shape};

use super::{build_shape, shape_number, value_list_items, value_to_symbol};

pub(super) fn shape_compare_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 2, "shape:compare")?;
    let left = shape_ref_as_shape(prepared.get(0).unwrap())?;
    let right = shape_ref_as_shape(prepared.get(1).unwrap())?;
    let relation = relate_shapes(cx, left, right, &[])?;
    relation_value(cx, relation)
}

pub(super) fn shape_compare_with_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 3, "shape:compare-with")?;
    let probes_value = prepared.get(2).unwrap().clone();
    let probes = parse_probes(cx, probes_value)?;
    let left = shape_ref_as_shape(prepared.get(0).unwrap())?;
    let right = shape_ref_as_shape(prepared.get(1).unwrap())?;
    let relation = relate_shapes(cx, left, right, &probes)?;
    relation_value(cx, relation)
}

pub(super) fn shape_venn_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 1, "shape:venn")?;
    let members_value = prepared.get(0).unwrap().clone();
    let members = parse_members(cx, members_value)?;
    cx.factory().opaque(Arc::new(VennShapeSet::new(members)))
}

pub(super) fn shape_venn_union_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 1, "shape:venn-union")?;
    let venn_value = prepared.get(0).unwrap().clone();
    let shape = venn_ref(&venn_value)?.union();
    runtime_shape(
        "venn-union",
        shape,
        vec![venn_evidence_expr(cx, &venn_value)],
    )
}

pub(super) fn shape_venn_intersection_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 1, "shape:venn-intersection")?;
    let venn_value = prepared.get(0).unwrap().clone();
    let shape = venn_ref(&venn_value)?.intersection();
    runtime_shape(
        "venn-intersection",
        shape,
        vec![venn_evidence_expr(cx, &venn_value)],
    )
}

pub(super) fn shape_venn_only_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 2, "shape:venn-only")?;
    let venn_value = prepared.get(0).unwrap().clone();
    let name_value = prepared.get(1).unwrap().clone();
    let name = value_to_symbol(cx, name_value.clone())?;
    let shape = venn_ref(&venn_value)?.only(&name)?;
    runtime_shape(
        "venn-only",
        shape,
        vec![
            venn_evidence_expr(cx, &venn_value),
            name_value.object().as_expr(cx)?,
        ],
    )
}

pub(super) fn shape_venn_outside_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 1, "shape:venn-outside")?;
    let venn_value = prepared.get(0).unwrap().clone();
    let shape = venn_ref(&venn_value)?.outside_all();
    runtime_shape(
        "venn-outside",
        shape,
        vec![venn_evidence_expr(cx, &venn_value)],
    )
}

pub(super) fn shape_venn_exactly_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 2, "shape:venn-exactly")?;
    let venn_value = prepared.get(0).unwrap().clone();
    let names_value = prepared.get(1).unwrap().clone();
    let names = value_list_items(cx, names_value.clone())?
        .into_iter()
        .map(|value| value_to_symbol(cx, value))
        .collect::<Result<Vec<_>>>()?;
    let shape = venn_ref(&venn_value)?.exactly(&names)?;
    runtime_shape(
        "venn-exactly",
        shape,
        vec![
            venn_evidence_expr(cx, &venn_value),
            names_value.object().as_expr(cx)?,
        ],
    )
}

fn venn_evidence_expr(cx: &mut Cx, value: &Value) -> Expr {
    match value.object().as_expr(cx) {
        Ok(expr) => expr,
        Err(_) => Expr::Symbol(Symbol::qualified("shape", "venn")),
    }
}

fn relation_value(cx: &mut Cx, relation: ShapeRelation) -> Result<Value> {
    let witnesses = relation
        .witnesses
        .into_iter()
        .map(|witness| {
            cx.factory().table(vec![
                (Symbol::new("label"), cx.factory().string(witness.label)?),
                (
                    Symbol::new("accepted-left"),
                    cx.factory().bool(witness.accepted_left)?,
                ),
                (
                    Symbol::new("accepted-right"),
                    cx.factory().bool(witness.accepted_right)?,
                ),
                (Symbol::new("note"), cx.factory().string(witness.note)?),
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    let witness_count = witnesses.len();
    let diagnostics = relation
        .diagnostics
        .into_iter()
        .map(|diagnostic| cx.factory().string(diagnostic.message))
        .collect::<Result<Vec<_>>>()?;
    let kind = cx.factory().symbol(relation_kind_symbol(relation.kind))?;
    let proven = cx.factory().bool(relation.proven)?;
    let left = cx.factory().string(relation.left.label)?;
    let right = cx.factory().string(relation.right.label)?;
    let witness_count = shape_number(cx, witness_count)?;
    let witnesses = cx.factory().list(witnesses)?;
    let diagnostics = cx.factory().list(diagnostics)?;
    cx.factory().table(vec![
        (Symbol::new("kind"), kind),
        (Symbol::new("proven"), proven),
        (Symbol::new("left"), left),
        (Symbol::new("right"), right),
        (Symbol::new("witness-count"), witness_count),
        (Symbol::new("witnesses"), witnesses),
        (Symbol::new("diagnostics"), diagnostics),
    ])
}

fn relation_kind_symbol(kind: ShapeRelationKind) -> Symbol {
    let name = match kind {
        ShapeRelationKind::Equal => "equal",
        ShapeRelationKind::LeftSubshape => "left-subshape",
        ShapeRelationKind::RightSubshape => "right-subshape",
        ShapeRelationKind::Overlap => "overlap",
        ShapeRelationKind::Disjoint => "disjoint",
        ShapeRelationKind::Unknown => "unknown",
    };
    Symbol::qualified("shape", name)
}

fn parse_probes(cx: &mut Cx, value: Value) -> Result<Vec<ShapeProbe>> {
    value_list_items(cx, value)?
        .into_iter()
        .map(|entry| {
            let parts = value_list_items(cx, entry)?;
            match parts.as_slice() {
                [label, value] => Ok(ShapeProbe::Value {
                    label: label_string(cx, label.clone())?,
                    value: value.clone(),
                }),
                [tag, label, value] => {
                    let tag = value_to_symbol(cx, tag.clone())?;
                    match tag.name.as_ref() {
                        "value" => Ok(ShapeProbe::Value {
                            label: label_string(cx, label.clone())?,
                            value: value.clone(),
                        }),
                        "expr" => Ok(ShapeProbe::Expr {
                            label: label_string(cx, label.clone())?,
                            expr: value.object().as_expr(cx)?,
                        }),
                        other => Err(Error::Eval(format!(
                            "shape:compare-with unknown probe kind {other}"
                        ))),
                    }
                }
                _ => Err(Error::Eval(
                    "shape:compare-with probes must be (label value) or (kind label value)"
                        .to_owned(),
                )),
            }
        })
        .collect()
}

fn parse_members(cx: &mut Cx, value: Value) -> Result<Vec<(Symbol, Arc<dyn sim_shape::Shape>)>> {
    value_list_items(cx, value)?
        .into_iter()
        .map(|entry| {
            let parts = value_list_items(cx, entry)?;
            let [name_value, shape_value] = parts.as_slice() else {
                return Err(Error::Eval(
                    "shape:venn member must be a two-item list".to_owned(),
                ));
            };
            Ok((
                value_to_symbol(cx, name_value.clone())?,
                shape_ref_arc(shape_value)?,
            ))
        })
        .collect()
}

fn label_string(cx: &mut Cx, value: Value) -> Result<String> {
    Ok(match value.object().as_expr(cx)? {
        Expr::Symbol(symbol) => symbol.to_string(),
        Expr::String(text) => text,
        other => format!("{other:?}"),
    })
}

fn venn_ref(value: &Value) -> Result<&VennShapeSet> {
    value
        .object()
        .downcast_ref::<VennShapeSet>()
        .ok_or(Error::TypeMismatch {
            expected: "shape-venn",
            found: "non-shape-venn",
        })
}

fn runtime_shape(name: &str, shape: Arc<dyn sim_shape::Shape>, args: Vec<Expr>) -> Result<Value> {
    Ok(build_shape(Symbol::qualified("shape", name), shape, args))
}

fn exact_len(prepared: &PreparedArgs, expected: usize, name: &str) -> Result<()> {
    if prepared.len() == expected {
        Ok(())
    } else {
        Err(Error::Eval(format!(
            "{name} expects {expected} argument(s), got {}",
            prepared.len()
        )))
    }
}
