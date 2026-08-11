#![cfg(feature = "typescript")]

mod recipe {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/recipes/typescript-notation/admitted-notation/setup.rs"
    ));
}

#[test]
fn admitted_typescript_notation_recipe_runs() {
    recipe::admitted_typescript_notation().expect("TypeScript notation SDK recipe should pass");
}
