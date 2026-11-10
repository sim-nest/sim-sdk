// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Unpublished manifests the repository contract excludes.

use std::path::{Path, PathBuf};

fn consume_fixture(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn assert_unpublished(manifest: &Path, package: &str) {
    let text = std::fs::read_to_string(manifest)
        .unwrap_or_else(|err| panic!("{}: {err}", manifest.display()));
    assert!(
        text.contains("publish = false"),
        "{} is publishable",
        manifest.display()
    );
    assert!(
        text.contains(&format!("name = \"{package}\"")),
        "{} does not name {package}",
        manifest.display()
    );
}

#[test]
fn proc_macro_ui_fixtures_stay_unpublished() {
    let root = consume_fixture("tests/proc-macro-ui");
    assert_unpublished(
        &root.join("invalid-shape/Cargo.toml"),
        "proc-macro-invalid-shape",
    );
    assert_unpublished(
        &root.join("missing-constructor/Cargo.toml"),
        "proc-macro-missing-constructor",
    );
}

#[test]
fn native_abi_ui_fixture_stays_unpublished() {
    let root = consume_fixture("tests/native-abi-ui");
    assert_unpublished(
        &root.join("borrow-to-destroy/Cargo.toml"),
        "native-abi-borrow-to-destroy",
    );
}

#[test]
fn native_plugin_fixture_stays_unpublished() {
    let root = consume_fixture("tests/native-plugin");
    assert_unpublished(&root.join("Cargo.toml"), "native-plugin-fixture");
}

#[test]
fn first_light_focused_harness_stays_unpublished() {
    let root = consume_fixture("tests/first-light-focused");
    assert_unpublished(&root.join("Cargo.toml"), "sim-sdk-first-light-focused");
}

#[test]
fn memo_acceptance_focused_harness_stays_unpublished() {
    let root = consume_fixture("tests/memo-acceptance-focused");
    assert_unpublished(&root.join("Cargo.toml"), "sim-sdk-memo-acceptance-focused");
}

#[test]
fn reversible_product_focused_harness_stays_unpublished() {
    let root = consume_fixture("tests/reversible-product-focused");
    assert_unpublished(
        &root.join("Cargo.toml"),
        "sim-sdk-reversible-product-focused",
    );
}
