// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// conformance: the SDK JavaScript composition runs the bounded module recipe.

#![cfg(feature = "javascript")]

mod recipe {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/",
        "recipes/javascript/bounded-module/setup.rs"
    ));
}

#[test]
fn bounded_javascript_module_recipe_runs() {
    recipe::bounded_javascript_module().expect("JavaScript SDK recipe should pass");
}
