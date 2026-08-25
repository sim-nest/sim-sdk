#![cfg(all(feature = "agent", feature = "hotload"))]

#[path = "../recipes/hotload/conversational-instrument-adoption/setup.rs"]
mod recipe;

#[test]
fn checked_recipe_uses_the_public_adoption_surface() {
    let function = recipe::adopt_reproduced_instrument;
    let _ = function;
}
