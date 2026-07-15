#[cfg(feature = "table-fs")]
use sim_kernel::Expr;
use sim_kernel::{Cx, Error, PreparedArgs, Result, Symbol, Value};
use sim_shape::Bindings;

use super::super::config_table_impl_capability;

#[cfg(feature = "table-remote")]
use sim_lib_server::Connection;
#[cfg(feature = "table-remote")]
use sim_table_remote::remote_dir_value;

#[cfg(feature = "table-db")]
use sim_table_db::install_db_dir_lib;
#[cfg(feature = "table-fs")]
use sim_table_fs::{FindGlobResult, FindGrepResult, FindMatch, FsDir, install_fs_dir_lib};
#[cfg(feature = "table-hash")]
use sim_table_hash::HashTable;
#[cfg(feature = "table-lazy")]
use sim_table_lazy::{LazyTable, ValueLoader};

#[cfg(feature = "table-fs")]
use super::helpers::required_string_arg;
#[cfg(feature = "table-fs")]
use super::helpers::value_kind;
use super::helpers::{
    number_value, required_arg, required_dir_arg, required_symbol_arg, required_table_arg,
    table_entries_from_pairs, value_to_symbol_name,
};

pub(super) fn table_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let entries = table_entries_from_pairs(cx, prepared, false)?;
    cx.new_table(entries)
}

pub(super) fn table_catalog_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    use std::sync::Arc;

    let entries = table_entries_from_pairs(cx, prepared, false)?;
    cx.factory()
        .opaque(Arc::new(sim_kernel::catalog::CatalogTable::with_entries(
            entries,
        )?))
}

#[cfg(feature = "table-hash")]
pub(super) fn table_hash_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    use std::sync::Arc;

    let entries = table_entries_from_pairs(cx, prepared, false)?;
    cx.factory()
        .opaque(Arc::new(HashTable::with_entries(entries)))
}

#[cfg(feature = "table-lazy")]
pub(super) fn table_lazy_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    use sim_kernel::Args;
    use std::sync::Arc;

    let entries = table_entries_from_pairs(cx, prepared, true)?
        .into_iter()
        .map(|(key, value)| {
            if value.object().as_callable().is_none() {
                return Err(Error::Eval(
                    "table/lazy expects alternating symbol and zero-argument callable pairs"
                        .to_owned(),
                ));
            }
            let loader: ValueLoader =
                Arc::new(move |cx: &mut Cx| cx.call_value(value.clone(), Args::new(Vec::new())));
            Ok((key, loader))
        })
        .collect::<Result<Vec<_>>>()?;
    cx.factory()
        .opaque(Arc::new(LazyTable::with_loaders(entries)))
}

#[cfg(feature = "table-fs")]
pub(super) fn table_fs_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let path = required_string_arg(cx, prepared, 0, "table/fs expects one root path string")?;
    install_fs_dir_lib(cx, &path)
}

#[cfg(feature = "table-fs")]
pub(super) fn dir_edit_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    if !(prepared.len() == 4 || prepared.len() == 5) {
        return Err(Error::Eval(
            "dir/edit expects a filesystem directory, key, old text, new text, and optional replace_all bool"
                .to_owned(),
        ));
    }
    let dir = required_fs_dir_arg(
        cx,
        prepared,
        0,
        "dir/edit expects a filesystem directory as its first argument",
    )?;
    let key = required_symbol_arg(cx, prepared, 1, "dir/edit expects a target key")?;
    let old = required_string_arg(cx, prepared, 2, "dir/edit expects old text")?;
    let new = required_string_arg(cx, prepared, 3, "dir/edit expects new text")?;
    let replace_all = optional_bool_arg(cx, prepared, 4, false)?;

    dir.edit(cx, key, &old, &new, replace_all)?;
    cx.factory().nil()
}

#[cfg(feature = "table-fs")]
pub(super) fn find_grep_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    if !(prepared.len() >= 2 && prepared.len() <= 4) {
        return Err(Error::Eval(
            "find/grep expects a filesystem directory, pattern, optional glob, and optional max"
                .to_owned(),
        ));
    }
    let dir = required_fs_dir_arg(
        cx,
        prepared,
        0,
        "find/grep expects a filesystem directory as its first argument",
    )?;
    let pattern = required_string_arg(cx, prepared, 1, "find/grep expects a pattern string")?;
    let glob = optional_string_arg(cx, prepared, 2)?;
    let max = optional_usize_arg(cx, prepared, 3, 100)?;

    let result = dir.find_grep(cx, &pattern, glob.as_deref(), max)?;
    find_grep_result_value(cx, result)
}

#[cfg(feature = "table-fs")]
pub(super) fn find_glob_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    if !(prepared.len() >= 2 && prepared.len() <= 3) {
        return Err(Error::Eval(
            "find/glob expects a filesystem directory, pattern, and optional max".to_owned(),
        ));
    }
    let dir = required_fs_dir_arg(
        cx,
        prepared,
        0,
        "find/glob expects a filesystem directory as its first argument",
    )?;
    let pattern = required_string_arg(cx, prepared, 1, "find/glob expects a pattern string")?;
    let max = optional_usize_arg(cx, prepared, 2, 100)?;

    let result = dir.find_glob(cx, &pattern, max)?;
    find_glob_result_value(cx, result)
}

#[cfg(feature = "table-fs")]
fn required_fs_dir_arg<'a>(
    cx: &mut Cx,
    prepared: &'a PreparedArgs,
    index: usize,
    message: &str,
) -> Result<&'a FsDir> {
    let value = prepared
        .get(index)
        .ok_or_else(|| Error::Eval(message.to_owned()))?;
    value.object().downcast_ref::<FsDir>().ok_or_else(|| {
        Error::Eval(format!(
            "{}; found {}",
            message,
            value_kind(cx, value).unwrap_or("unknown")
        ))
    })
}

#[cfg(feature = "table-fs")]
fn optional_string_arg(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    index: usize,
) -> Result<Option<String>> {
    let Some(value) = prepared.get(index) else {
        return Ok(None);
    };
    match value.object().as_expr(cx)? {
        Expr::Nil => Ok(None),
        Expr::String(text) => Ok(Some(text)),
        _ => Err(Error::TypeMismatch {
            expected: "string or nil",
            found: value_kind(cx, value)?,
        }),
    }
}

#[cfg(feature = "table-fs")]
fn optional_bool_arg(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    index: usize,
    default: bool,
) -> Result<bool> {
    let Some(value) = prepared.get(index) else {
        return Ok(default);
    };
    match value.object().as_expr(cx)? {
        Expr::Bool(value) => Ok(value),
        _ => Err(Error::TypeMismatch {
            expected: "bool",
            found: value_kind(cx, value)?,
        }),
    }
}

#[cfg(feature = "table-fs")]
fn optional_usize_arg(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    index: usize,
    default: usize,
) -> Result<usize> {
    let Some(value) = prepared.get(index) else {
        return Ok(default);
    };
    match value.object().as_expr(cx)? {
        Expr::Number(number) => number.canonical.parse::<usize>().map_err(|_| {
            Error::Eval(format!(
                "expected non-negative integer max, found {}",
                number.canonical
            ))
        }),
        _ => Err(Error::TypeMismatch {
            expected: "number",
            found: value_kind(cx, value)?,
        }),
    }
}

#[cfg(feature = "table-fs")]
fn find_grep_result_value(cx: &mut Cx, result: FindGrepResult) -> Result<Value> {
    let matches = result
        .matches
        .into_iter()
        .map(|matched| find_match_value(cx, matched))
        .collect::<Result<Vec<_>>>()?;
    let matches = cx.new_list(matches)?;
    let truncated = cx.factory().bool(result.truncated)?;
    cx.new_table(vec![
        (Symbol::new("matches"), matches),
        (Symbol::new("truncated"), truncated),
    ])
}

#[cfg(feature = "table-fs")]
fn find_match_value(cx: &mut Cx, matched: FindMatch) -> Result<Value> {
    let line = usize::try_from(matched.line)
        .map_err(|_| Error::Eval("find/grep line number does not fit usize".to_owned()))?;
    let path = cx.factory().string(matched.path)?;
    let line = number_value(cx, line)?;
    let text = cx.factory().string(matched.text)?;
    cx.new_table(vec![
        (Symbol::new("path"), path),
        (Symbol::new("line"), line),
        (Symbol::new("text"), text),
    ])
}

#[cfg(feature = "table-fs")]
fn find_glob_result_value(cx: &mut Cx, result: FindGlobResult) -> Result<Value> {
    let paths = result
        .paths
        .into_iter()
        .map(|path| cx.factory().string(path))
        .collect::<Result<Vec<_>>>()?;
    let paths = cx.new_list(paths)?;
    let truncated = cx.factory().bool(result.truncated)?;
    cx.new_table(vec![
        (Symbol::new("paths"), paths),
        (Symbol::new("truncated"), truncated),
    ])
}

#[cfg(feature = "table-db")]
pub(super) fn table_db_impl(
    cx: &mut Cx,
    _prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    install_db_dir_lib(cx)
}

#[cfg(feature = "table-remote")]
pub(super) fn table_remote_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let value = required_arg(prepared, 0, "table/remote expects one server connection")?;
    let connection = value
        .object()
        .downcast_ref::<Connection>()
        .ok_or_else(|| Error::Eval("table/remote expects one server connection".to_owned()))?;
    remote_dir_value(
        cx,
        connection.site().clone(),
        connection.default_codec().clone(),
    )
}

pub(super) fn table_impl_name_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    if prepared.is_empty() {
        return cx
            .factory()
            .symbol(Symbol::new(cx.table_registry().active().to_owned()));
    }

    cx.require(&config_table_impl_capability())?;
    let value = required_arg(prepared, 0, "table-impl expects zero or one symbol")?;
    let name = value_to_symbol_name(cx, &value)?;
    cx.table_registry_mut().set_active(&name)?;
    cx.factory().symbol(Symbol::new(name))
}

pub(super) fn get_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let table = required_table_arg(cx, prepared, 0, "get expects a table and a key")?;
    let key = required_symbol_arg(cx, prepared, 1, "get expects a table and a key")?;
    table.get(cx, key)
}

pub(super) fn set_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let table = required_table_arg(cx, prepared, 0, "set expects a table, key, and value")?;
    let key = required_symbol_arg(cx, prepared, 1, "set expects a table, key, and value")?;
    let value = required_arg(prepared, 2, "set expects a table, key, and value")?;
    table.set(cx, key, value)?;
    cx.factory().nil()
}

pub(super) fn has_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let table = required_table_arg(cx, prepared, 0, "has? expects a table and a key")?;
    let key = required_symbol_arg(cx, prepared, 1, "has? expects a table and a key")?;
    let present = table.has(cx, key)?;
    cx.factory().bool(present)
}

pub(super) fn del_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let table = required_table_arg(cx, prepared, 0, "del expects a table and a key")?;
    let key = required_symbol_arg(cx, prepared, 1, "del expects a table and a key")?;
    table.del(cx, key)
}

pub(super) fn keys_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let table = required_table_arg(cx, prepared, 0, "keys expects one table")?;
    let values = table
        .keys(cx)?
        .into_iter()
        .map(|symbol| cx.factory().symbol(symbol))
        .collect::<Result<Vec<_>>>()?;
    cx.new_list(values)
}

pub(super) fn entries_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let table = required_table_arg(cx, prepared, 0, "entries expects one table")?;
    let mut values = Vec::new();
    for (key, value) in table.entries(cx)? {
        let pair = cx.new_list(vec![cx.factory().symbol(key)?, value])?;
        values.push(pair);
    }
    cx.new_list(values)
}

pub(super) fn len_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let value = required_arg(prepared, 0, "len expects one list or table")?;
    if let Some(list) = value.object().as_list() {
        return match list.len(cx)? {
            sim_kernel::LengthResult::Known(len) => number_value(cx, len),
            sim_kernel::LengthResult::Unknown => cx.factory().symbol(Symbol::new("unknown")),
        };
    }
    if let Some(table) = value.object().as_table_impl() {
        let len = table.len(cx)?;
        return number_value(cx, len);
    }
    Err(Error::Eval("len expects one list or table".to_owned()))
}

pub(super) fn clear_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let table = required_table_arg(cx, prepared, 0, "clear expects one table")?;
    table.clear(cx)?;
    cx.factory().nil()
}

pub(super) fn mkdir_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let dir = required_dir_arg(cx, prepared, 0)?;
    let name = required_symbol_arg(
        cx,
        prepared,
        1,
        "mkdir expects a directory table and a name",
    )?;
    dir.mkdir(cx, name)
}

pub(super) fn opendir_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let dir = required_dir_arg(cx, prepared, 0)?;
    let name = required_symbol_arg(
        cx,
        prepared,
        1,
        "opendir expects a directory table and a name",
    )?;
    match dir.opendir(cx, name)? {
        Some(value) => Ok(value),
        None => cx.factory().nil(),
    }
}

pub(super) fn rmdir_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let dir = required_dir_arg(cx, prepared, 0)?;
    let name = required_symbol_arg(
        cx,
        prepared,
        1,
        "rmdir expects a directory table and a name",
    )?;
    dir.rmdir(cx, name)
}

pub(super) fn dir_impl(cx: &mut Cx, prepared: &PreparedArgs, _bindings: Bindings) -> Result<Value> {
    let dir = required_dir_arg(cx, prepared, 0)?;
    let name = required_symbol_arg(cx, prepared, 1, "dir? expects a directory table and a name")?;
    let present = dir.is_dir(cx, name)?;
    cx.factory().bool(present)
}
