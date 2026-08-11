#![cfg(feature = "python")]

mod recipe {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/recipes/python/capability-scoped/setup.rs"
    ));
}

#[test]
fn capability_scoped_python_recipe_runs() {
    recipe::capability_scoped_python().expect("Python SDK recipe should pass");
}
