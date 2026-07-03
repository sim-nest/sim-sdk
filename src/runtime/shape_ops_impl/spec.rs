use sim_kernel::Demand;

use super::{
    algebra, class_subclass_impl, compare, hooks, shape_assert_impl, shape_check_expr_impl,
    shape_check_impl, shape_match_accepted_impl, shape_match_diagnostics_impl,
    shape_match_expr_captures_impl, shape_match_rejected_impl, shape_match_score_impl,
    shape_match_value_captures_impl, shape_parents_impl, shape_subshape_impl,
};

pub(super) fn shape_helper_spec(
    namespace: &str,
    name: &str,
) -> (Vec<Demand>, sim_shape::NativeFunctionImpl) {
    match (namespace, name) {
        ("class", "subclass?") => (
            vec![Demand::Value, Demand::Value],
            class_subclass_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "subshape?") => (
            vec![Demand::Value, Demand::Value],
            shape_subshape_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "parents") => (
            vec![Demand::Value],
            shape_parents_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "check") => (
            vec![Demand::Value, Demand::Value],
            shape_check_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "check-expr") => (
            vec![Demand::Value, Demand::Expr],
            shape_check_expr_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "assert") => (
            vec![Demand::Value, Demand::Value],
            shape_assert_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "accepted?") => (
            vec![Demand::Value],
            shape_match_accepted_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "rejected?") => (
            vec![Demand::Value],
            shape_match_rejected_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "score") => (
            vec![Demand::Value],
            shape_match_score_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "value-captures") => (
            vec![Demand::Value],
            shape_match_value_captures_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "expr-captures") => (
            vec![Demand::Value],
            shape_match_expr_captures_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "diagnostics") => (
            vec![Demand::Value],
            shape_match_diagnostics_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "and") | ("shape", "all") => (
            vec![Demand::Value],
            algebra::shape_and_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "or") | ("shape", "any") => (
            vec![Demand::Value],
            algebra::shape_or_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "not") | ("shape", "none") => (
            vec![Demand::Value],
            algebra::shape_not_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "list") => (
            vec![Demand::Value],
            algebra::shape_list_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "list-rest") => (
            vec![Demand::Value, Demand::Value],
            algebra::shape_list_rest_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "table") => (
            vec![Demand::Value, Demand::Value],
            algebra::shape_table_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "table-required") => (
            vec![Demand::Value],
            algebra::shape_table_required_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "table-open") => (
            vec![Demand::Value],
            algebra::shape_table_open_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "table-closed") => (
            vec![Demand::Value],
            algebra::shape_table_closed_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "repeat") => (
            vec![Demand::Value],
            algebra::shape_repeat_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "repeat-bounds") => (
            vec![Demand::Value, Demand::Value, Demand::Value],
            algebra::shape_repeat_bounds_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "without") => (
            vec![Demand::Value, Demand::Value],
            algebra::shape_without_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "compare") => (
            vec![Demand::Value, Demand::Value],
            compare::shape_compare_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "compare-with") => (
            vec![Demand::Value, Demand::Value, Demand::Value],
            compare::shape_compare_with_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "venn") => (
            vec![Demand::Value],
            compare::shape_venn_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "venn-union") => (
            vec![Demand::Value],
            compare::shape_venn_union_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "venn-intersection") => (
            vec![Demand::Value],
            compare::shape_venn_intersection_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "venn-only") => (
            vec![Demand::Value, Demand::Value],
            compare::shape_venn_only_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "venn-outside") => (
            vec![Demand::Value],
            compare::shape_venn_outside_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "venn-exactly") => (
            vec![Demand::Value, Demand::Value],
            compare::shape_venn_exactly_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "hook") => (
            vec![Demand::Value, Demand::Value],
            hooks::shape_hook_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "hook-trace") => (
            Vec::new(),
            hooks::shape_hook_trace_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "hook-score-floor") => (
            vec![Demand::Value],
            hooks::shape_hook_score_floor_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "hook-accept-on-no-diagnostics") => (
            Vec::new(),
            hooks::shape_hook_accept_on_no_diagnostics_impl as sim_shape::NativeFunctionImpl,
        ),
        ("shape", "hook-discard-on-diagnostic-prefix") => (
            vec![Demand::Value],
            hooks::shape_hook_discard_on_diagnostic_prefix_impl as sim_shape::NativeFunctionImpl,
        ),
        _ => panic!("unknown shape helper {namespace}/{name}"),
    }
}
