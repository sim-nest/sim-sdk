use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use sha2::{Digest, Sha256};

fn main() {
    for variable in [
        "RUSTC",
        "TARGET",
        "PROFILE",
        "OPT_LEVEL",
        "DEBUG",
        "CARGO_CFG_PANIC",
        "CARGO_CFG_TARGET_FEATURE",
        "CARGO_ENCODED_RUSTFLAGS",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    println!("cargo:rerun-if-changed=src");
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source = hash_source_inputs(&manifest_dir);
    let lock_path = find_lock(&manifest_dir)
        .expect("sim-conformance-packs requires an exact Cargo.lock build input");
    println!("cargo:rerun-if-changed={}", lock_path.display());
    let lock = hash_bytes(&fs::read(lock_path).expect("read dependency lock"));
    let rustc = env::var_os("RUSTC").expect("rustc executable");
    let toolchain = Command::new(rustc)
        .arg("-vV")
        .output()
        .expect("query rustc identity");
    assert!(toolchain.status.success(), "rustc -vV failed");
    let toolchain = hash_bytes(&toolchain.stdout);
    let mut features = env::vars()
        .filter_map(|(key, value)| key.starts_with("CARGO_FEATURE_").then_some((key, value)))
        .collect::<Vec<_>>();
    features.sort();
    let features = hash_lines(features.iter().map(|(key, value)| format!("{key}={value}")));
    let build_configuration = hash_lines(
        [
            "TARGET",
            "PROFILE",
            "OPT_LEVEL",
            "DEBUG",
            "CARGO_CFG_PANIC",
            "CARGO_CFG_TARGET_FEATURE",
            "CARGO_ENCODED_RUSTFLAGS",
        ]
        .into_iter()
        .map(|key| format!("{key}={}", env::var(key).unwrap_or_default())),
    );
    let exact = hash_lines([
        format!("source={source}"),
        format!("dependency-lock={lock}"),
        format!("toolchain={toolchain}"),
        format!("features={features}"),
        format!("build-configuration={build_configuration}"),
    ]);
    println!("cargo:rustc-env=SIM_CONFORMANCE_PACKS_SOURCE_SHA256={source}");
    println!("cargo:rustc-env=SIM_CONFORMANCE_PACKS_DEPENDENCY_LOCK_SHA256={lock}");
    println!("cargo:rustc-env=SIM_CONFORMANCE_PACKS_TOOLCHAIN_SHA256={toolchain}");
    println!("cargo:rustc-env=SIM_CONFORMANCE_PACKS_FEATURES_SHA256={features}");
    println!(
        "cargo:rustc-env=SIM_CONFORMANCE_PACKS_BUILD_CONFIGURATION_SHA256={build_configuration}"
    );
    println!("cargo:rustc-env=SIM_CONFORMANCE_PACKS_BUILD_INPUTS_SHA256={exact}");
}

fn hash_source_inputs(manifest_dir: &Path) -> String {
    let mut files = vec![
        manifest_dir.join("Cargo.toml"),
        manifest_dir.join("build.rs"),
    ];
    collect_files(&manifest_dir.join("src"), &mut files);
    files.sort();
    let mut hasher = Sha256::new();
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let relative = path.strip_prefix(manifest_dir).expect("owned source path");
        let name = relative.to_string_lossy();
        let bytes = fs::read(&path).expect("read checker source input");
        hasher.update((name.len() as u64).to_be_bytes());
        hasher.update(name.as_bytes());
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    hex(hasher.finalize())
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .expect("read source directory")
        .map(|entry| entry.expect("read source entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_files(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

fn find_lock(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|directory| directory.join("Cargo.lock"))
        .find(|path| path.is_file())
}

fn hash_lines(lines: impl IntoIterator<Item = String>) -> String {
    let mut hasher = Sha256::new();
    for line in lines {
        hasher.update((line.len() as u64).to_be_bytes());
        hasher.update(line.as_bytes());
    }
    hex(hasher.finalize())
}

fn hash_bytes(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes))
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
