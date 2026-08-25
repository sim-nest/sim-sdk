#![cfg(feature = "hotload")]

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex, Weak},
};

use sim::{
    hotload::{
        ActivationRequest, ActivationService, AdmissionRequest, AdmissionService, BuildMounts,
        CompatibilityPolicy, HotloadGeneration, NativeBuildRequest, NativeBuilder, PreflightLimits,
        ToolchainIdentity,
    },
    kernel::{
        AbiVersion, Args, Callable, Cx, DefaultFactory, Dependency, EagerPolicy, Export,
        HandleSeed, Lib, LibManifest, LibSource, LibTarget, Linker, LoadCx, Object, ObjectCompat,
        Symbol, Value, Version,
    },
};
use sim_lib_exec::{
    ProcessCancellation, SandboxAttempt, SandboxControl, SandboxEvidence, SandboxLauncher,
    SandboxReport, SandboxResult,
};
use sim_lib_journal::MemoryBackend;
use sim_run_loaders::{LoadOutcome, LoadRequest, LoaderKind, LoaderPort, bytes_source};
use sim_storage_port::{HostDirPort, NeverCancel};
use sim_table_fs::MemoryHostDirPort;

const FIXTURE_A_MANIFEST: &[u8] =
    include_bytes!("../recipes/hotload/public-generation-loop/generation-a/Cargo.toml");
const FIXTURE_A_LOCK: &[u8] =
    include_bytes!("../recipes/hotload/public-generation-loop/generation-a/Cargo.lock");
const FIXTURE_A_SOURCE: &[u8] =
    include_bytes!("../recipes/hotload/public-generation-loop/generation-a/src/lib.rs");
const FIXTURE_B_MANIFEST: &[u8] =
    include_bytes!("../recipes/hotload/public-generation-loop/generation-b/Cargo.toml");
const FIXTURE_B_LOCK: &[u8] =
    include_bytes!("../recipes/hotload/public-generation-loop/generation-b/Cargo.lock");
const FIXTURE_B_SOURCE: &[u8] =
    include_bytes!("../recipes/hotload/public-generation-loop/generation-b/src/lib.rs");

fn path(value: &str) -> Vec<String> {
    value.split('/').map(str::to_owned).collect()
}

fn mount_fixture(label: &str, manifest: &[u8], lock: &[u8], source: &[u8]) -> MemoryHostDirPort {
    let mount = MemoryHostDirPort::new(label, 64 * 1024);
    mount
        .replace(&path("Cargo.toml"), manifest, &NeverCancel)
        .unwrap();
    mount
        .replace(&path("Cargo.lock"), lock, &NeverCancel)
        .unwrap();
    mount.create_dir(&path("src")).unwrap();
    mount
        .replace(&path("src/lib.rs"), source, &NeverCancel)
        .unwrap();
    mount
}

struct FixtureLauncher {
    artifact: Vec<u8>,
    target: Arc<MemoryHostDirPort>,
    escape: bool,
}

impl SandboxLauncher for FixtureLauncher {
    fn id(&self) -> &str {
        "sdk-fixture-sandbox"
    }

    fn launch(
        &self,
        _request: &sim_lib_exec::SandboxRequest,
        _cancellation: &ProcessCancellation,
    ) -> SandboxAttempt {
        if self.escape {
            return SandboxAttempt::Refused(sim_lib_exec::SandboxRefusal {
                launcher: self.id().into(),
                reason: "sandbox escape attempt refused".into(),
                report: None,
            });
        }
        self.target
            .create_dir(&path("debug"))
            .and_then(|_| {
                self.target.replace(
                    &path("debug/libsdk_hotload_fixture.so"),
                    &self.artifact,
                    &NeverCancel,
                )
            })
            .unwrap();
        let controls = [
            SandboxControl::Network,
            SandboxControl::Mounts,
            SandboxControl::Root,
            SandboxControl::Environment,
            SandboxControl::Identity,
            SandboxControl::Cpu,
            SandboxControl::Memory,
            SandboxControl::WallTime,
            SandboxControl::ProcessCount,
            SandboxControl::FileCount,
            SandboxControl::FileBytes,
            SandboxControl::Output,
            SandboxControl::Stdin,
            SandboxControl::ProcessTree,
        ]
        .into_iter()
        .map(|control| SandboxEvidence {
            control,
            achieved: true,
            detail: "deterministic modeled control".into(),
        })
        .collect();
        SandboxAttempt::Completed(SandboxResult {
            stdout: br#"{"reason":"compiler-artifact","package_id":"sdk-hotload-fixture 0.1.0 (path+file:///source)","target":{"kind":["cdylib"]},"filenames":["/target/debug/libsdk_hotload_fixture.so"]}"#.to_vec(),
            stderr: Vec::new(),
            exit_code: 0,
            report: SandboxReport {
                launcher: self.id().into(), controls, limit_hits: Vec::new(),
                cleanup: "no descendants remained".into(),
            },
        })
    }
}

fn build(
    source: &MemoryHostDirPort,
    artifacts: &MemoryHostDirPort,
    bytes: &[u8],
    mount_id: &str,
) -> sim::hotload::ArtifactCandidate {
    let target = Arc::new(MemoryHostDirPort::new(
        format!("target-{mount_id}"),
        64 * 1024,
    ));
    let launcher = FixtureLauncher {
        artifact: bytes.to_vec(),
        target: target.clone(),
        escape: false,
    };
    NativeBuilder::new(&launcher)
        .build(
            &NativeBuildRequest {
                source_mount: mount_id.into(),
                manifest: "Cargo.toml".into(),
                package: "sdk-hotload-fixture".into(),
                features: BTreeSet::new(),
                expected_library: Symbol::qualified("sdk-fixture", "library"),
                toolchain: ToolchainIdentity {
                    content: "sha256:sealed-rust-toolchain".into(),
                    cargo_program: "sealed-cargo".into(),
                    environment: vec![("PATH".into(), "/toolchain/bin".into())],
                },
            },
            BuildMounts {
                source,
                target: target.as_ref(),
                artifacts,
            },
            &ProcessCancellation::default(),
        )
        .unwrap()
}

#[derive(Default)]
struct LoaderState {
    last_a: Option<Weak<()>>,
}

struct FixtureLoader(Mutex<LoaderState>);

impl FixtureLoader {
    fn new() -> Self {
        Self(Mutex::new(LoaderState::default()))
    }
}

impl LoaderPort for FixtureLoader {
    fn loader_kinds(&self) -> Vec<LoaderKind> {
        vec![LoaderKind::new(Symbol::qualified("loader", "sdk-fixture"))]
    }

    fn realize(&self, _cx: &mut Cx, request: LoadRequest) -> sim::kernel::Result<LoadOutcome> {
        let bytes = sim_run_loaders::bytes_from_source(&request.source)?
            .ok_or_else(|| sim::kernel::Error::HostError("fixture requires bytes".into()))?;
        if bytes == b"failed-test" {
            return Err(sim::kernel::Error::Lib(
                "candidate test sdk-fixture/self-test did not pass".into(),
            ));
        }
        let generation = generation(&bytes)?;
        let token = Arc::new(());
        if generation == "alpha" {
            self.0.lock().unwrap().last_a = Some(Arc::downgrade(&token));
        }
        let library = FixtureLib {
            generation,
            token,
            removed: bytes == b"removed",
        };
        Ok(LoadOutcome {
            manifest: library.manifest(),
            library: Box::new(library),
        })
    }

    fn inspect(
        &self,
        _cx: &mut Cx,
        request: &LoadRequest,
    ) -> sim::kernel::Result<Option<LibManifest>> {
        let bytes = sim_run_loaders::bytes_from_source(&request.source)?
            .ok_or_else(|| sim::kernel::Error::HostError("fixture requires bytes".into()))?;
        if bytes == b"failed-test" {
            return Ok(Some(FixtureLib::manifest_for("alpha", false)));
        }
        Ok(Some(FixtureLib::manifest_for(
            generation(&bytes)?,
            bytes == b"removed",
        )))
    }
}

fn generation(bytes: &[u8]) -> sim::kernel::Result<&'static str> {
    match bytes {
        b"native-generation-alpha" => Ok("alpha"),
        b"native-generation-beta" => Ok("beta"),
        b"removed" => Ok("beta"),
        _ => Err(sim::kernel::Error::Lib(
            "unknown immutable fixture content".into(),
        )),
    }
}

struct FixtureLib {
    generation: &'static str,
    token: Arc<()>,
    removed: bool,
}

impl FixtureLib {
    fn manifest_for(_generation: &str, removed: bool) -> LibManifest {
        LibManifest {
            id: Symbol::qualified("sdk-fixture", "library"),
            version: Version("1.0.0".into()),
            abi: AbiVersion { major: 1, minor: 0 },
            target: LibTarget::Native,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: if removed {
                Vec::new()
            } else {
                vec![Export::Function {
                    symbol: Symbol::qualified("sdk-fixture", "value"),
                    function_id: None,
                }]
            },
        }
    }
}

impl Lib for FixtureLib {
    fn manifest(&self) -> LibManifest {
        Self::manifest_for(self.generation, self.removed)
    }
    fn load(&self, cx: &mut LoadCx, linker: &mut Linker<'_>) -> sim::kernel::Result<()> {
        if !self.removed {
            linker.function_value(
                Symbol::qualified("sdk-fixture", "value"),
                cx.factory().opaque(Arc::new(GenerationCallable {
                    value: self.generation,
                    token: self.token.clone(),
                }))?,
            )?;
        }
        Ok(())
    }
}

struct GenerationCallable {
    value: &'static str,
    token: Arc<()>,
}
impl Object for GenerationCallable {
    fn display(&self, _cx: &mut Cx) -> sim::kernel::Result<String> {
        Ok(format!("#<{}>", self.value))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl ObjectCompat for GenerationCallable {
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}
impl Callable for GenerationCallable {
    fn call(&self, cx: &mut Cx, args: Args) -> sim::kernel::Result<Value> {
        assert!(args.values().is_empty());
        let _keep_generation_alive = &self.token;
        cx.factory().string(self.value.to_owned())
    }
}

fn call(cx: &mut Cx, value: &Value) -> String {
    let result = value
        .object()
        .as_callable()
        .unwrap()
        .call(cx, Args::new(Vec::new()))
        .unwrap();
    result.object().display(cx).unwrap()
}

fn fresh_a(cx: &mut Cx) -> String {
    let value = cx
        .resolve_function(&Symbol::qualified("sdk-fixture", "value"))
        .unwrap();
    call(cx, &value)
}

fn admit<'a>(
    service: &AdmissionService<'_>,
    cx: &Cx,
    candidate: &'a sim::hotload::ArtifactCandidate,
    latest: Option<&'a HotloadGeneration>,
    bytes: Vec<u8>,
    seed: u64,
) -> Result<sim::hotload::AdmissionReceipt, sim::hotload::AdmissionFailure> {
    service.admit(
        cx,
        AdmissionRequest {
            candidate,
            loader_kind: LoaderKind::new(Symbol::qualified("loader", "sdk-fixture")),
            source: bytes_source(bytes),
            latest_generation: latest,
            compatibility: CompatibilityPolicy::Exact,
            dependency_receipts: &[],
            shadow_seed: HandleSeed(seed),
            limits: PreflightLimits {
                max_tests: 4,
                max_events_per_test: 8,
                max_detail_chars: 128,
            },
        },
    )
}

#[test]
fn public_build_to_generation_loop_is_safe_and_replayable() {
    assert!(
        std::str::from_utf8(FIXTURE_A_SOURCE)
            .unwrap()
            .contains("alpha")
    );
    assert!(
        std::str::from_utf8(FIXTURE_B_SOURCE)
            .unwrap()
            .contains("beta")
    );
    let source_a = mount_fixture(
        "source-a",
        FIXTURE_A_MANIFEST,
        FIXTURE_A_LOCK,
        FIXTURE_A_SOURCE,
    );
    let source_b = mount_fixture(
        "source-b",
        FIXTURE_B_MANIFEST,
        FIXTURE_B_LOCK,
        FIXTURE_B_SOURCE,
    );
    let artifacts = MemoryHostDirPort::new("immutable-artifacts", 64 * 1024);
    let candidate_a = build(
        &source_a,
        &artifacts,
        b"native-generation-alpha",
        "sha256:source-a",
    );
    let candidate_b = build(
        &source_b,
        &artifacts,
        b"native-generation-beta",
        "sha256:source-b",
    );
    assert_ne!(candidate_a.content, candidate_b.content);

    let loader = Arc::new(FixtureLoader::new());
    let admission = AdmissionService::new(&artifacts, loader.clone());
    let mut cx = Cx::new(
        Arc::new(EagerPolicy),
        Arc::new(DefaultFactory),
        HandleSeed(1),
    );
    let admitted_a = admit(
        &admission,
        &cx,
        &candidate_a,
        None,
        b"native-generation-alpha".to_vec(),
        10,
    )
    .unwrap();
    let backend = Arc::new(MemoryBackend::new());
    let mut activation = ActivationService::new(backend.clone(), loader.clone()).unwrap();
    let receipt_a = activation
        .activate(
            &mut cx,
            ActivationRequest {
                admission: &admitted_a,
                source: bytes_source(b"native-generation-alpha".to_vec()),
                loader_kind: LoaderKind::new(Symbol::qualified("loader", "sdk-fixture")),
            },
        )
        .unwrap();
    let captured_a = cx
        .resolve_function(&Symbol::qualified("sdk-fixture", "value"))
        .unwrap();
    assert_eq!(call(&mut cx, &captured_a), "alpha");

    let generation_a = HotloadGeneration {
        library: admitted_a.manifest.id.clone(),
        content: receipt_a.generation.clone(),
        manifest: admitted_a.manifest.clone(),
    };

    let removed = build(&source_b, &artifacts, b"removed", "sha256:removed");
    assert!(
        admit(
            &admission,
            &cx,
            &removed,
            Some(&generation_a),
            b"removed".to_vec(),
            11
        )
        .unwrap_err()
        .to_string()
        .contains("removed export")
    );
    assert_eq!(fresh_a(&mut cx), "alpha");

    let dependent = DependentLib;
    let dependent_id = cx.load_lib(&dependent).unwrap();
    assert!(
        admit(
            &admission,
            &cx,
            &candidate_b,
            Some(&generation_a),
            b"native-generation-beta".to_vec(),
            12
        )
        .unwrap_err()
        .to_string()
        .contains("dependents")
    );
    assert_eq!(fresh_a(&mut cx), "alpha");
    cx.unload_lib(dependent_id).unwrap();

    let failed = build(&source_b, &artifacts, b"failed-test", "sha256:failed-test");
    assert!(
        admit(
            &admission,
            &cx,
            &failed,
            Some(&generation_a),
            b"failed-test".to_vec(),
            13
        )
        .unwrap_err()
        .to_string()
        .contains("candidate realization failed")
    );
    assert_eq!(fresh_a(&mut cx), "alpha");

    let admitted_b = admit(
        &admission,
        &cx,
        &candidate_b,
        Some(&generation_a),
        b"native-generation-beta".to_vec(),
        14,
    )
    .unwrap();
    let mut stale = admitted_b.clone();
    stale.current_generation = Some(candidate_b.content.clone());
    assert!(
        activation
            .activate(
                &mut cx,
                ActivationRequest {
                    admission: &stale,
                    source: bytes_source(b"native-generation-beta".to_vec()),
                    loader_kind: LoaderKind::new(Symbol::qualified("loader", "sdk-fixture")),
                }
            )
            .unwrap_err()
            .to_string()
            .contains("stale expected-current")
    );
    assert_eq!(fresh_a(&mut cx), "alpha");

    let escaping_target = Arc::new(MemoryHostDirPort::new("escape-target", 1024));
    let escaping = FixtureLauncher {
        artifact: Vec::new(),
        target: escaping_target.clone(),
        escape: true,
    };
    let escape = NativeBuilder::new(&escaping)
        .build(
            &NativeBuildRequest {
                source_mount: "sha256:escape".into(),
                manifest: "Cargo.toml".into(),
                package: "sdk-hotload-fixture".into(),
                features: BTreeSet::new(),
                expected_library: Symbol::qualified("sdk-fixture", "library"),
                toolchain: ToolchainIdentity {
                    content: "sha256:toolchain".into(),
                    cargo_program: "sealed-cargo".into(),
                    environment: Vec::new(),
                },
            },
            BuildMounts {
                source: &source_b,
                target: escaping_target.as_ref(),
                artifacts: &artifacts,
            },
            &ProcessCancellation::default(),
        )
        .unwrap_err();
    assert_eq!(escape.kind, sim::hotload::FailureKind::SandboxRefusal);
    assert_eq!(fresh_a(&mut cx), "alpha");

    let receipt_b = activation
        .activate(
            &mut cx,
            ActivationRequest {
                admission: &admitted_b,
                source: bytes_source(b"native-generation-beta".to_vec()),
                loader_kind: LoaderKind::new(Symbol::qualified("loader", "sdk-fixture")),
            },
        )
        .unwrap();
    assert_eq!(fresh_a(&mut cx), "beta");
    assert_eq!(call(&mut cx, &captured_a), "alpha");
    let old_generation = loader.0.lock().unwrap().last_a.clone().unwrap();
    assert!(old_generation.upgrade().is_some());
    drop(captured_a);
    assert!(old_generation.upgrade().is_none());

    let mut replay = ActivationService::new(backend, loader.clone()).unwrap();
    replay
        .replay_completed([receipt_a.clone(), receipt_b.clone()])
        .unwrap();
    let mut fresh = Cx::new(
        Arc::new(EagerPolicy),
        Arc::new(DefaultFactory),
        HandleSeed(2),
    );
    let outcome = loader
        .realize(
            &mut fresh,
            LoadRequest {
                kind: LoaderKind::new(Symbol::qualified("loader", "sdk-fixture")),
                source: bytes_source(b"native-generation-beta".to_vec()),
            },
        )
        .unwrap();
    assert_eq!(outcome.manifest, admitted_b.manifest);
    fresh.load_lib(outcome.library.as_ref()).unwrap();
    assert_eq!(fresh_a(&mut fresh), "beta");
    assert_eq!(receipt_b.generation, candidate_b.content);
    assert_eq!(replay.status(), sim::hotload::ActivationStatus::Ready);
}

struct DependentLib;
impl Lib for DependentLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: Symbol::qualified("sdk-fixture", "dependent"),
            version: Version("1.0.0".into()),
            abi: AbiVersion { major: 1, minor: 0 },
            target: LibTarget::HostRegistered,
            requires: vec![Dependency {
                id: Symbol::qualified("sdk-fixture", "library"),
                minimum_version: None,
            }],
            capabilities: Vec::new(),
            exports: Vec::new(),
        }
    }
    fn load(&self, _cx: &mut LoadCx, _linker: &mut Linker<'_>) -> sim::kernel::Result<()> {
        Ok(())
    }
}
