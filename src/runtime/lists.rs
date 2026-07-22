use std::{cmp::Ordering, sync::Arc};

use sim_kernel::{Cx, Demand, Error, Expr, FunctionId, PreparedArgs, Result, Symbol, Value};
use sim_shape::{AnyShape, Bindings, CaptureShape, FunctionCase, FunctionObject, ListShape};

use super::config_list_impl_capability;

fn variadic_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
    implementation: fn(&mut Cx, &PreparedArgs, Bindings) -> Result<Value>,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "many"),
            args: Arc::new(AnyShape),
            result: Some(Arc::new(AnyShape)),
            demand: Vec::new(),
            priority: 10,
            implementation,
        }],
    )
}

fn unary_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
    implementation: fn(&mut Cx, &PreparedArgs, Bindings) -> Result<Value>,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "one"),
            args: Arc::new(ListShape::new(vec![Arc::new(CaptureShape::new(
                Symbol::new("list"),
                Arc::new(AnyShape),
            ))])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![Demand::Value],
            priority: 10,
            implementation,
        }],
    )
}

fn binary_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
    implementation: fn(&mut Cx, &PreparedArgs, Bindings) -> Result<Value>,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "two"),
            args: Arc::new(ListShape::new(vec![
                Arc::new(CaptureShape::new(Symbol::new("left"), Arc::new(AnyShape))),
                Arc::new(CaptureShape::new(Symbol::new("right"), Arc::new(AnyShape))),
            ])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![Demand::Value, Demand::Value],
            priority: 10,
            implementation,
        }],
    )
}

pub(crate) fn list_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    variadic_function(case_id, function_id, symbol, list_impl)
}

pub(crate) fn cons_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, cons_impl)
}

pub(crate) fn car_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_function(case_id, function_id, symbol, car_impl)
}

pub(crate) fn cdr_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_function(case_id, function_id, symbol, cdr_impl)
}

pub(crate) fn head_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_function(case_id, function_id, symbol, car_impl)
}

pub(crate) fn tail_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_function(case_id, function_id, symbol, cdr_impl)
}

pub(crate) fn empty_list_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_function(case_id, function_id, symbol, empty_impl)
}

pub(crate) fn nth_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, nth_impl)
}

pub(crate) fn len_cmp_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, len_cmp_impl)
}

pub(crate) fn len_lt_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, len_lt_impl)
}

pub(crate) fn len_lte_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, len_lte_impl)
}

pub(crate) fn len_eq_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, len_eq_impl)
}

pub(crate) fn len_gte_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, len_gte_impl)
}

pub(crate) fn len_gt_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, len_gt_impl)
}

pub(crate) fn take_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, take_impl)
}

pub(crate) fn drop_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_function(case_id, function_id, symbol, drop_impl)
}

pub(crate) fn list_impl_function(
    zero_case_id: sim_kernel::CaseId,
    one_case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![
            FunctionCase {
                id: zero_case_id,
                name: Symbol::qualified(symbol.to_string(), "zero"),
                args: Arc::new(ListShape::new(Vec::new())),
                result: Some(Arc::new(AnyShape)),
                demand: Vec::new(),
                priority: 10,
                implementation: list_impl_impl,
            },
            FunctionCase {
                id: one_case_id,
                name: Symbol::qualified(symbol.to_string(), "one"),
                args: Arc::new(ListShape::new(vec![Arc::new(CaptureShape::new(
                    Symbol::new("subject"),
                    Arc::new(AnyShape),
                ))])),
                result: Some(Arc::new(AnyShape)),
                demand: vec![Demand::Value],
                priority: 20,
                implementation: list_impl_impl,
            },
        ],
    )
}

fn list_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    cx.new_list(prepared.values().to_vec())
}

fn cons_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let car = required_arg(prepared, 0, "cons expects two arguments")?;
    let cdr = required_arg(prepared, 1, "cons expects two arguments")?;
    cx.new_cons(car, cdr)
}

fn car_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let list = required_list_arg(cx, prepared, 0, "car expects one list")?;
    match list.car(cx)? {
        Some(value) => Ok(value),
        None => cx.factory().nil(),
    }
}

fn cdr_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let list = required_list_arg(cx, prepared, 0, "cdr expects one list")?;
    match list.cdr(cx)? {
        Some(value) => Ok(value),
        None => cx.factory().nil(),
    }
}

fn empty_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let list = required_list_arg(cx, prepared, 0, "empty? expects one list")?;
    let is_empty = list.is_empty(cx)?;
    cx.factory().bool(is_empty)
}

fn nth_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let list = required_list_arg(cx, prepared, 0, "nth expects a list and an index")?;
    let index = required_index_arg(cx, prepared, 1, "nth index must be a non-negative integer")?;
    match list.get(cx, index)? {
        Some(value) => Ok(value),
        None => cx.factory().nil(),
    }
}

fn len_cmp_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let list = required_list_arg(cx, prepared, 0, "len-cmp expects a list and an index")?;
    let index = required_index_arg(
        cx,
        prepared,
        1,
        "len-cmp index must be a non-negative integer",
    )?;
    let ordering = list.len_cmp(cx, index)?;
    cx.factory().symbol(ordering_symbol(ordering))
}

fn len_lt_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    len_predicate_impl(
        cx,
        prepared,
        Ordering::Less,
        "len< expects a list and an index",
    )
}

fn len_lte_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let ordering = list_len_cmp(cx, prepared, "len<= expects a list and an index")?;
    cx.factory()
        .bool(matches!(ordering, Ordering::Less | Ordering::Equal))
}

fn len_eq_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    len_predicate_impl(
        cx,
        prepared,
        Ordering::Equal,
        "len= expects a list and an index",
    )
}

fn len_gte_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let ordering = list_len_cmp(cx, prepared, "len>= expects a list and an index")?;
    cx.factory()
        .bool(matches!(ordering, Ordering::Equal | Ordering::Greater))
}

fn len_gt_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    len_predicate_impl(
        cx,
        prepared,
        Ordering::Greater,
        "len> expects a list and an index",
    )
}

fn take_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let list = required_list_arg(cx, prepared, 0, "take expects a list and an index")?;
    let index = required_index_arg(cx, prepared, 1, "take index must be a non-negative integer")?;
    let items = list.to_vec(cx, Some(index))?;
    cx.new_list(items)
}

fn drop_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let mut current = required_arg(prepared, 0, "drop expects a list and an index")?.clone();
    if current.object().as_list().is_none() {
        return Err(Error::TypeMismatch {
            expected: "list",
            found: value_kind(cx, &current)?,
        });
    }
    let index = required_index_arg(cx, prepared, 1, "drop index must be a non-negative integer")?;
    for _ in 0..index {
        let Some(list) = current.object().as_list() else {
            return Err(Error::Eval("list cdr did not yield a list".to_owned()));
        };
        match list.cdr(cx)? {
            Some(next) => current = next,
            None => return cx.new_list(Vec::new()),
        }
    }
    Ok(current)
}

fn list_impl_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    if prepared.is_empty() {
        return cx
            .factory()
            .symbol(Symbol::new(cx.list_registry().active().to_owned()));
    }

    cx.require(&config_list_impl_capability())?;
    let value = required_arg(prepared, 0, "list-impl expects zero or one symbol")?;
    let name = value_to_symbol_name(cx, &value)?;
    cx.list_registry_mut().set_active(&name)?;
    cx.factory().symbol(Symbol::new(name))
}

fn required_arg(prepared: &PreparedArgs, index: usize, message: &str) -> Result<Value> {
    prepared
        .get(index)
        .cloned()
        .ok_or_else(|| Error::Eval(message.to_owned()))
}

fn required_list_arg<'a>(
    cx: &mut Cx,
    prepared: &'a PreparedArgs,
    index: usize,
    message: &str,
) -> Result<&'a dyn sim_kernel::ListValue> {
    let value = prepared
        .get(index)
        .ok_or_else(|| Error::Eval(message.to_owned()))?;
    value.object().as_list().ok_or(Error::TypeMismatch {
        expected: "list",
        found: value_kind(cx, value)?,
    })
}

fn required_index_arg(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    index: usize,
    message: &str,
) -> Result<usize> {
    let value = required_arg(prepared, index, message)?;
    let expr = value.object().as_expr(cx)?;
    let Expr::Number(number) = expr else {
        return Err(Error::TypeMismatch {
            expected: "number",
            found: value_kind(cx, &value)?,
        });
    };
    number
        .canonical
        .parse::<usize>()
        .map_err(|_| Error::Eval(message.to_owned()))
}

fn value_to_symbol_name(cx: &mut Cx, value: &Value) -> Result<String> {
    match value.object().as_expr(cx)? {
        Expr::Symbol(symbol) => Ok(symbol.to_string()),
        Expr::String(text) => Ok(text),
        _ => Err(Error::TypeMismatch {
            expected: "symbol",
            found: value_kind(cx, value)?,
        }),
    }
}

fn list_len_cmp(cx: &mut Cx, prepared: &PreparedArgs, message: &str) -> Result<Ordering> {
    let list = required_list_arg(cx, prepared, 0, message)?;
    let index = required_index_arg(cx, prepared, 1, message)?;
    list.len_cmp(cx, index)
}

fn len_predicate_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    expected: Ordering,
    message: &str,
) -> Result<Value> {
    let matched = list_len_cmp(cx, prepared, message)? == expected;
    cx.factory().bool(matched)
}

fn ordering_symbol(ordering: Ordering) -> Symbol {
    match ordering {
        Ordering::Less => Symbol::new("lt"),
        Ordering::Equal => Symbol::new("eq"),
        Ordering::Greater => Symbol::new("gt"),
    }
}

fn value_kind(cx: &mut Cx, value: &Value) -> Result<&'static str> {
    Ok(match value.object().as_expr(cx)? {
        Expr::Nil => "nil",
        Expr::Bool(_) => "bool",
        Expr::Number(_) => "number",
        Expr::Symbol(_) => "symbol",
        Expr::Local(_) => "local",
        Expr::String(_) => "string",
        Expr::Bytes(_) => "bytes",
        Expr::List(_) => "list",
        Expr::Vector(_) => "vector",
        Expr::Map(_) => "map",
        Expr::Set(_) => "set",
        Expr::Call { .. } => "call",
        Expr::Infix { .. } => "infix",
        Expr::Prefix { .. } => "prefix",
        Expr::Postfix { .. } => "postfix",
        Expr::Block(_) => "block",
        Expr::Quote { .. } => "quote",
        Expr::Annotated { .. } => "annotated",
        Expr::Extension { .. } => "extension",
    })
}
