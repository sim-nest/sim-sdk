use std::sync::Arc;

use sim_kernel::{Cx, Demand, FunctionId, PreparedArgs, Result, Symbol, Value};
use sim_shape::{AnyShape, Bindings, CaptureShape, FunctionCase, FunctionObject, ListShape};

#[cfg(feature = "table-db")]
use super::ops::table_db_impl;
#[cfg(feature = "table-hash")]
use super::ops::table_hash_impl;
#[cfg(feature = "table-lazy")]
use super::ops::table_lazy_impl;
#[cfg(feature = "table-remote")]
use super::ops::table_remote_impl;
use super::ops::{
    clear_impl, del_impl, dir_impl, entries_impl, get_impl, has_impl, keys_impl, len_impl,
    mkdir_impl, opendir_impl, rmdir_impl, set_impl, table_catalog_impl, table_impl,
    table_impl_name_impl,
};
#[cfg(feature = "table-fs")]
use super::ops::{dir_edit_impl, find_glob_impl, find_grep_impl, table_fs_impl};

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

fn unary_any_function(
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

fn binary_table_function(
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
                Arc::new(CaptureShape::new(
                    Symbol::new("subject"),
                    Arc::new(AnyShape),
                )),
                Arc::new(CaptureShape::new(Symbol::new("arg"), Arc::new(AnyShape))),
            ])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![Demand::Value, Demand::Value],
            priority: 10,
            implementation,
        }],
    )
}

fn ternary_any_function(
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
            name: Symbol::qualified(symbol.to_string(), "three"),
            args: Arc::new(ListShape::new(vec![
                Arc::new(CaptureShape::new(
                    Symbol::new("subject"),
                    Arc::new(AnyShape),
                )),
                Arc::new(CaptureShape::new(Symbol::new("arg0"), Arc::new(AnyShape))),
                Arc::new(CaptureShape::new(Symbol::new("arg1"), Arc::new(AnyShape))),
            ])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![Demand::Value, Demand::Value, Demand::Value],
            priority: 10,
            implementation,
        }],
    )
}

pub(crate) fn table_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    variadic_function(case_id, function_id, symbol, table_impl)
}

pub(crate) fn table_catalog_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    variadic_function(case_id, function_id, symbol, table_catalog_impl)
}

#[cfg(feature = "table-hash")]
pub(crate) fn table_hash_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    variadic_function(case_id, function_id, symbol, table_hash_impl)
}

#[cfg(feature = "table-lazy")]
pub(crate) fn table_lazy_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    variadic_function(case_id, function_id, symbol, table_lazy_impl)
}

#[cfg(feature = "table-fs")]
pub(crate) fn table_fs_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_any_function(case_id, function_id, symbol, table_fs_impl)
}

#[cfg(feature = "table-fs")]
pub(crate) fn dir_edit_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    variadic_function(case_id, function_id, symbol, dir_edit_impl)
}

#[cfg(feature = "table-fs")]
pub(crate) fn find_grep_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    variadic_function(case_id, function_id, symbol, find_grep_impl)
}

#[cfg(feature = "table-fs")]
pub(crate) fn find_glob_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    variadic_function(case_id, function_id, symbol, find_glob_impl)
}

#[cfg(feature = "table-db")]
pub(crate) fn table_db_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "zero"),
            args: Arc::new(ListShape::new(Vec::new())),
            result: Some(Arc::new(AnyShape)),
            demand: Vec::new(),
            priority: 10,
            implementation: table_db_impl,
        }],
    )
}

#[cfg(feature = "table-remote")]
pub(crate) fn table_remote_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_any_function(case_id, function_id, symbol, table_remote_impl)
}

pub(crate) fn table_impl_function(
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
                implementation: table_impl_name_impl,
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
                implementation: table_impl_name_impl,
            },
        ],
    )
}

pub(crate) fn get_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_table_function(case_id, function_id, symbol, get_impl)
}

pub(crate) fn set_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    ternary_any_function(case_id, function_id, symbol, set_impl)
}

pub(crate) fn has_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_table_function(case_id, function_id, symbol, has_impl)
}

pub(crate) fn del_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_table_function(case_id, function_id, symbol, del_impl)
}

pub(crate) fn keys_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_any_function(case_id, function_id, symbol, keys_impl)
}

pub(crate) fn entries_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_any_function(case_id, function_id, symbol, entries_impl)
}

pub(crate) fn len_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_any_function(case_id, function_id, symbol, len_impl)
}

pub(crate) fn clear_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    unary_any_function(case_id, function_id, symbol, clear_impl)
}

pub(crate) fn mkdir_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_table_function(case_id, function_id, symbol, mkdir_impl)
}

pub(crate) fn opendir_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_table_function(case_id, function_id, symbol, opendir_impl)
}

pub(crate) fn rmdir_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_table_function(case_id, function_id, symbol, rmdir_impl)
}

pub(crate) fn dir_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    binary_table_function(case_id, function_id, symbol, dir_impl)
}
