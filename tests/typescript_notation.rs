// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// conformance: the SDK TypeScript notation composition runs the admitted notation recipe.

#![cfg(feature = "typescript")]

mod recipe {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/",
        "recipes/typescript-notation/admitted-notation/setup.rs"
    ));
}

#[test]
fn admitted_typescript_notation_recipe_runs() {
    recipe::admitted_typescript_notation().expect("TypeScript notation SDK recipe should pass");
}
