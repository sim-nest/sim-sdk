// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// conformance: the SDK source-authority facade admits dynamic text only after the caller power is granted.

#![cfg(all(feature = "core", feature = "shape", feature = "codec-lisp"))]

mod recipe {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/",
        "recipes/source-authority/dynamic-text/setup.rs"
    ));
}

#[test]
fn dynamic_text_source_authority_admits_only_after_the_caller_power() {
    recipe::dynamic_text_source_authority()
        .expect("dynamic text source authority should admit 42 after the caller power");
}
