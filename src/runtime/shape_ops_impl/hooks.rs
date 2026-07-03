use std::sync::Arc;

use sim_kernel::{Cx, Error, Expr, PreparedArgs, Result, Symbol, Value};
use sim_shape::{
    AcceptOnNoDiagnosticsHook, Bindings, DiscardOnDiagnosticPrefixHook, HookedShape, MatchHook,
    ScoreFloorHook, Shape, TraceMarkHook, hook_ref_arc, hook_value,
};

use crate::shapes::shape_ref_arc;

use super::{build_shape, value_list_items};

pub(super) fn shape_hook_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 2, "shape:hook")?;
    let inner_value = prepared.get(0).unwrap().clone();
    let hooks_value = prepared.get(1).unwrap().clone();
    let inner = shape_ref_arc(&inner_value)?;
    let hooks = hook_list(cx, hooks_value.clone())?;
    build_runtime_shape(
        "hook",
        Arc::new(HookedShape::new(inner, hooks)),
        vec![
            inner_value.object().as_expr(cx)?,
            hooks_value.object().as_expr(cx)?,
        ],
    )
}

pub(super) fn shape_hook_trace_impl(
    _cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 0, "shape:hook-trace")?;
    Ok(hook_value(Arc::new(TraceMarkHook)))
}

pub(super) fn shape_hook_score_floor_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 1, "shape:hook-score-floor")?;
    let floor = value_to_i32(
        cx,
        prepared.get(0).unwrap().clone(),
        "shape:hook-score-floor",
    )?;
    Ok(hook_value(Arc::new(ScoreFloorHook::new(floor))))
}

pub(super) fn shape_hook_accept_on_no_diagnostics_impl(
    _cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 0, "shape:hook-accept-on-no-diagnostics")?;
    Ok(hook_value(Arc::new(AcceptOnNoDiagnosticsHook)))
}

pub(super) fn shape_hook_discard_on_diagnostic_prefix_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    exact_len(prepared, 1, "shape:hook-discard-on-diagnostic-prefix")?;
    let prefix = value_to_string(cx, prepared.get(0).unwrap().clone())?;
    Ok(hook_value(Arc::new(DiscardOnDiagnosticPrefixHook::new(
        prefix,
    ))))
}

fn build_runtime_shape(name: &str, shape: Arc<dyn Shape>, args: Vec<Expr>) -> Result<Value> {
    Ok(build_shape(Symbol::qualified("shape", name), shape, args))
}

fn hook_list(cx: &mut Cx, value: Value) -> Result<Vec<Arc<dyn MatchHook>>> {
    value_list_items(cx, value)?
        .into_iter()
        .map(|item| hook_ref_arc(&item))
        .collect()
}

fn value_to_i32(cx: &mut Cx, value: Value, context: &str) -> Result<i32> {
    let Expr::Number(number) = value.object().as_expr(cx)? else {
        return Err(Error::Eval(format!("{context} expects a number")));
    };
    number
        .canonical
        .parse::<i32>()
        .map_err(|_| Error::Eval(format!("{context} expects an integer")))
}

fn value_to_string(cx: &mut Cx, value: Value) -> Result<String> {
    Ok(match value.object().as_expr(cx)? {
        Expr::String(text) => text,
        Expr::Symbol(symbol) => symbol.to_string(),
        _ => {
            return Err(Error::Eval(
                "shape:hook-discard-on-diagnostic-prefix expects string or symbol".to_owned(),
            ));
        }
    })
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
