#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn expr_tree_facade_is_opt_in_and_complete() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "expr-tree",
        &[
            "dep:sim-expr-tree-core",
            "dep:sim-expr-tree-calc",
            "dep:sim-lib-expr-tree",
            "dep:sim-lib-view-expr-tree",
            "dep:sim-lib-expr-tree-server",
        ],
    );
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "expr-tree")
    );
}

#[test]
fn expr_tree_facade_reexports_only_canonical_product_layers() {
    assert_eq!(sim::expr_tree_core::crate_identity(), "sim-expr-tree-core");
    assert_eq!(sim::expr_tree_calc::crate_identity(), "sim-expr-tree-calc");
    assert_eq!(sim::expr_tree::crate_identity(), "sim-lib-expr-tree");
    assert_eq!(
        sim::view_expr_tree::crate_identity(),
        "sim-lib-view-expr-tree"
    );
    assert_eq!(
        sim::expr_tree_server::crate_identity(),
        "sim-lib-expr-tree-server"
    );
}
