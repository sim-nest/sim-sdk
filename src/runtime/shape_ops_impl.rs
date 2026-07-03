use std::sync::Arc;

#[path = "shape_ops_impl/algebra.rs"]
mod algebra;
#[path = "shape_ops_impl/compare.rs"]
mod compare;
#[path = "shape_ops_impl/hooks.rs"]
mod hooks;
#[path = "shape_ops_impl/spec.rs"]
mod spec;

use sim_kernel::{
    Cx, Error, Expr, ExprKind, ObjectEncoding, PreparedArgs, Result, ShapeMatchObject, Symbol,
    Value, class_is_subclass_of, shape_is_subshape_of,
};
use sim_shape::{
    AnyShape, Bindings, CaptureShape, ClassShape, EffectfulShape, ExactExprShape, ExprKindShape,
    FieldShape, FieldSpec, ListShape, OneOfShape, Shape,
};

use super::shape_class_symbol;
use crate::shapes::{
    check_shape_expr, check_shape_value, shape_ref_arc, shape_ref_as_shape, shape_ref_id,
    shape_value_with_encoding,
};

pub(super) fn shape_helper_spec(
    namespace: &str,
    name: &str,
) -> (Vec<sim_kernel::Demand>, sim_shape::NativeFunctionImpl) {
    spec::shape_helper_spec(namespace, name)
}

pub(super) fn shape_number(cx: &mut Cx, value: impl ToString) -> Result<Value> {
    cx.factory()
        .number_literal(Symbol::qualified("numbers", "f64"), value.to_string())
}
fn shape_match(value: &Value) -> Result<&ShapeMatchObject> {
    value
        .object()
        .downcast_ref::<ShapeMatchObject>()
        .ok_or(Error::TypeMismatch {
            expected: "shape-match",
            found: "non-shape-match",
        })
}
fn build_shape(symbol: Symbol, shape: Arc<dyn Shape>, args: Vec<Expr>) -> Value {
    shape_value_with_encoding(
        symbol.clone(),
        shape,
        ObjectEncoding::Constructor {
            class: symbol,
            args,
        },
    )
}
fn value_to_expr_kind(cx: &mut Cx, value: Value) -> Result<ExprKind> {
    match value.object().as_expr(cx)? {
        Expr::Symbol(symbol) => expr_kind_name(symbol.to_string().as_str()),
        Expr::String(text) => expr_kind_name(text.as_str()),
        _ => Err(Error::Eval("expr kind expects symbol or string".to_owned())),
    }
}
fn expr_kind_name(name: &str) -> Result<ExprKind> {
    match name {
        "nil" => Ok(ExprKind::Nil),
        "bool" => Ok(ExprKind::Bool),
        "number" => Ok(ExprKind::Number),
        "symbol" => Ok(ExprKind::Symbol),
        "string" => Ok(ExprKind::String),
        "bytes" => Ok(ExprKind::Bytes),
        "list" => Ok(ExprKind::List),
        "vector" => Ok(ExprKind::Vector),
        "map" => Ok(ExprKind::Map),
        "set" => Ok(ExprKind::Set),
        "call" => Ok(ExprKind::Call),
        "infix" => Ok(ExprKind::Infix),
        "prefix" => Ok(ExprKind::Prefix),
        "postfix" => Ok(ExprKind::Postfix),
        "block" => Ok(ExprKind::Block),
        "quote" => Ok(ExprKind::Quote),
        "annotated" => Ok(ExprKind::Annotated),
        "extension" => Ok(ExprKind::Extension),
        other => Err(Error::Eval(format!("unknown expr kind {other}"))),
    }
}
fn value_to_symbol(cx: &mut Cx, value: Value) -> Result<Symbol> {
    if let Some(class) = value.object().as_class() {
        return Ok(class.symbol());
    }
    match value.object().as_expr(cx)? {
        Expr::Symbol(symbol) => Ok(symbol),
        Expr::String(text) => Ok(Symbol::new(text)),
        _ => Err(Error::Eval("expected symbol, string, or class".to_owned())),
    }
}
fn value_list_items(cx: &mut Cx, value: Value) -> Result<Vec<Value>> {
    let Some(_list) = value.object().as_list() else {
        return Err(Error::TypeMismatch {
            expected: "list",
            found: "non-list",
        });
    };
    let mut items = Vec::new();
    let mut current = Some(value);
    while let Some(node_value) = current {
        let Some(node) = node_value.object().as_list() else {
            break;
        };
        if node.is_empty(cx)? {
            break;
        }
        if let Some(item) = node.car(cx)? {
            items.push(item);
        }
        current = node.cdr(cx)?;
    }
    Ok(items)
}
pub(super) fn any_shape_impl(
    _cx: &mut Cx,
    _prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    Ok(build_shape(
        shape_class_symbol("AnyShape"),
        Arc::new(AnyShape),
        Vec::new(),
    ))
}

pub(super) fn expr_kind_shape_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let value = prepared
        .get(0)
        .cloned()
        .ok_or_else(|| Error::Eval("missing expr kind".to_owned()))?;
    let kind = value_to_expr_kind(cx, value.clone())?;
    Ok(build_shape(
        shape_class_symbol("ExprKindShape"),
        Arc::new(ExprKindShape::new(kind)),
        vec![value.object().as_expr(cx)?],
    ))
}

pub(super) fn class_shape_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let value = prepared
        .get(0)
        .cloned()
        .ok_or_else(|| Error::Eval("missing class".to_owned()))?;
    let symbol = value_to_symbol(cx, value.clone())?;
    Ok(build_shape(
        shape_class_symbol("ClassShape"),
        Arc::new(ClassShape::new(symbol.clone())),
        vec![Expr::Symbol(symbol)],
    ))
}

pub(super) fn exact_expr_shape_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let expr = prepared
        .get(0)
        .ok_or_else(|| Error::Eval("missing expression".to_owned()))?
        .object()
        .as_expr(cx)?;
    Ok(build_shape(
        shape_class_symbol("ExactExprShape"),
        Arc::new(ExactExprShape::new(expr.clone())),
        vec![expr],
    ))
}

pub(super) fn list_shape_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let value = prepared
        .get(0)
        .cloned()
        .ok_or_else(|| Error::Eval("missing shape list".to_owned()))?;
    let items = value_list_items(cx, value.clone())?
        .into_iter()
        .map(|item| shape_ref_arc(&item))
        .collect::<Result<Vec<_>>>()?;
    Ok(build_shape(
        shape_class_symbol("ListShape"),
        Arc::new(ListShape::new(items)),
        vec![value.object().as_expr(cx)?],
    ))
}

pub(super) fn capture_shape_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let name_value = prepared
        .get(0)
        .cloned()
        .ok_or_else(|| Error::Eval("missing capture name".to_owned()))?;
    let inner_value = prepared
        .get(1)
        .cloned()
        .ok_or_else(|| Error::Eval("missing capture shape".to_owned()))?;
    let name = value_to_symbol(cx, name_value.clone())?;
    let inner = shape_ref_arc(&inner_value)?;
    Ok(build_shape(
        shape_class_symbol("CaptureShape"),
        Arc::new(CaptureShape::new(name.clone(), inner)),
        vec![Expr::Symbol(name), inner_value.object().as_expr(cx)?],
    ))
}

pub(super) fn one_of_shape_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let value = prepared
        .get(0)
        .cloned()
        .ok_or_else(|| Error::Eval("missing shape choices".to_owned()))?;
    let choices = value_list_items(cx, value.clone())?
        .into_iter()
        .map(|item| shape_ref_arc(&item))
        .collect::<Result<Vec<_>>>()?;
    Ok(build_shape(
        shape_class_symbol("OneOfShape"),
        Arc::new(OneOfShape::new(choices)),
        vec![value.object().as_expr(cx)?],
    ))
}

pub(super) fn field_shape_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let class_value = prepared
        .get(0)
        .cloned()
        .ok_or_else(|| Error::Eval("missing field-shape class".to_owned()))?;
    let fields_value = prepared
        .get(1)
        .cloned()
        .ok_or_else(|| Error::Eval("missing field specs".to_owned()))?;
    let class = value_to_symbol(cx, class_value.clone())?;
    let mut fields = Vec::new();
    for entry in value_list_items(cx, fields_value.clone())? {
        let parts = value_list_items(cx, entry)?;
        let [name_value, shape_value] = parts.as_slice() else {
            return Err(Error::Eval("field spec must be a two-item list".to_owned()));
        };
        fields.push(FieldSpec::required(
            value_to_symbol(cx, name_value.clone())?,
            shape_ref_arc(shape_value)?,
        ));
    }
    Ok(build_shape(
        shape_class_symbol("FieldShape"),
        Arc::new(FieldShape::new(class.clone(), fields)),
        vec![Expr::Symbol(class), fields_value.object().as_expr(cx)?],
    ))
}

pub(super) fn effectful_shape_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let inner_value = prepared
        .get(0)
        .cloned()
        .ok_or_else(|| Error::Eval("missing inner shape".to_owned()))?;
    let inner = shape_ref_arc(&inner_value)?;
    Ok(build_shape(
        shape_class_symbol("EffectfulShape"),
        Arc::new(EffectfulShape::new(inner)),
        vec![inner_value.object().as_expr(cx)?],
    ))
}

pub(super) fn class_subclass_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let left = prepared
        .get(0)
        .ok_or_else(|| Error::Eval("missing subclass".to_owned()))?;
    let right = prepared
        .get(1)
        .ok_or_else(|| Error::Eval("missing parent".to_owned()))?;
    let Some(left) = left.object().as_class() else {
        return Err(Error::TypeMismatch {
            expected: "class",
            found: "non-class",
        });
    };
    let result = class_is_subclass_of(cx, left, right.clone())?;
    cx.factory().bool(result)
}

pub(super) fn shape_subshape_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let child = shape_ref_as_shape(
        prepared
            .get(0)
            .ok_or_else(|| Error::Eval("missing child shape".to_owned()))?,
    )?;
    let parent = shape_ref_as_shape(
        prepared
            .get(1)
            .ok_or_else(|| Error::Eval("missing parent shape".to_owned()))?,
    )?;
    let result = shape_is_subshape_of(cx, child, parent)?;
    cx.factory().bool(result)
}

pub(super) fn shape_parents_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let shape = prepared
        .get(0)
        .ok_or_else(|| Error::Eval("missing shape".to_owned()))?;
    let parents = shape_ref_as_shape(shape)?.parents(cx)?;
    cx.factory().list(parents)
}

pub(super) fn shape_check_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    check_shape_value(
        cx,
        prepared.get(0).unwrap(),
        prepared.get(1).unwrap().clone(),
    )
    .and_then(|matched| sim_kernel::shape_match_value(cx, matched))
}

pub(super) fn shape_check_expr_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let expr = prepared.get(1).unwrap().object().as_expr(cx)?;
    check_shape_expr(cx, prepared.get(0).unwrap(), &expr)
        .and_then(|matched| sim_kernel::shape_match_value(cx, matched))
}

pub(super) fn shape_assert_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let shape = prepared.get(0).unwrap();
    let value = prepared.get(1).unwrap().clone();
    let matched = check_shape_value(cx, shape, value.clone())?;
    if matched.accepted {
        Ok(value)
    } else {
        Err(Error::WrongShape {
            expected: shape_ref_id(shape),
            diagnostics: matched.diagnostics,
        })
    }
}

pub(super) fn shape_match_accepted_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    cx.factory()
        .bool(shape_match(prepared.get(0).unwrap())?.matched().accepted)
}

pub(super) fn shape_match_rejected_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    cx.factory()
        .bool(!shape_match(prepared.get(0).unwrap())?.matched().accepted)
}

pub(super) fn shape_match_score_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    shape_number(
        cx,
        shape_match(prepared.get(0).unwrap())?
            .matched()
            .score
            .value(),
    )
}

pub(super) fn shape_match_value_captures_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    cx.factory().table(
        shape_match(prepared.get(0).unwrap())?
            .matched()
            .captures
            .values()
            .to_vec(),
    )
}

pub(super) fn shape_match_expr_captures_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let entries = shape_match(prepared.get(0).unwrap())?
        .matched()
        .captures
        .exprs()
        .iter()
        .map(|(name, expr)| Ok((name.clone(), cx.factory().expr(expr.clone())?)))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().table(entries)
}

pub(super) fn shape_match_diagnostics_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let items = shape_match(prepared.get(0).unwrap())?
        .matched()
        .diagnostics
        .iter()
        .map(|diagnostic| cx.factory().string(diagnostic.message.clone()))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(items)
}
