use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

use sim::kernel::{
    AbiVersion, Args, Callable, CatalogSource, Cx, DefaultFactory, EagerPolicy, Error, Export,
    ExportRecord, ExportState, Lib, LibLoader, LibManifest, LibSource, LibSourceSpec, LibTarget,
    Linker, LoadCx, LoaderRegistry, Object, ObjectCompat, RegistryBootState, Result, Symbol, Value,
    Version,
};

#[test]
fn cli_boot_receipts_replay_to_same_loaded_surface() {
    let codec_source = Symbol::qualified("codec", "lisp");
    let scenario_source = Symbol::qualified("cli", "scenario-source");
    let loaders = LoaderRegistry::new()
        .with_loader(CliBootFixtureLoader)
        .with_source(
            codec_source.clone(),
            CatalogSource::Bytes(b"codec-lisp".to_vec()),
        )
        .with_source(
            scenario_source.clone(),
            CatalogSource::Bytes(b"scenario-app".to_vec()),
        );
    let mut recorded = cx();

    let codec_receipt = loaders
        .load_and_register_with_receipt(&mut recorded, LibSourceSpec::Symbol(codec_source))
        .unwrap();
    let app_receipt = loaders
        .load_and_register_with_receipt(&mut recorded, LibSourceSpec::Symbol(scenario_source))
        .unwrap();
    let state = RegistryBootState::new(vec![codec_receipt, app_receipt]);
    let encoded = state.to_datum();
    encoded.canonical_bytes().unwrap();
    let decoded = RegistryBootState::from_datum(&encoded).unwrap();
    assert_eq!(decoded, state);

    let mut replayed = cx();
    assert_eq!(
        loaders.replay_boot_state(&mut replayed, &decoded).unwrap(),
        state.receipts
    );
    assert_eq!(cli_loaded_surface(&replayed), cli_loaded_surface(&recorded));
    assert!(
        recorded
            .resolve_codec(&Symbol::qualified("codec", "lisp"))
            .is_ok()
    );
    assert!(
        recorded
            .resolve_function(&Symbol::qualified("cli", "main"))
            .is_ok()
    );
    assert!(
        recorded
            .registry()
            .value_by_symbol(&Symbol::qualified("placement", "report"))
            .is_some()
    );
}

#[test]
#[ignore = "requires the generated constellation meta-workspace and native dynamic plugin builds"]
fn sim_repl_evaluates_through_bootloader_surface() {
    let meta_manifest = meta_workspace_manifest()
        .expect("bootloader REPL check must run from the generated constellation meta-workspace");
    let target_dir = unique_target_dir();
    build_native_dylib(
        &meta_manifest,
        "sim-codec-lisp",
        &["native-export"],
        &target_dir,
    );
    build_native_dylib(
        &meta_manifest,
        "sim-lib-numbers-f64",
        &["native-export"],
        &target_dir,
    );
    build_native_dylib(
        &meta_manifest,
        "sim-lib-standard-core",
        &["native-export"],
        &target_dir,
    );

    let bundle_dir = target_dir.join("debug");
    assert!(bundle_dir.join(dylib_file_name("sim_codec_lisp")).is_file());
    assert!(
        bundle_dir
            .join(dylib_file_name("sim_lib_numbers_f64"))
            .is_file()
    );
    assert!(
        bundle_dir
            .join(dylib_file_name("sim_lib_standard_core"))
            .is_file()
    );

    let mut command = Command::new(cargo_bin());
    command
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(&meta_manifest)
        .arg("-p")
        .arg("sim-run")
        .arg("--features")
        .arg("dynamic-native")
        .arg("--bin")
        .arg("sim")
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--")
        .arg("repl")
        .env("SIM_REPL_BUNDLE_DIR", &bundle_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("sim repl should start");
    child
        .stdin
        .as_mut()
        .expect("sim repl stdin should be piped")
        .write_all(b"42\n")
        .expect("write repl input");
    let output = child.wait_with_output().expect("wait for sim repl");

    remove_dir_all_if_exists(&target_dir);

    assert!(
        output.status.success(),
        "sim repl failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "42\n");
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "");
}

fn build_native_dylib(meta_manifest: &Path, package: &str, features: &[&str], target_dir: &Path) {
    let mut command = Command::new(cargo_bin());
    command
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("RUSTFLAGS", "-D warnings")
        .arg("build")
        .arg("--manifest-path")
        .arg(meta_manifest)
        .arg("-p")
        .arg(package);
    if !features.is_empty() {
        command.arg("--features").arg(features.join(","));
    }
    command.arg("--target-dir").arg(target_dir);

    let status = command
        .status()
        .unwrap_or_else(|err| panic!("cargo build for {package} should start: {err}"));
    assert!(status.success(), "{package} native dylib build failed");
}

fn meta_workspace_manifest() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if manifest_dir
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == "packages")
    {
        return manifest_dir
            .parent()
            .and_then(Path::parent)
            .map(|root| root.join("Cargo.toml"));
    }
    None
}

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

fn unique_target_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("sim-conformance-repl-bundle-{nanos}"))
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

fn remove_dir_all_if_exists(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

fn cx() -> Cx {
    Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory))
}

struct CliBootFixtureLoader;

impl LibLoader for CliBootFixtureLoader {
    fn can_load(&self, source: &LibSource) -> bool {
        matches!(
            source,
            LibSource::Bytes(bytes) if matches!(bytes.as_slice(), b"codec-lisp" | b"scenario-app")
        )
    }

    fn load(&self, _cx: &mut Cx, source: LibSource) -> Result<Box<dyn Lib>> {
        match source {
            LibSource::Bytes(bytes) if bytes == b"codec-lisp" => Ok(Box::new(CliBootFixtureLib {
                kind: FixtureKind::Codec,
            })),
            LibSource::Bytes(bytes) if bytes == b"scenario-app" => {
                Ok(Box::new(CliBootFixtureLib {
                    kind: FixtureKind::App,
                }))
            }
            _ => Err(Error::HostError("unsupported CLI boot fixture".to_owned())),
        }
    }
}

struct CliBootFixtureLib {
    kind: FixtureKind,
}

enum FixtureKind {
    Codec,
    App,
}

impl Lib for CliBootFixtureLib {
    fn manifest(&self) -> LibManifest {
        match self.kind {
            FixtureKind::Codec => LibManifest {
                id: Symbol::new("codec-lisp-fixture"),
                version: Version("0.1.0".to_owned()),
                abi: AbiVersion { major: 0, minor: 1 },
                target: LibTarget::HostRegistered,
                requires: Vec::new(),
                capabilities: Vec::new(),
                exports: vec![Export::Codec {
                    symbol: Symbol::qualified("codec", "lisp"),
                    codec_id: None,
                }],
            },
            FixtureKind::App => LibManifest {
                id: Symbol::new("cli-scenario-fixture"),
                version: Version("0.1.0".to_owned()),
                abi: AbiVersion { major: 0, minor: 1 },
                target: LibTarget::HostRegistered,
                requires: Vec::new(),
                capabilities: Vec::new(),
                exports: vec![
                    Export::Function {
                        symbol: Symbol::qualified("cli", "main"),
                        function_id: None,
                    },
                    Export::Value {
                        symbol: Symbol::qualified("placement", "report"),
                    },
                ],
            },
        }
    }

    fn load(&self, cx: &mut LoadCx, linker: &mut Linker<'_>) -> Result<()> {
        match self.kind {
            FixtureKind::Codec => {
                linker.codec_value(Symbol::qualified("codec", "lisp"), cx.factory().bool(true)?)?;
                Ok(())
            }
            FixtureKind::App => {
                linker.function_value(
                    Symbol::qualified("cli", "main"),
                    cx.factory().opaque(Arc::new(CliMainFixture))?,
                )?;
                linker.value(
                    Symbol::qualified("placement", "report"),
                    cx.factory().string(
                        "(placement-report (node fx) (site local) (latency sample-exact))"
                            .to_owned(),
                    )?,
                )
            }
        }
    }
}

struct CliMainFixture;

impl Object for CliMainFixture {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("cli/main".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ObjectCompat for CliMainFixture {
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for CliMainFixture {
    fn call(&self, cx: &mut Cx, _args: Args) -> Result<Value> {
        cx.factory().bool(true)
    }
}

fn cli_loaded_surface(cx: &Cx) -> Vec<(String, Vec<String>)> {
    cx.registry()
        .libs()
        .iter()
        .map(|loaded| {
            (
                loaded.manifest.id.as_qualified_str(),
                loaded.exports.iter().map(record_label).collect(),
            )
        })
        .collect()
}

fn record_label(record: &ExportRecord) -> String {
    format!(
        "{}:{}:{}",
        record.kind.symbol().as_qualified_str(),
        record.symbol.as_qualified_str(),
        state_label(&record.state)
    )
}

fn state_label(state: &ExportState) -> String {
    match state {
        ExportState::Resolved { id } => format!("resolved:{id:?}"),
        ExportState::Declared => "declared".to_owned(),
        ExportState::Unsupported { reason } => format!("unsupported:{reason}"),
        ExportState::Invalid { error } => format!("invalid:{error}"),
    }
}
