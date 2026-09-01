// conformance: the SDK model-test feature exposes the canonical loadable entrypoint unchanged.

#[test]
fn model_test_facade_preserves_the_product_entrypoint() {
    assert_eq!(
        crate::model_test::product::model_test_entrypoint_symbol(),
        sim_kernel::Symbol::qualified("cli", "main/model-test")
    );
}
