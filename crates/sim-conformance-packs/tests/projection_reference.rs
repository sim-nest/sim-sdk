// conformance: NV12.06 projection packs exercise the released reference products.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    time::{Duration, Instant},
};

use sim_kernel::Datum;
use sim_lib_world::{
    DISCLOSURE_CONCLUSION, DISCLOSURE_FACT, SOURCE_CONCLUSION, SOURCE_FACT, WorldError,
    WorldProduct,
};
use sim_platform_ubuntu_pc::BwrapLauncher;
use sim_wasm_abi::{
    DeterministicWasmImport, ProjectionModuleError, ProjectionModulePolicy, WasmFrameLimits,
    inspect_projection_module,
};

#[test]
fn released_world_product_explains_but_refuses_unqualified_projection() {
    let world = WorldProduct::bundled().expect("released reference world must qualify");
    assert!(matches!(
        world.project(
            "world/public-api-v1",
            SOURCE_FACT,
            Datum::String("pub fn project()".into()),
            None,
        ),
        Err(WorldError::MissingIndependentSupport)
    ));

    let source_path = world
        .why(SOURCE_CONCLUSION, SOURCE_FACT)
        .expect("source explanation must exist");
    assert_eq!(
        source_path,
        world
            .why(SOURCE_CONCLUSION, SOURCE_FACT)
            .expect("repeated explanation must exist")
    );
    assert!(world.why(SOURCE_CONCLUSION, DISCLOSURE_FACT).is_err());
    assert!(world.why(DISCLOSURE_CONCLUSION, DISCLOSURE_FACT).is_ok());

    assert!(matches!(
        world.diff(
            "no-v3/disclosure-policy-v1",
            DISCLOSURE_FACT,
            Datum::String("internal".into()),
            Datum::String("public".into()),
        ),
        Err(WorldError::MissingIndependentSupport)
    ));
}

#[test]
fn released_world_why_meets_the_reference_latency_contract() {
    let world = WorldProduct::bundled().expect("released reference world must qualify");
    for _ in 0..20 {
        world
            .why(DISCLOSURE_CONCLUSION, DISCLOSURE_FACT)
            .expect("warmup explanation must pass");
    }
    let mut samples = Vec::with_capacity(200);
    for _ in 0..200 {
        let started = Instant::now();
        world
            .why(DISCLOSURE_CONCLUSION, DISCLOSURE_FACT)
            .expect("measured explanation must pass");
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    let p95 = samples[189];
    assert!(p95 <= Duration::from_millis(200), "p95 was {p95:?}");
}

#[test]
fn released_wasm_admission_refuses_ambient_and_inexact_imports() {
    let exact = DeterministicWasmImport::new("projection.input", "get");
    let bytes = wat::parse_str(
        r#"(module (import "projection.input" "get" (func (param i32) (result i32))))"#,
    )
    .expect("fixture must compile");
    let policy = ProjectionModulePolicy {
        imports: BTreeSet::from([exact.clone()]),
        allow_start: false,
        limits: WasmFrameLimits::default(),
    };
    let admission = inspect_projection_module(&bytes, &policy).expect("exact import must pass");
    assert_eq!(admission.imports, BTreeSet::from([exact]));

    let ambient = DeterministicWasmImport::new("wasi:random/random", "get-random-bytes");
    let ambient_bytes =
        wat::parse_str(r#"(module (import "wasi:random/random" "get-random-bytes" (func)))"#)
            .expect("fixture must compile");
    let ambient_policy = ProjectionModulePolicy {
        imports: BTreeSet::from([ambient.clone()]),
        ..policy.clone()
    };
    assert_eq!(
        inspect_projection_module(&ambient_bytes, &ambient_policy),
        Err(ProjectionModuleError::ForbiddenImport(ambient))
    );

    let absent = DeterministicWasmImport::new("projection.input", "list");
    let mismatch = ProjectionModulePolicy {
        imports: BTreeSet::from([absent.clone()]),
        ..policy
    };
    assert!(matches!(
        inspect_projection_module(&bytes, &mismatch),
        Err(ProjectionModuleError::ImportManifestMismatch { missing, undeclared })
            if missing == BTreeSet::from([absent]) && undeclared.len() == 1
    ));
}

#[test]
fn released_bwrap_status_cannot_claim_projector_purity() {
    let launcher = BwrapLauncher::new(
        PathBuf::from("/definitely/missing/nv12-bwrap"),
        PathBuf::from("/definitely/missing/nv12-prlimit"),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let status = launcher.confinement_status();
    assert_eq!(status.membrane, "platform/sandbox/ubuntu-bwrap");
    assert!(!status.available);
    assert!(status.detail.contains("purity-qualified=false"));
}
