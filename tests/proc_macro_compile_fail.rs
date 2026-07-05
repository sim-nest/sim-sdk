#![cfg(feature = "proc-macros")]

use std::path::{Path, PathBuf};
use std::process::Command;

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn unique_target_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("sim-proc-macro-ui-{nanos}"))
}

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/proc-macro-ui")
        .join(name)
}

fn native_abi_fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/native-abi-ui")
        .join(name)
}

const UI_PATCHES: &[(&str, &str, &str)] = &[
    ("sim", "sim-sdk", "."),
    ("sim-kernel", "sim-kernel", "."),
    ("sim-citizen", "sim-citizen", "crates/sim-citizen"),
    (
        "sim-citizen-derive",
        "sim-citizen",
        "crates/sim-citizen-derive",
    ),
    ("sim-run-loaders", "sim-cli", "crates/sim-run-loaders"),
    ("sim-cookbook", "sim-foundation", "crates/sim-cookbook"),
    ("sim-value", "sim-foundation", "crates/sim-value"),
    ("sim-lib-core", "sim-runtime", "crates/sim-lib-core"),
    ("sim-macros", "sim-foundation", "crates/sim-macros"),
    ("sim-shape", "sim-shape", "."),
];

fn local_patch_path(crate_name: &str, repo_name: &str, source_path: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if manifest_dir
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == "packages")
    {
        return manifest_dir
            .parent()
            .expect("meta-workspace package should have a packages parent")
            .join(crate_name);
    }

    if repo_name == "sim-sdk" {
        return manifest_dir.join(source_path);
    }
    manifest_dir
        .parent()
        .expect("sim-sdk checkout should have sibling repos")
        .join(repo_name)
        .join(source_path)
}

fn toml_string(path: &Path) -> String {
    let raw = path.to_string_lossy();
    format!("\"{}\"", raw.replace('\\', "\\\\").replace('"', "\\\""))
}

fn add_ui_patch_args(command: &mut Command) {
    for (crate_name, repo_name, source_path) in UI_PATCHES {
        let path = local_patch_path(crate_name, repo_name, source_path);
        command.arg("--config").arg(format!(
            "patch.crates-io.{crate_name}.path={}",
            toml_string(&path)
        ));
    }
}

fn check_fixture(manifest_path: PathBuf, target_dir: &Path) -> std::process::Output {
    let mut command = Command::new(cargo_bin());
    command
        .arg("check")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--target-dir")
        .arg(target_dir);
    add_ui_patch_args(&mut command);
    command
        .output()
        .expect("cargo check for proc-macro UI fixture should start")
}

fn stderr_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn remove_dir_all_if_exists(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

#[test]
fn invalid_shape_literal_fails_compilation() {
    let target_dir = unique_target_dir();
    let output = check_fixture(fixture_dir("invalid-shape").join("Cargo.toml"), &target_dir);

    assert!(
        !output.status.success(),
        "invalid-shape fixture unexpectedly compiled"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("unterminated list") || stderr.contains("unexpected end of input"),
        "expected shape parse failure in stderr, got:\n{stderr}"
    );

    remove_dir_all_if_exists(&target_dir);
}

#[test]
fn missing_constructor_fails_compilation() {
    let target_dir = unique_target_dir();
    let output = check_fixture(
        fixture_dir("missing-constructor").join("Cargo.toml"),
        &target_dir,
    );

    assert!(
        !output.status.success(),
        "missing-constructor fixture unexpectedly compiled"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("must have a matching #[sim_constructor]"),
        "expected missing constructor error in stderr, got:\n{stderr}"
    );

    remove_dir_all_if_exists(&target_dir);
}

#[test]
fn borrowed_args_cannot_be_passed_to_owned_destroy() {
    let target_dir = unique_target_dir();
    let output = check_fixture(
        native_abi_fixture_dir("borrow-to-destroy").join("Cargo.toml"),
        &target_dir,
    );

    assert!(
        !output.status.success(),
        "borrow-to-destroy fixture unexpectedly compiled"
    );
    let stderr = stderr_text(&output);
    assert!(
        stderr.contains("mismatched types") && stderr.contains("NativeAbiOwnedBytes"),
        "expected an ABI type mismatch, got:\n{stderr}"
    );

    remove_dir_all_if_exists(&target_dir);
}
