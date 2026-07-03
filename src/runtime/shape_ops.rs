use std::sync::Arc;

#[path = "shape_ops_impl.rs"]
mod shape_ops_impl;

use sim_kernel::{Demand, FunctionId, Symbol};
use sim_shape::{AnyShape, ClassShape, FunctionCase, FunctionObject, Shape};

use crate::classes::NativeClass;

use self::shape_ops_impl::*;

const SHAPE_CLASS_NAMES: [&str; 9] = [
    "AnyShape",
    "ExprKindShape",
    "ClassShape",
    "ExactExprShape",
    "ListShape",
    "CaptureShape",
    "OneOfShape",
    "FieldShape",
    "EffectfulShape",
];

const SHAPE_HELPER_SYMBOL_NAMES: &[(&str, &str)] = &[
    ("class", "subclass?"),
    ("shape", "subshape?"),
    ("shape", "parents"),
    ("shape", "check"),
    ("shape", "check-expr"),
    ("shape", "assert"),
    ("shape", "accepted?"),
    ("shape", "rejected?"),
    ("shape", "score"),
    ("shape", "value-captures"),
    ("shape", "expr-captures"),
    ("shape", "diagnostics"),
    ("shape", "and"),
    ("shape", "or"),
    ("shape", "not"),
    ("shape", "list"),
    ("shape", "list-rest"),
    ("shape", "table"),
    ("shape", "table-required"),
    ("shape", "table-open"),
    ("shape", "table-closed"),
    ("shape", "repeat"),
    ("shape", "repeat-bounds"),
    ("shape", "all"),
    ("shape", "any"),
    ("shape", "none"),
    ("shape", "without"),
    ("shape", "compare"),
    ("shape", "compare-with"),
    ("shape", "venn"),
    ("shape", "venn-union"),
    ("shape", "venn-intersection"),
    ("shape", "venn-only"),
    ("shape", "venn-outside"),
    ("shape", "venn-exactly"),
    ("shape", "hook"),
    ("shape", "hook-trace"),
    ("shape", "hook-score-floor"),
    ("shape", "hook-accept-on-no-diagnostics"),
    ("shape", "hook-discard-on-diagnostic-prefix"),
];

pub(crate) fn shape_class_symbol(name: &str) -> Symbol {
    Symbol::qualified("core", name)
}

pub(crate) fn shape_class_symbols() -> impl Iterator<Item = Symbol> {
    SHAPE_CLASS_NAMES.into_iter().map(shape_class_symbol)
}

pub(crate) fn shape_runtime_class(
    class_id: sim_kernel::ClassId,
    function_id: FunctionId,
    case_id: sim_kernel::CaseId,
    name: &str,
) -> NativeClass {
    let shape_parent = vec![Symbol::qualified("core", "Shape")];
    let instance_shape = Some(shape_class_impl_shape());
    match name {
        "AnyShape" => NativeClass::new(
            class_id,
            shape_class_symbol("AnyShape"),
            shape_constructor(case_id, function_id, "AnyShape", vec![], any_shape_impl),
            instance_shape,
            Vec::new(),
        )
        .with_parents(shape_parent),
        "ExprKindShape" => NativeClass::new(
            class_id,
            shape_class_symbol("ExprKindShape"),
            shape_constructor(
                case_id,
                function_id,
                "ExprKindShape",
                vec![Demand::Value],
                expr_kind_shape_impl,
            ),
            instance_shape,
            Vec::new(),
        )
        .with_parents(shape_parent),
        "ClassShape" => NativeClass::new(
            class_id,
            shape_class_symbol("ClassShape"),
            shape_constructor(
                case_id,
                function_id,
                "ClassShape",
                vec![Demand::Value],
                class_shape_impl,
            ),
            instance_shape,
            Vec::new(),
        )
        .with_parents(shape_parent),
        "ExactExprShape" => NativeClass::new(
            class_id,
            shape_class_symbol("ExactExprShape"),
            shape_constructor(
                case_id,
                function_id,
                "ExactExprShape",
                vec![Demand::Expr],
                exact_expr_shape_impl,
            ),
            instance_shape,
            Vec::new(),
        )
        .with_parents(shape_parent),
        "ListShape" => NativeClass::new(
            class_id,
            shape_class_symbol("ListShape"),
            shape_constructor(
                case_id,
                function_id,
                "ListShape",
                vec![Demand::Value],
                list_shape_impl,
            ),
            instance_shape,
            Vec::new(),
        )
        .with_parents(shape_parent),
        "CaptureShape" => NativeClass::new(
            class_id,
            shape_class_symbol("CaptureShape"),
            shape_constructor(
                case_id,
                function_id,
                "CaptureShape",
                vec![Demand::Value, Demand::Value],
                capture_shape_impl,
            ),
            instance_shape,
            Vec::new(),
        )
        .with_parents(shape_parent),
        "OneOfShape" => NativeClass::new(
            class_id,
            shape_class_symbol("OneOfShape"),
            shape_constructor(
                case_id,
                function_id,
                "OneOfShape",
                vec![Demand::Value],
                one_of_shape_impl,
            ),
            instance_shape,
            Vec::new(),
        )
        .with_parents(shape_parent),
        "FieldShape" => NativeClass::new(
            class_id,
            shape_class_symbol("FieldShape"),
            shape_constructor(
                case_id,
                function_id,
                "FieldShape",
                vec![Demand::Value, Demand::Value],
                field_shape_impl,
            ),
            instance_shape,
            Vec::new(),
        )
        .with_parents(shape_parent),
        "EffectfulShape" => NativeClass::new(
            class_id,
            shape_class_symbol("EffectfulShape"),
            shape_constructor(
                case_id,
                function_id,
                "EffectfulShape",
                vec![Demand::Value],
                effectful_shape_impl,
            ),
            instance_shape,
            Vec::new(),
        )
        .with_parents(shape_parent),
        other => panic!("unknown shape runtime class {other}"),
    }
}

pub(crate) fn shape_helper_symbol_names() -> impl Iterator<Item = (&'static str, &'static str)> {
    SHAPE_HELPER_SYMBOL_NAMES.iter().copied()
}

pub(crate) fn shape_helper_function(
    function_id: FunctionId,
    case_id: sim_kernel::CaseId,
    namespace: &str,
    name: &str,
) -> FunctionObject {
    let symbol = Symbol::qualified(namespace, name);
    let (demand, implementation) = shape_helper_spec(namespace, name);
    helper_function(function_id, case_id, symbol, demand, implementation)
}

fn shape_constructor(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    name: &str,
    demand: Vec<Demand>,
    implementation: sim_shape::NativeFunctionImpl,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        shape_class_symbol(name),
        vec![FunctionCase {
            id: case_id,
            name: shape_class_symbol(name),
            args: Arc::new(AnyShape),
            result: Some(Arc::new(AnyShape)),
            demand,
            priority: 10,
            implementation,
        }],
    )
}

fn shape_class_impl_shape() -> Arc<dyn Shape> {
    Arc::new(ClassShape::new(Symbol::qualified("core", "Shape")))
}

fn helper_function(
    function_id: FunctionId,
    case_id: sim_kernel::CaseId,
    symbol: Symbol,
    demand: Vec<Demand>,
    implementation: sim_shape::NativeFunctionImpl,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: symbol,
            args: Arc::new(AnyShape),
            result: Some(Arc::new(AnyShape)),
            demand,
            priority: 10,
            implementation,
        }],
    )
}
