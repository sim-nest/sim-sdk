#![cfg(feature = "javascript")]

mod recipe {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/recipes/javascript/bounded-module/setup.rs"
    ));
}

#[test]
fn bounded_javascript_module_recipe_runs() {
    recipe::bounded_javascript_module().expect("JavaScript SDK recipe should pass");
}
