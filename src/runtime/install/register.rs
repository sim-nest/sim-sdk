use std::sync::Arc;

use sim_kernel::{Cx, FunctionId, Linker, LoadCx, Result, Symbol};
use sim_shape::AnyShape;

use crate::{
    classes::NativeClass,
    functions::FunctionObject,
    macros::macroexpand_function,
    runtime::{
        eval_policy::WithEvalPolicyFunction,
        lambda::LambdaBuilder,
        lists::{
            car_function, cdr_function, cons_function, drop_function, empty_list_function,
            head_function, len_cmp_function, len_eq_function, len_gt_function, len_gte_function,
            len_lt_function, len_lte_function, list_function, list_impl_function, nth_function,
            tail_function, take_function,
        },
        realize::{LocalEvalFabricObject, RealizeFunction},
        shape_ops::{
            shape_class_symbols, shape_helper_function, shape_helper_symbol_names,
            shape_runtime_class,
        },
        tables::len_function,
    },
    shapes::CORE_SHAPE_CLASS,
};

use super::register_browse::register_core_browse_functions;
use super::register_tables::register_core_table_functions;

pub(super) trait CoreBuildCx {
    fn factory(&self) -> &dyn sim_kernel::Factory;
    fn fresh_function_id(&mut self) -> FunctionId;
    fn fresh_case_id(&mut self) -> sim_kernel::CaseId;
}

impl CoreBuildCx for Cx {
    fn factory(&self) -> &dyn sim_kernel::Factory {
        self.factory()
    }

    fn fresh_function_id(&mut self) -> FunctionId {
        self.registry_mut().fresh_function_id()
    }

    fn fresh_case_id(&mut self) -> sim_kernel::CaseId {
        self.registry_mut().fresh_case_id()
    }
}

impl CoreBuildCx for LoadCx {
    fn factory(&self) -> &dyn sim_kernel::Factory {
        self.factory()
    }

    fn fresh_function_id(&mut self) -> FunctionId {
        self.fresh_function_id()
    }

    fn fresh_case_id(&mut self) -> sim_kernel::CaseId {
        self.fresh_case_id()
    }
}

pub(super) fn register_core_classes(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
) -> Result<()> {
    for (symbol, class_id) in [
        (
            Symbol::qualified("core", "Class"),
            sim_kernel::CORE_CLASS_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Function"),
            sim_kernel::CORE_FUNCTION_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Macro"),
            sim_kernel::CORE_MACRO_CLASS_ID,
        ),
        (
            Symbol::qualified("core", CORE_SHAPE_CLASS),
            sim_kernel::CORE_SHAPE_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "ShapeMatch"),
            sim_kernel::CORE_SHAPE_MATCH_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Codec"),
            sim_kernel::CORE_CODEC_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Help"),
            sim_kernel::CORE_HELP_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Test"),
            sim_kernel::CORE_TEST_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "NumberDomain"),
            sim_kernel::CORE_NUMBER_DOMAIN_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Nil"),
            sim_kernel::CORE_NIL_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Bool"),
            sim_kernel::CORE_BOOL_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Number"),
            sim_kernel::CORE_NUMBER_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Symbol"),
            sim_kernel::CORE_SYMBOL_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "String"),
            sim_kernel::CORE_STRING_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Bytes"),
            sim_kernel::CORE_BYTES_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "List"),
            sim_kernel::CORE_LIST_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Table"),
            sim_kernel::CORE_TABLE_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Expr"),
            sim_kernel::CORE_EXPR_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Thunk"),
            sim_kernel::CORE_THUNK_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "EvalRequest"),
            sim_kernel::CORE_EVAL_REQUEST_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "EvalReply"),
            sim_kernel::CORE_EVAL_REPLY_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "LocalEvalFabric"),
            sim_kernel::CORE_LOCAL_EVAL_FABRIC_CLASS_ID,
        ),
        (
            Symbol::qualified("core", "Card"),
            sim_kernel::CORE_CARD_CLASS_ID,
        ),
    ] {
        let class_id = linker.class_with_id(symbol.clone(), class_id)?;
        let constructor_id = cx.fresh_function_id();
        let class = empty_class(class_id, constructor_id, symbol);
        let value = cx.factory().opaque(Arc::new(class))?;
        linker.bind_class_value(class_id, value)?;
    }
    for symbol in shape_class_symbols() {
        let class_id = linker.class(symbol.clone())?;
        let class = shape_runtime_class(
            class_id,
            cx.fresh_function_id(),
            cx.fresh_case_id(),
            symbol.name.as_ref(),
        );
        let value = cx.factory().opaque(Arc::new(class))?;
        linker.bind_class_value(class_id, value)?;
    }
    Ok(())
}

pub(super) fn register_core_functions(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
) -> Result<()> {
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "macroexpand"),
        macroexpand_function,
    )?;
    link_function(cx, linker, Symbol::new("macroexpand"), macroexpand_function)?;
    link_callable_value(
        cx,
        linker,
        Symbol::qualified("core", "lambda"),
        Arc::new(LambdaBuilder),
    )?;
    link_callable_value(cx, linker, Symbol::new("lambda"), Arc::new(LambdaBuilder))?;
    link_callable_value(
        cx,
        linker,
        Symbol::qualified("core", "with-eval-policy"),
        Arc::new(WithEvalPolicyFunction),
    )?;
    link_callable_value(
        cx,
        linker,
        Symbol::new("with-eval-policy"),
        Arc::new(WithEvalPolicyFunction),
    )?;
    register_core_browse_functions(cx, linker)?;
    for (namespace, name) in shape_helper_symbol_names() {
        let function_id = linker.function(Symbol::qualified(namespace, name))?;
        let function = shape_helper_function(function_id, cx.fresh_case_id(), namespace, name);
        let value = cx.factory().opaque(Arc::new(function))?;
        linker.bind_function_value(function_id, value)?;
    }
    link_function(cx, linker, Symbol::qualified("core", "list"), list_function)?;
    link_function(cx, linker, Symbol::new("list"), list_function)?;
    link_function(cx, linker, Symbol::qualified("core", "cons"), cons_function)?;
    link_function(cx, linker, Symbol::new("cons"), cons_function)?;
    link_function(cx, linker, Symbol::qualified("core", "car"), car_function)?;
    link_function(cx, linker, Symbol::new("car"), car_function)?;
    link_function(cx, linker, Symbol::qualified("core", "cdr"), cdr_function)?;
    link_function(cx, linker, Symbol::new("cdr"), cdr_function)?;
    link_function(cx, linker, Symbol::qualified("core", "head"), head_function)?;
    link_function(cx, linker, Symbol::new("head"), head_function)?;
    link_function(cx, linker, Symbol::qualified("core", "tail"), tail_function)?;
    link_function(cx, linker, Symbol::new("tail"), tail_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "empty?"),
        empty_list_function,
    )?;
    link_function(cx, linker, Symbol::new("empty?"), empty_list_function)?;
    link_function(cx, linker, Symbol::qualified("core", "len"), len_function)?;
    link_function(cx, linker, Symbol::new("len"), len_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "len-cmp"),
        len_cmp_function,
    )?;
    link_function(cx, linker, Symbol::new("len-cmp"), len_cmp_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "len<"),
        len_lt_function,
    )?;
    link_function(cx, linker, Symbol::new("len<"), len_lt_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "len<="),
        len_lte_function,
    )?;
    link_function(cx, linker, Symbol::new("len<="), len_lte_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "len="),
        len_eq_function,
    )?;
    link_function(cx, linker, Symbol::new("len="), len_eq_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "len>="),
        len_gte_function,
    )?;
    link_function(cx, linker, Symbol::new("len>="), len_gte_function)?;
    link_function(
        cx,
        linker,
        Symbol::qualified("core", "len>"),
        len_gt_function,
    )?;
    link_function(cx, linker, Symbol::new("len>"), len_gt_function)?;
    link_function(cx, linker, Symbol::qualified("core", "nth"), nth_function)?;
    link_function(cx, linker, Symbol::new("nth"), nth_function)?;
    link_function(cx, linker, Symbol::qualified("core", "take"), take_function)?;
    link_function(cx, linker, Symbol::new("take"), take_function)?;
    link_function(cx, linker, Symbol::qualified("core", "drop"), drop_function)?;
    link_function(cx, linker, Symbol::new("drop"), drop_function)?;
    link_list_impl_function(cx, linker, Symbol::qualified("core", "list-impl"))?;
    link_list_impl_function(cx, linker, Symbol::new("list-impl"))?;
    register_core_table_functions(cx, linker)?;
    link_callable_value(
        cx,
        linker,
        Symbol::qualified("core", "realize"),
        Arc::new(RealizeFunction),
    )?;
    link_callable_value(
        cx,
        linker,
        Symbol::new("realize"),
        Arc::new(RealizeFunction),
    )?;
    Ok(())
}

pub(super) fn register_core_values(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
) -> Result<()> {
    linker.value(
        Symbol::qualified("core", "local-fabric"),
        cx.factory().opaque(Arc::new(LocalEvalFabricObject))?,
    )?;
    Ok(())
}

pub(super) fn link_function(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
    symbol: Symbol,
    build: fn(sim_kernel::CaseId, FunctionId, Symbol) -> FunctionObject,
) -> Result<()> {
    let function_id = linker.function(symbol.clone())?;
    let function = build(cx.fresh_case_id(), function_id, symbol);
    let function_value = cx.factory().opaque(Arc::new(function))?;
    linker.bind_function_value(function_id, function_value)?;
    Ok(())
}

pub(super) fn link_two_case_function(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
    symbol: Symbol,
    build: fn(sim_kernel::CaseId, sim_kernel::CaseId, FunctionId, Symbol) -> FunctionObject,
) -> Result<()> {
    let function_id = linker.function(symbol.clone())?;
    let function = build(cx.fresh_case_id(), cx.fresh_case_id(), function_id, symbol);
    let function_value = cx.factory().opaque(Arc::new(function))?;
    linker.bind_function_value(function_id, function_value)?;
    Ok(())
}

fn link_callable_value(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
    symbol: Symbol,
    value: Arc<dyn sim_kernel::RuntimeObject>,
) -> Result<()> {
    let function_id = linker.function(symbol)?;
    let value = cx.factory().opaque(value)?;
    linker.bind_function_value(function_id, value)?;
    Ok(())
}

fn link_list_impl_function(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
    symbol: Symbol,
) -> Result<()> {
    let function_id = linker.function(symbol.clone())?;
    let function = list_impl_function(cx.fresh_case_id(), cx.fresh_case_id(), function_id, symbol);
    let function_value = cx.factory().opaque(Arc::new(function))?;
    linker.bind_function_value(function_id, function_value)?;
    Ok(())
}

fn empty_function(id: FunctionId, symbol: Symbol) -> FunctionObject {
    FunctionObject::new(id, symbol, Vec::new())
}

fn empty_class(
    class_id: sim_kernel::ClassId,
    function_id: FunctionId,
    symbol: Symbol,
) -> NativeClass {
    NativeClass::new(
        class_id,
        symbol.clone(),
        empty_function(function_id, symbol),
        Some(Arc::new(AnyShape)),
        Vec::new(),
    )
    .with_read_constructor(None)
}
