use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use sha2::{Digest, Sha256};

fn main() {
    for variable in [
        "SIM_CONFORMANCE_PACKS_LOCK_FILE",
        "SIM_CONFORMANCE_PACKS_LOCK_SHA256",
        "CARGO_FEATURE_EXTERNAL_RESOLVER_LOCK",
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
    let path_dependencies = hash_path_dependencies(&manifest_dir);
    let (lock_path, lock_bytes) = selected_lock(&manifest_dir);
    println!("cargo:rerun-if-changed={}", lock_path.display());
    let lock = hash_bytes(&lock_bytes);
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
        format!("path-dependencies={path_dependencies}"),
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

fn hash_path_dependencies(manifest_dir: &Path) -> String {
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("pack manifest");
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    for dependency in path_dependencies(&manifest) {
        walk_manifest(&manifest_dir.join(dependency), &mut files, &mut seen);
    }
    files.sort();
    files.dedup();
    let mut hasher = Sha256::new();
    for (label, path) in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = fs::read(&path).expect("read path dependency");
        hasher.update((label.len() as u64).to_be_bytes());
        hasher.update(label.as_bytes());
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    hex(hasher.finalize())
}

fn path_dependencies(manifest: &str) -> Vec<String> {
    let mut section = String::new();
    let mut paths = Vec::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix('[') {
            section = name.trim_end_matches(']').to_owned();
            continue;
        }
        if section == "dependencies"
            && let Some(path) = path_from_line(trimmed)
        {
            paths.push(path);
        }
    }
    paths
}

fn path_from_line(line: &str) -> Option<String> {
    let (_, after) = line.split_once("path")?;
    let (_, after) = after.split_once('"')?;
    let (path, _) = after.split_once('"')?;
    if path.is_empty() {
        None
    } else {
        Some(path.to_owned())
    }
}

fn walk_manifest(
    manifest_dir: &Path,
    files: &mut Vec<(String, PathBuf)>,
    seen: &mut BTreeSet<PathBuf>,
) {
    let manifest_dir = manifest_dir
        .canonicalize()
        .expect("canonical path dependency");
    if !seen.insert(manifest_dir.clone()) {
        return;
    }
    collect_crate_sources(&manifest_dir, files);
    let manifest =
        fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("dependency manifest");
    for dependency in path_dependencies(&manifest) {
        walk_manifest(&manifest_dir.join(dependency), files, seen);
    }
}

fn collect_crate_sources(manifest_dir: &Path, files: &mut Vec<(String, PathBuf)>) {
    let crate_name = manifest_dir
        .file_name()
        .expect("crate directory")
        .to_string_lossy();
    let manifest = manifest_dir.join("Cargo.toml");
    files.push((format!("{crate_name}/Cargo.toml"), manifest));
    let source = manifest_dir.join("src");
    if source.is_dir() {
        let mut found = Vec::new();
        collect_files(&source, &mut found);
        for path in found {
            let relative = path
                .strip_prefix(manifest_dir)
                .expect("crate source path")
                .to_string_lossy();
            files.push((format!("{crate_name}/{relative}"), path));
        }
    }
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

fn selected_lock(manifest_dir: &Path) -> (PathBuf, Vec<u8>) {
    let selected = env::var_os("SIM_CONFORMANCE_PACKS_LOCK_FILE");
    let expected = env::var("SIM_CONFORMANCE_PACKS_LOCK_SHA256").ok();
    match (selected, expected) {
        (Some(path), Some(expected)) => {
            assert!(
                expected.len() == 64
                    && expected
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
                "SIM_CONFORMANCE_PACKS_LOCK_SHA256 must be lowercase SHA-256"
            );
            let path = PathBuf::from(path);
            assert!(
                path.is_absolute(),
                "SIM_CONFORMANCE_PACKS_LOCK_FILE must be absolute"
            );
            let metadata = fs::symlink_metadata(&path).expect("inspect selected dependency lock");
            assert!(
                metadata.file_type().is_file() && !metadata.file_type().is_symlink(),
                "selected dependency lock must be a regular non-symlink file"
            );
            let bytes = fs::read(&path).expect("read selected dependency lock");
            let actual = hash_bytes(&bytes);
            assert_eq!(actual, expected, "selected dependency lock digest differs");
            (path, bytes)
        }
        (None, None) => {
            assert!(
                env::var_os("CARGO_FEATURE_EXTERNAL_RESOLVER_LOCK").is_none(),
                "external-resolver-lock requires an explicit lock path and digest"
            );
            let packaged = manifest_dir.join("Cargo.lock");
            if packaged.is_file() {
                let bytes = fs::read(&packaged).expect("read packaged dependency lock");
                return (packaged, bytes);
            }
            let path = find_lock(manifest_dir)
                .expect("sim-conformance-packs requires an exact Cargo.lock build input");
            let bytes = fs::read(&path).expect("read primary dependency lock");
            (path, bytes)
        }
        _ => panic!(
            "SIM_CONFORMANCE_PACKS_LOCK_FILE and SIM_CONFORMANCE_PACKS_LOCK_SHA256 must be supplied together"
        ),
    }
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
