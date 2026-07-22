use std::{fs, path::Path};

use super::support::repo_root;

const REEXPORT_AND_WIRING: &[&str] = &[
    "src/bin/sim.rs",
    "src/femm_exports.rs",
    "src/lib.rs",
    "src/loaders.rs",
    "src/loaders/registry.rs",
    "src/numbers_exports.rs",
    "src/roadmap11_exports.rs",
    "src/runtime.rs",
    "src/standard_exports.rs",
];

const AUTHORING_HELPERS: &[&str] = &[
    "src/classes.rs",
    "src/classes/",
    "src/compat.rs",
    "src/functions.rs",
    "src/macros.rs",
    "src/macros/",
    "src/music_stack.rs",
    "src/shapes.rs",
];

const RUNTIME_BEHAVIOR_ALLOWLIST: &[&str] = &[
    "src/runtime/browse.rs",
    "src/runtime/browse/",
    "src/runtime/capabilities.rs",
    "src/runtime/cookbook_directory.rs",
    "src/runtime/cookbook_directory/",
    "src/runtime/cookbook_discovery.rs",
    "src/runtime/eval_policy.rs",
    "src/runtime/glasses.rs",
    "src/runtime/glasses/",
    "src/runtime/help.rs",
    "src/runtime/install.rs",
    "src/runtime/install/",
    "src/runtime/lambda.rs",
    "src/runtime/lists.rs",
    "src/runtime/realize.rs",
    "src/runtime/reference_device.rs",
    "src/runtime/reference_device/",
    "src/runtime/watch.rs",
    "src/runtime/watch/",
    "src/runtime/shape_ops.rs",
    "src/runtime/shape_ops_impl.rs",
    "src/runtime/shape_ops_impl/",
    "src/runtime/tables.rs",
    "src/runtime/tables/",
    "src/runtime/test_runs.rs",
    "src/runtime/testing.rs",
];

const TEST_SUPPORT: &[&str] = &[
    "src/codec_matrix_tests.rs",
    "src/codec_matrix_tests/",
    "src/feature_contract_tests.rs",
    "src/feature_contract_tests/",
    "src/loaders/tests.rs",
    "src/loaders/tests/",
    "src/music_stack_tests.rs",
    "src/runtime/tests.rs",
    "src/runtime/tests/",
    "src/skill_tests.rs",
];

const EXTRACTED_LOADER_BEHAVIOR: &[&str] = &[
    "src/loaders/binary_pack.rs",
    "src/loaders/reexport.rs",
    "src/loaders/shared.rs",
    "src/loaders/source.rs",
    "src/loaders/source/compile.rs",
];

const LOADER_WIRING_FILES: &[&str] = &["src/loaders/registry.rs"];

#[test]
fn facade_sources_match_behavior_allowlist() {
    let root = repo_root();
    let mut unclassified = collect_rust_files(&root.join("src"))
        .into_iter()
        .map(|path| slash_path(path.strip_prefix(&root).unwrap()))
        .filter(|path| !is_test_path(path))
        .filter(|path| !matches_any(path, REEXPORT_AND_WIRING))
        .filter(|path| !matches_any(path, AUTHORING_HELPERS))
        .filter(|path| !matches_any(path, RUNTIME_BEHAVIOR_ALLOWLIST))
        .collect::<Vec<_>>();
    unclassified.sort();

    assert!(
        unclassified.is_empty(),
        "new facade production modules must be classified as wiring or explicit behavior: {unclassified:?}"
    );
}

#[test]
fn loader_behavior_stays_out_of_facade() {
    let root = repo_root();
    let lingering = EXTRACTED_LOADER_BEHAVIOR
        .iter()
        .filter(|path| root.join(path).exists())
        .copied()
        .collect::<Vec<_>>();
    assert!(
        lingering.is_empty(),
        "loader implementation belongs in sim-run-loaders, not the facade: {lingering:?}"
    );

    let loader_files = collect_rust_files(&root.join("src/loaders"))
        .into_iter()
        .map(|path| slash_path(path.strip_prefix(&root).unwrap()))
        .filter(|path| !is_test_path(path))
        .filter(|path| !LOADER_WIRING_FILES.contains(&path.as_str()))
        .collect::<Vec<_>>();
    assert!(
        loader_files.is_empty(),
        "facade loader modules are limited to registry wiring: {loader_files:?}"
    );
}

fn collect_rust_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files
}

fn matches_any(path: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| {
        if pattern.ends_with('/') {
            path.starts_with(pattern)
        } else {
            path == *pattern
        }
    })
}

fn is_test_path(path: &str) -> bool {
    path.ends_with("_tests.rs") || path.ends_with("/tests.rs") || matches_any(path, TEST_SUPPORT)
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
