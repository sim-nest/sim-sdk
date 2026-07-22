#![cfg(all(
    feature = "dynamic-native",
    feature = "proc-macros",
    not(target_arch = "wasm32")
))]
#![allow(deprecated)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

use sim::kernel::{EncodeOptions, ReadPolicy};
use sim::{
    codec::{Input, Output, decode_with_codec, encode_with_codec},
    kernel::{
        Args, DefaultFactory, EagerPolicy, Expr, Symbol, macro_expand_capability,
        native_dynamic_load_capability,
    },
    loaders::standard_loader_registry,
    runtime::install_core_runtime,
};

const NATIVE_PLUGIN_PATCHES: &[(&str, &str, &str)] = &[
    ("sim-nest", "sim-sdk", "."),
    ("sim-kernel", "sim-kernel", "."),
    ("sim-citizen", "sim-citizen", "crates/sim-citizen"),
    (
        "sim-citizen-derive",
        "sim-citizen",
        "crates/sim-citizen-derive",
    ),
    ("sim-run-loaders", "sim-run", "crates/sim-run-loaders"),
    ("sim-cookbook", "sim-foundation", "crates/sim-cookbook"),
    ("sim-value", "sim-foundation", "crates/sim-value"),
    ("sim-macros", "sim-foundation", "crates/sim-macros"),
    ("sim-shape", "sim-shape", "."),
    ("sim-codec", "sim-codecs", "crates/sim-codec"),
    ("sim-codec-binary", "sim-codecs", "crates/sim-codec-binary"),
    ("sim-lib-core", "sim-runtime", "crates/sim-lib-core"),
    (
        "sim-lib-numbers-core",
        "sim-numbers",
        "crates/sim-lib-numbers-core",
    ),
    (
        "sim-lib-numbers-arith",
        "sim-numbers",
        "crates/sim-lib-numbers-arith",
    ),
    (
        "sim-lib-numbers-f64",
        "sim-numbers",
        "crates/sim-lib-numbers-f64",
    ),
];

const NATIVE_NUMBERS_F64_PATCHES: &[(&str, &str, &str)] = &[
    ("sim-kernel", "sim-kernel", "."),
    ("sim-citizen", "sim-citizen", "crates/sim-citizen"),
    (
        "sim-citizen-derive",
        "sim-citizen",
        "crates/sim-citizen-derive",
    ),
    ("sim-cookbook", "sim-foundation", "crates/sim-cookbook"),
    ("sim-value", "sim-foundation", "crates/sim-value"),
    ("sim-macros", "sim-foundation", "crates/sim-macros"),
    ("sim-shape", "sim-shape", "."),
    ("sim-codec", "sim-codecs", "crates/sim-codec"),
    ("sim-codec-binary", "sim-codecs", "crates/sim-codec-binary"),
];

const NATIVE_STANDARD_CORE_PATCHES: &[(&str, &str, &str)] = &[
    ("sim-kernel", "sim-kernel", "."),
    ("sim-citizen", "sim-citizen", "crates/sim-citizen"),
    (
        "sim-citizen-derive",
        "sim-citizen",
        "crates/sim-citizen-derive",
    ),
    ("sim-cookbook", "sim-foundation", "crates/sim-cookbook"),
    ("sim-value", "sim-foundation", "crates/sim-value"),
    ("sim-shape", "sim-shape", "."),
    ("sim-codec", "sim-codecs", "crates/sim-codec"),
    ("sim-codec-binary", "sim-codecs", "crates/sim-codec-binary"),
    ("sim-lib-core", "sim-runtime", "crates/sim-lib-core"),
];

fn cx() -> sim::kernel::Cx {
    let mut cx = sim::kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn plugin_manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/native-plugin")
}

fn numbers_f64_manifest_path() -> PathBuf {
    local_patch_path(
        "sim-lib-numbers-f64",
        "sim-numbers",
        "crates/sim-lib-numbers-f64",
    )
    .join("Cargo.toml")
}

fn standard_core_manifest_path() -> PathBuf {
    local_patch_path(
        "sim-lib-standard-core",
        "sim-runtime",
        "crates/sim-lib-standard-core",
    )
    .join("Cargo.toml")
}

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn unique_target_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("sim-native-plugin-{nanos}"))
}

fn dylib_file_name(base: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!("{base}.dll")
    }
    #[cfg(target_os = "macos")]
    {
        format!("lib{base}.dylib")
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        format!("lib{base}.so")
    }
}

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

fn add_native_plugin_patch_args(command: &mut Command, patches: &[(&str, &str, &str)]) {
    for (crate_name, repo_name, source_path) in patches {
        let path = local_patch_path(crate_name, repo_name, source_path);
        command.arg("--config").arg(format!(
            "patch.crates-io.{crate_name}.path={}",
            toml_string(&path)
        ));
    }
}

fn native_build_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct CargoLockSnapshot {
    path: PathBuf,
    contents: Option<Vec<u8>>,
}

impl CargoLockSnapshot {
    fn capture(manifest_path: &Path) -> Self {
        let path = cargo_lock_path(manifest_path);
        let contents = std::fs::read(&path).ok();
        Self { path, contents }
    }
}

impl Drop for CargoLockSnapshot {
    fn drop(&mut self) {
        match &self.contents {
            Some(contents) => {
                std::fs::write(&self.path, contents).expect("Cargo.lock snapshot should restore");
            }
            None => {
                if self.path.exists() {
                    std::fs::remove_file(&self.path).expect("generated Cargo.lock should remove");
                }
            }
        }
    }
}

fn cargo_lock_path(manifest_path: &Path) -> PathBuf {
    let mut current = manifest_path
        .parent()
        .expect("manifest path should have a parent");
    loop {
        let lock_path = current.join("Cargo.lock");
        if lock_path.exists() {
            return lock_path;
        }
        let Some(parent) = current.parent() else {
            return manifest_path
                .parent()
                .expect("manifest path should have a parent")
                .join("Cargo.lock");
        };
        current = parent;
    }
}

fn refresh_native_lockfile(manifest_path: &Path, patches: &[(&str, &str, &str)]) {
    let mut command = Command::new(cargo_bin());
    command
        .arg("update")
        .arg("--manifest-path")
        .arg(manifest_path);
    add_native_plugin_patch_args(&mut command, patches);
    let status = command
        .status()
        .unwrap_or_else(|err| panic!("cargo update for native fixture should start: {err}"));
    assert!(status.success(), "native fixture dependency resolve failed");
}

fn build_native_plugin() -> Option<PathBuf> {
    build_native_dylib(
        plugin_manifest_dir().join("Cargo.toml"),
        "sim-native-plugin",
        "native_plugin_fixture",
        &[],
        NATIVE_PLUGIN_PATCHES,
    )
}

fn build_native_numbers_f64() -> Option<PathBuf> {
    build_native_dylib(
        numbers_f64_manifest_path(),
        "sim-native-numbers-f64",
        "sim_lib_numbers_f64",
        &["native-export"],
        NATIVE_NUMBERS_F64_PATCHES,
    )
}

fn build_native_standard_core() -> Option<PathBuf> {
    build_native_dylib(
        standard_core_manifest_path(),
        "sim-native-standard-core",
        "sim_lib_standard_core",
        &["native-export"],
        NATIVE_STANDARD_CORE_PATCHES,
    )
}

fn build_native_dylib(
    manifest_path: PathBuf,
    target_prefix: &str,
    dylib_base: &str,
    features: &[&str],
    patches: &[(&str, &str, &str)],
) -> Option<PathBuf> {
    if let Some(missing) = missing_native_build_input(&manifest_path, patches) {
        eprintln!(
            "skipping {target_prefix}: required local manifest is absent at {}",
            missing.display()
        );
        return None;
    }

    let _lock = native_build_lock()
        .lock()
        .expect("native fixture build lock should not be poisoned");
    let _lockfile = CargoLockSnapshot::capture(&manifest_path);
    refresh_native_lockfile(&manifest_path, patches);
    build_native_dylib_locked(&manifest_path, target_prefix, dylib_base, features, patches)
}

fn build_native_dylib_locked(
    manifest_path: &Path,
    target_prefix: &str,
    dylib_base: &str,
    features: &[&str],
    patches: &[(&str, &str, &str)],
) -> Option<PathBuf> {
    let target_dir = unique_target_dir();
    let mut command = Command::new(cargo_bin());
    command
        .env("RUSTFLAGS", "-D warnings")
        .arg("build")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--target-dir")
        .arg(&target_dir);
    if !features.is_empty() {
        command.arg("--features").arg(features.join(","));
    }
    add_native_plugin_patch_args(&mut command, patches);
    let status = command
        .status()
        .unwrap_or_else(|err| panic!("cargo build for {target_prefix} should start: {err}"));
    assert!(status.success(), "{target_prefix} build failed");
    Some(target_dir.join("debug").join(dylib_file_name(dylib_base)))
}

fn missing_native_build_input(
    manifest_path: &Path,
    patches: &[(&str, &str, &str)],
) -> Option<PathBuf> {
    if !manifest_path.is_file() {
        return Some(manifest_path.to_path_buf());
    }

    for (crate_name, repo_name, source_path) in patches {
        let manifest = local_patch_path(crate_name, repo_name, source_path).join("Cargo.toml");
        if !manifest.is_file() {
            return Some(manifest);
        }
    }

    None
}

fn remove_dir_all_if_exists(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

#[test]
fn native_loader_can_build_and_load_external_plugin_dylib() {
    let Some(plugin_path) = build_native_plugin() else {
        return;
    };
    assert!(
        plugin_path.is_file(),
        "missing plugin dylib {plugin_path:?}"
    );

    let target_dir = plugin_path
        .parent()
        .and_then(Path::parent)
        .expect("plugin dylib should live in target/<profile>");

    let mut cx = cx();
    cx.grant(native_dynamic_load_capability());
    let registry = standard_loader_registry();

    registry
        .load_and_register(&mut cx, sim::loaders::path_source(plugin_path.clone()))
        .unwrap();

    let hello = cx
        .call_function(&Symbol::new("native-hello"), Args::new(Vec::new()))
        .unwrap();
    assert_eq!(
        hello.object().as_expr(&mut cx).unwrap(),
        Expr::String("hello from native".to_owned())
    );

    let described = cx
        .call_function(
            &Symbol::new("native-describe"),
            Args::new(vec![cx.factory().string("payload".to_owned()).unwrap()]),
        )
        .unwrap();
    assert_eq!(
        described.object().as_expr(&mut cx).unwrap(),
        Expr::String("native:String(\"payload\")".to_owned())
    );

    assert!(cx.registry().lib(&Symbol::new("native-fixture")).is_some());

    let codec = Symbol::qualified("codec", "native-fixture");
    let decoded = decode_with_codec(
        &mut cx,
        &codec,
        Input::Text("(+ 1 2)".to_owned()),
        ReadPolicy::default(),
    )
    .unwrap();
    assert_eq!(
        decoded,
        Expr::List(vec![
            Expr::Symbol(Symbol::qualified("native", "decoded")),
            Expr::String("(+ 1 2)".to_owned()),
        ])
    );

    let encoded = encode_with_codec(
        &mut cx,
        &codec,
        &Expr::Symbol(Symbol::qualified("native", "decoded")),
        EncodeOptions::default(),
    )
    .unwrap();
    assert_eq!(encoded, Output::Text("encoded:native/decoded".to_owned()));

    remove_dir_all_if_exists(target_dir);
}

#[cfg(feature = "numbers-arith")]
#[test]
fn native_loader_can_load_f64_number_domain_dylib() {
    let Some(plugin_path) = build_native_numbers_f64() else {
        return;
    };
    assert!(
        plugin_path.is_file(),
        "missing numbers f64 dylib {plugin_path:?}"
    );

    let target_dir = plugin_path
        .parent()
        .and_then(Path::parent)
        .expect("plugin dylib should live in target/<profile>");

    let mut cx = sim::kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    cx.grant(native_dynamic_load_capability());
    cx.load_lib(&sim::numbers_arith::NumbersArithmeticLib::new())
        .unwrap();
    let registry = standard_loader_registry();

    registry
        .load_and_register(&mut cx, sim::loaders::path_source(plugin_path.clone()))
        .unwrap();

    let domain = Symbol::qualified("numbers", "f64");
    assert!(cx.registry().number_domain_by_symbol(&domain).is_some());
    let parsed = cx.parse_number_literal("1.5").unwrap().unwrap();
    assert_eq!(parsed.domain, domain);
    assert_eq!(parsed.canonical, "1.5");

    let one = cx
        .factory()
        .number_literal(domain.clone(), "1".to_owned())
        .unwrap();
    let two = cx
        .factory()
        .number_literal(domain.clone(), "2".to_owned())
        .unwrap();
    let added = cx
        .call_function(&Symbol::new("+"), Args::new(vec![one, two]))
        .unwrap();
    assert_eq!(
        added.object().as_expr(&mut cx).unwrap(),
        Expr::Number(sim::kernel::NumberLiteral {
            domain,
            canonical: "3".to_owned(),
        })
    );

    remove_dir_all_if_exists(target_dir);
}

#[test]
fn native_loader_can_load_standard_core_class_and_macro_dylib() {
    let Some(plugin_path) = build_native_standard_core() else {
        return;
    };
    assert!(
        plugin_path.is_file(),
        "missing standard-core dylib {plugin_path:?}"
    );

    let target_dir = plugin_path
        .parent()
        .and_then(Path::parent)
        .expect("plugin dylib should live in target/<profile>");

    let mut cx = cx();
    cx.grant(native_dynamic_load_capability());
    cx.grant(macro_expand_capability());
    let registry = standard_loader_registry();

    registry
        .load_and_register(&mut cx, sim::loaders::path_source(plugin_path.clone()))
        .unwrap();

    let class = Symbol::qualified("standard", "proof-box");
    assert!(cx.registry().class_by_symbol(&class).is_some());
    let instance = cx
        .call_class(
            &class,
            Args::new(vec![cx.factory().string("loaded".to_owned()).unwrap()]),
        )
        .unwrap();
    let value = cx
        .call_function(
            &Symbol::qualified("standard/proof-box", "value"),
            Args::new(vec![instance]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::String("loaded".to_owned())
    );

    let expanded = cx
        .expand_macros(
            sim::kernel::Phase::Expand,
            Expr::List(vec![
                Expr::Symbol(Symbol::qualified("standard", "proof-quote")),
                Expr::String("macro-loaded".to_owned()),
            ]),
        )
        .unwrap();
    assert_eq!(expanded, Expr::String("macro-loaded".to_owned()));

    remove_dir_all_if_exists(target_dir);
}

#[test]
fn native_loader_rejects_extra_args_with_generated_arity_check() {
    let Some(plugin_path) = build_native_plugin() else {
        return;
    };
    let target_dir = plugin_path
        .parent()
        .and_then(Path::parent)
        .expect("plugin dylib should live in target/<profile>");

    let mut cx = cx();
    cx.grant(native_dynamic_load_capability());
    let registry = standard_loader_registry();

    registry
        .load_and_register(&mut cx, sim::loaders::path_source(plugin_path.clone()))
        .unwrap();

    let error = cx
        .call_function(
            &Symbol::new("native-hello"),
            Args::new(vec![cx.factory().string("extra".to_owned()).unwrap()]),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        sim::kernel::Error::HostError(message)
            if message.contains("native-hello expects 0 args, got 1")
    ));

    remove_dir_all_if_exists(target_dir);
}
