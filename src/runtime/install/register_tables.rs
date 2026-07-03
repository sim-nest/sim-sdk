use sim_kernel::{Linker, Result, Symbol};

use crate::runtime::tables::{
    clear_function, del_function, dir_function, entries_function, get_function, has_function,
    keys_function, mkdir_function, opendir_function, rmdir_function, set_function,
    table_catalog_function, table_function, table_impl_function,
};

use super::register::{CoreBuildCx, link_function};

pub(super) fn register_core_table_functions(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
) -> Result<()> {
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "table"),
        table_function,
    )?;
    link_function(cx, linker, Symbol::new("table"), table_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("table", "catalog"),
        table_catalog_function,
    )?;
    #[cfg(feature = "table-hash")]
    link_function(
        cx,
        linker,
        Symbol::qualified("table", "hash"),
        crate::runtime::tables::table_hash_function,
    )?;
    #[cfg(feature = "table-lazy")]
    link_function(
        cx,
        linker,
        Symbol::qualified("table", "lazy"),
        crate::runtime::tables::table_lazy_function,
    )?;
    #[cfg(feature = "table-fs")]
    link_function(
        cx,
        linker,
        Symbol::qualified("table", "fs"),
        crate::runtime::tables::table_fs_function,
    )?;
    #[cfg(feature = "table-db")]
    link_function(
        cx,
        linker,
        Symbol::qualified("table", "db"),
        crate::runtime::tables::table_db_function,
    )?;
    #[cfg(feature = "table-remote")]
    link_function(
        cx,
        linker,
        Symbol::qualified("table", "remote"),
        crate::runtime::tables::table_remote_function,
    )?;
    link_table_impl_function(cx, linker, Symbol::qualified("core", "table-impl"))?;
    link_table_impl_function(cx, linker, Symbol::new("table-impl"))?;
    link_function(cx, linker, Symbol::qualified("core", "get"), get_function)?;
    link_function(cx, linker, Symbol::new("get"), get_function)?;
    link_function(cx, linker, Symbol::qualified("core", "set"), set_function)?;
    link_function(cx, linker, Symbol::new("set"), set_function)?;
    link_function(cx, linker, Symbol::qualified("core", "has?"), has_function)?;
    link_function(cx, linker, Symbol::new("has?"), has_function)?;
    link_function(cx, linker, Symbol::qualified("core", "del"), del_function)?;
    link_function(cx, linker, Symbol::new("del"), del_function)?;
    link_function(cx, linker, Symbol::qualified("core", "keys"), keys_function)?;
    link_function(cx, linker, Symbol::new("keys"), keys_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "entries"),
        entries_function,
    )?;
    link_function(cx, linker, Symbol::new("entries"), entries_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "clear"),
        clear_function,
    )?;
    link_function(cx, linker, Symbol::new("clear"), clear_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "mkdir"),
        mkdir_function,
    )?;
    link_function(cx, linker, Symbol::new("mkdir"), mkdir_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "opendir"),
        opendir_function,
    )?;
    link_function(cx, linker, Symbol::new("opendir"), opendir_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "rmdir"),
        rmdir_function,
    )?;
    link_function(cx, linker, Symbol::new("rmdir"), rmdir_function)?;
    link_function(cx, linker, Symbol::qualified("core", "dir?"), dir_function)?;
    link_function(cx, linker, Symbol::new("dir?"), dir_function)?;
    Ok(())
}

fn link_table_impl_function(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
    symbol: Symbol,
) -> Result<()> {
    let function_id = linker.function(symbol.clone())?;
    let function = table_impl_function(cx.fresh_case_id(), cx.fresh_case_id(), function_id, symbol);
    let function_value = cx.factory().opaque(std::sync::Arc::new(function))?;
    linker.bind_function_value(function_id, function_value)?;
    Ok(())
}
