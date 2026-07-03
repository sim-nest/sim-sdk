use sim_kernel::{Cx, Result, ShapeRef, TableRef};
use sim_shape::Shape;

pub use sim_shape::{
    FunctionCase, FunctionObject, NativeFunctionImpl, SelectedCase, case_result_shape, case_shape,
    function_cases, overload,
};

/// Builds an empty member table for classes that expose no members.
pub fn empty_member_table(cx: &mut Cx) -> Result<TableRef> {
    cx.factory().table(Vec::new())
}

/// Returns a nil shape reference, used where a function has no declared shape.
pub fn empty_shape_ref(cx: &mut Cx) -> Result<ShapeRef> {
    cx.factory().nil()
}

/// Returns the argument shape of a function overload case.
pub fn function_case_shape(case: &FunctionCase) -> &dyn Shape {
    case_shape(case)
}

/// Returns the result shape of a function overload case, if it declares one.
pub fn function_case_result_shape(case: &FunctionCase) -> Option<&dyn Shape> {
    case_result_shape(case)
}
