use std::sync::Arc;

use sim_kernel::{Cx, Error, Expr, PreparedArgs, Result, Symbol, Value};
use sim_shape::{
    AndShape, Bindings, ListShape, NotShape, OrShape, RepeatShape, Shape, TableExtraPolicy,
    TableFieldSpec, TableShape,
};

use crate::shapes::shape_ref_arc;

use super::{build_shape, value_list_items, value_to_symbol};

pub(super) fn shape_and_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let shapes_value = exact_arg(prepared, 1, "shape:and")?.clone();
    let parts = shape_list(cx, shapes_value.clone())?;
    build_runtime_shape(
        "and",
        Arc::new(AndShape::new(parts)),
        vec![shapes_value.object().as_expr(cx)?],
    )
}

pub(super) fn shape_or_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let shapes_value = exact_arg(prepared, 1, "shape:or")?.clone();
    let choices = shape_list(cx, shapes_value.clone())?;
    build_runtime_shape(
        "or",
        Arc::new(OrShape::new(choices)),
        vec![shapes_value.object().as_expr(cx)?],
    )
}

pub(super) fn shape_not_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let inner_value = exact_arg(prepared, 1, "shape:not")?.clone();
    let inner = shape_ref_arc(&inner_value)?;
    build_runtime_shape(
        "not",
        Arc::new(NotShape::new(inner)),
        vec![inner_value.object().as_expr(cx)?],
    )
}

pub(super) fn shape_list_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let shapes_value = exact_arg(prepared, 1, "shape:list")?.clone();
    let items = shape_list(cx, shapes_value.clone())?;
    build_runtime_shape(
        "list",
        Arc::new(ListShape::new(items)),
        vec![shapes_value.object().as_expr(cx)?],
    )
}

pub(super) fn shape_list_rest_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 2, "shape:list-rest")?;
    let prefix_value = prepared.get(0).unwrap().clone();
    let rest_value = prepared.get(1).unwrap().clone();
    let prefix = shape_list(cx, prefix_value.clone())?;
    let rest = shape_ref_arc(&rest_value)?;
    build_runtime_shape(
        "list-rest",
        Arc::new(ListShape::with_rest(prefix, rest)),
        vec![
            prefix_value.object().as_expr(cx)?,
            rest_value.object().as_expr(cx)?,
        ],
    )
}

pub(super) fn shape_table_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 2, "shape:table")?;
    let key_value = prepared.get(0).unwrap().clone();
    let shape_value = prepared.get(1).unwrap().clone();
    let key = value_to_symbol(cx, key_value.clone())?;
    let shape = shape_ref_arc(&shape_value)?;
    build_runtime_shape(
        "table",
        Arc::new(TableShape::single(key.clone(), shape)),
        vec![Expr::Symbol(key), shape_value.object().as_expr(cx)?],
    )
}

pub(super) fn shape_table_open_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    table_with_extra(cx, prepared, "table-open", TableExtraPolicy::Allow)
}

pub(super) fn shape_table_required_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    table_with_extra(cx, prepared, "table-required", TableExtraPolicy::Allow)
}

pub(super) fn shape_table_closed_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    table_with_extra(cx, prepared, "table-closed", TableExtraPolicy::Reject)
}

fn table_with_extra(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    name: &str,
    extra: TableExtraPolicy,
) -> Result<Value> {
    let fields_value = exact_arg(prepared, 1, &format!("shape:{name}"))?.clone();
    let fields = table_fields(cx, fields_value.clone())?;
    build_runtime_shape(
        name,
        Arc::new(TableShape::new(fields, extra)),
        vec![fields_value.object().as_expr(cx)?],
    )
}

pub(super) fn shape_repeat_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let body_value = exact_arg(prepared, 1, "shape:repeat")?.clone();
    let body = shape_ref_arc(&body_value)?;
    build_runtime_shape(
        "repeat",
        Arc::new(RepeatShape::new(body)),
        vec![body_value.object().as_expr(cx)?],
    )
}

pub(super) fn shape_repeat_bounds_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 3, "shape:repeat-bounds")?;
    let body_value = prepared.get(0).unwrap().clone();
    let min_value = prepared.get(1).unwrap().clone();
    let max_value = prepared.get(2).unwrap().clone();
    let body = shape_ref_arc(&body_value)?;
    let min = value_to_usize(cx, min_value.clone(), "shape:repeat-bounds min")?;
    let max = value_to_optional_usize(cx, max_value.clone(), "shape:repeat-bounds max")?;
    if matches!(max, Some(max) if max < min) {
        return Err(Error::Eval(
            "shape:repeat-bounds max must be greater than or equal to min".to_owned(),
        ));
    }
    build_runtime_shape(
        "repeat-bounds",
        Arc::new(RepeatShape::with_bounds(body, min, max)),
        vec![
            body_value.object().as_expr(cx)?,
            min_value.object().as_expr(cx)?,
            max_value.object().as_expr(cx)?,
        ],
    )
}

pub(super) fn shape_without_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 2, "shape:without")?;
    let left_value = prepared.get(0).unwrap().clone();
    let right_value = prepared.get(1).unwrap().clone();
    let left = shape_ref_arc(&left_value)?;
    let right = shape_ref_arc(&right_value)?;
    build_runtime_shape(
        "without",
        Arc::new(AndShape::new(vec![left, Arc::new(NotShape::new(right))])),
        vec![
            left_value.object().as_expr(cx)?,
            right_value.object().as_expr(cx)?,
        ],
    )
}

fn build_runtime_shape(name: &str, shape: Arc<dyn Shape>, args: Vec<Expr>) -> Result<Value> {
    let symbol = Symbol::qualified("shape", name);
    Ok(build_shape(symbol, shape, args))
}

fn exact_arg<'a>(prepared: &'a PreparedArgs, expected: usize, name: &str) -> Result<&'a Value> {
    exact_len(prepared, expected, name)?;
    prepared
        .get(0)
        .ok_or_else(|| Error::Eval(format!("{name} missing argument")))
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

fn shape_list(cx: &mut Cx, value: Value) -> Result<Vec<Arc<dyn Shape>>> {
    value_list_items(cx, value)?
        .into_iter()
        .map(|item| shape_ref_arc(&item))
        .collect()
}

fn table_fields(cx: &mut Cx, value: Value) -> Result<Vec<TableFieldSpec>> {
    value_list_items(cx, value)?
        .into_iter()
        .map(|entry| {
            let parts = value_list_items(cx, entry)?;
            let [key_value, shape_value] = parts.as_slice() else {
                return Err(Error::Eval(
                    "shape:table field must be a two-item list".to_owned(),
                ));
            };
            Ok(TableFieldSpec {
                key: value_to_symbol(cx, key_value.clone())?,
                shape: shape_ref_arc(shape_value)?,
                required: true,
            })
        })
        .collect()
}

fn value_to_optional_usize(cx: &mut Cx, value: Value, context: &str) -> Result<Option<usize>> {
    if matches!(value.object().as_expr(cx)?, Expr::Nil) {
        Ok(None)
    } else {
        value_to_usize(cx, value, context).map(Some)
    }
}

fn value_to_usize(cx: &mut Cx, value: Value, context: &str) -> Result<usize> {
    let expr = value.object().as_expr(cx)?;
    let Expr::Number(number) = expr else {
        return Err(Error::Eval(format!("{context} expects a number")));
    };
    number
        .canonical
        .parse::<usize>()
        .map_err(|_| Error::Eval(format!("{context} expects a non-negative integer")))
}
