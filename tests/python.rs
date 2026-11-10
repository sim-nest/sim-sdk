// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// conformance: the SDK Python composition runs the capability-scoped recipe.

#![cfg(feature = "python")]

mod recipe {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/",
        "recipes/python/capability-scoped/setup.rs"
    ));
}

#[test]
fn capability_scoped_python_recipe_runs() {
    recipe::capability_scoped_python().expect("Python SDK recipe should pass");
}
