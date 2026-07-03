//! Shared helpers for the `rNN_*_features` Cargo-manifest contract tests.
//!
//! Every feature-contract integration test parses the root `Cargo.toml`
//! `[features]` table and asserts that a feature pulls in (or omits) specific
//! dependencies. These helpers were copy-pasted into each test; this module is
//! their single home, included via `#[path = "support/features.rs"] mod
//! features;`.

use std::collections::BTreeMap;

/// Parse the `[features]` table of a Cargo manifest into feature -> deps.
pub fn collect_feature_dependencies(manifest: &str) -> BTreeMap<String, Vec<String>> {
    let mut in_features = false;
    let mut features = BTreeMap::new();
    let mut current: Option<(String, Vec<String>)> = None;

    for raw in manifest.lines() {
        let line = raw.trim();
        if line == "[features]" {
            in_features = true;
            continue;
        }
        if in_features && line.starts_with('[') {
            break;
        }
        if !in_features || line.is_empty() {
            continue;
        }
        if let Some((name, deps)) = parse_feature_line(line) {
            if let Some((previous, previous_deps)) = current.take() {
                features.insert(previous, previous_deps);
            }
            if line.ends_with(']') {
                features.insert(name, deps);
            } else {
                current = Some((name, deps));
            }
            continue;
        }
        if let Some((_, deps)) = current.as_mut() {
            deps.extend(parse_array_items(line));
            if line.ends_with(']') {
                let (name, deps) = current.take().expect("current feature");
                features.insert(name, deps);
            }
        }
    }

    features
}

fn parse_feature_line(line: &str) -> Option<(String, Vec<String>)> {
    let (name, rest) = line.split_once('=')?;
    Some((name.trim().to_owned(), parse_array_items(rest)))
}

fn parse_array_items(line: &str) -> Vec<String> {
    line.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// Assert that `feature` exists and includes every dependency in `expected`.
pub fn assert_feature_includes(
    features: &BTreeMap<String, Vec<String>>,
    feature: &str,
    expected: &[&str],
) {
    let deps = features.get(feature).expect("feature exists");
    for dep in expected {
        assert!(
            deps.iter().any(|candidate| candidate == dep),
            "{feature} should include {dep}; got {deps:?}"
        );
    }
}
