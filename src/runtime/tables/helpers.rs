use sim_kernel::{Cx, Error, Expr, PreparedArgs, Result, Symbol, Value};

pub(super) fn table_entries_from_pairs(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    lazy: bool,
) -> Result<Vec<(Symbol, Value)>> {
    if !prepared.len().is_multiple_of(2) {
        let name = if lazy { "table/lazy" } else { "table" };
        return Err(Error::Eval(format!(
            "{name} expects alternating symbol and value pairs"
        )));
    }

    let mut entries = Vec::with_capacity(prepared.len() / 2);
    for pair in prepared.values().chunks(2) {
        let key = value_to_symbol(cx, &pair[0])?;
        entries.push((key, pair[1].clone()));
    }
    Ok(entries)
}

pub(super) fn required_arg(prepared: &PreparedArgs, index: usize, message: &str) -> Result<Value> {
    prepared
        .get(index)
        .cloned()
        .ok_or_else(|| Error::Eval(message.to_owned()))
}

pub(super) fn required_table_arg<'a>(
    cx: &mut Cx,
    prepared: &'a PreparedArgs,
    index: usize,
    message: &str,
) -> Result<&'a dyn sim_kernel::Table> {
    let value = prepared
        .get(index)
        .ok_or_else(|| Error::Eval(message.to_owned()))?;
    value.object().as_table_impl().ok_or_else(|| {
        Error::Eval(format!(
            "{}; found {}",
            message,
            value_kind(cx, value).unwrap_or("unknown")
        ))
    })
}

pub(super) fn required_dir_arg<'a>(
    cx: &mut Cx,
    prepared: &'a PreparedArgs,
    index: usize,
) -> Result<&'a dyn sim_kernel::table::Dir> {
    let value = prepared
        .get(index)
        .ok_or_else(|| Error::Eval("missing directory table argument".to_owned()))?;
    value.object().as_dir().ok_or_else(|| {
        if value.object().as_table_impl().is_some() {
            Error::Eval("this table backend does not support directories".to_owned())
        } else {
            Error::Eval(format!(
                "expected a directory table; found {}",
                value_kind(cx, value).unwrap_or("unknown")
            ))
        }
    })
}

pub(super) fn required_symbol_arg(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    index: usize,
    message: &str,
) -> Result<Symbol> {
    value_to_symbol(cx, &required_arg(prepared, index, message)?)
}

#[cfg(feature = "table-fs")]
pub(super) fn required_string_arg(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    index: usize,
    message: &str,
) -> Result<String> {
    let value = required_arg(prepared, index, message)?;
    match value.object().as_expr(cx)? {
        Expr::String(text) => Ok(text),
        _ => Err(Error::TypeMismatch {
            expected: "string",
            found: value_kind(cx, &value)?,
        }),
    }
}

pub(super) fn value_to_symbol(cx: &mut Cx, value: &Value) -> Result<Symbol> {
    match value.object().as_expr(cx)? {
        Expr::Symbol(symbol) => Ok(symbol),
        Expr::String(text) => Ok(Symbol::new(text)),
        _ => Err(Error::TypeMismatch {
            expected: "symbol",
            found: value_kind(cx, value)?,
        }),
    }
}

pub(super) fn value_to_symbol_name(cx: &mut Cx, value: &Value) -> Result<String> {
    Ok(value_to_symbol(cx, value)?.to_string())
}

pub(super) fn number_value(cx: &mut Cx, value: usize) -> Result<Value> {
    cx.factory()
        .number_literal(Symbol::qualified("numbers", "f64"), value.to_string())
}

pub(super) fn value_kind(cx: &mut Cx, value: &Value) -> Result<&'static str> {
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
