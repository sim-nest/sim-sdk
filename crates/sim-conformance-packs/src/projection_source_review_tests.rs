use super::*;
use sim_conformance_core::LiveCheckerError;
use sim_lib_hotload::CargoResolverIdentity;

const LOCK: &[u8] = b"version = 4\n";
const CONFIGURATION: &[u8] = b"[net]\noffline = true\n[source.crates-io]\nreplace-with = 'vendored-sources'\n[source.vendored-sources]\ndirectory = '/toolchain/vendor'\n";

fn id(value: &str) -> ContentId {
    Datum::String(value.into()).content_id().unwrap()
}

fn input(
    origin: ConsumedBuildInputOrigin,
    path: &str,
    content: &str,
    role: ReviewedInputRole,
) -> ReviewedConsumedInput {
    ReviewedConsumedInput {
        origin,
        immutable_resource: None,
        path: path.into(),
        content: id(content),
        role,
    }
}

fn snapshot() -> ReviewedProjectionSource {
    ReviewedProjectionSource {
        request: id("request"),
        invocation: id("build invocation"),
        readable_inputs: id("readable inputs"),
        consumed_inputs: id("consumed-input ledger"),
        inputs: vec![
            input(
                ConsumedBuildInputOrigin::Source,
                "Cargo.toml",
                "manifest",
                ReviewedInputRole::CargoManifest,
            ),
            input(
                ConsumedBuildInputOrigin::Source,
                "Cargo.lock",
                "resolver lock",
                ReviewedInputRole::ResolverLock,
            ),
            input(
                ConsumedBuildInputOrigin::Source,
                "src/lib.rs",
                "implementation",
                ReviewedInputRole::RustSource,
            ),
            input(
                ConsumedBuildInputOrigin::Toolchain,
                "bin/rustc",
                "toolchain",
                ReviewedInputRole::ToolchainTrustRoot,
            ),
        ],
        artifact: id("artifact"),
        build_receipt: id("build receipt"),
    }
}

fn request() -> NativeBuildRequest {
    let resolver = CargoResolverIdentity::from_bytes(
        ".",
        "Cargo.lock",
        LOCK,
        ".cargo/config.toml",
        CONFIGURATION,
        "/toolchain/vendor",
    );
    let mut environment = vec![("RUSTFLAGS".into(), "-Cpanic=abort".into())];
    environment.extend(resolver.environment_bindings());
    NativeBuildRequest {
        source_mount: "source/exact".into(),
        manifest: "Cargo.toml".into(),
        package: "guest".into(),
        features: BTreeSet::new(),
        target_triple: "x86_64-unknown-linux-gnu".into(),
        profile: sim_lib_hotload::NativeBuildProfile::Development,
        resolver,
        read_only_resources: std::collections::BTreeMap::new(),
        artifact: NativeArtifactTarget::DynamicLibrary {
            expected_library: Symbol::qualified("test", "guest"),
        },
        toolchain: sim_lib_hotload::ToolchainIdentity {
            content: "toolchain/exact".into(),
            cargo_program: "cargo".into(),
            environment,
        },
    }
}

#[test]
fn exact_owner_receipt_is_current_revocable_and_owner_bound() {
    let generation = id("generation/same-description");
    let first = ProjectionSourceReviewOwner::boot(generation.clone()).unwrap();
    let second = ProjectionSourceReviewOwner::boot(generation).unwrap();
    let first_authority = first.authority();
    let exact = snapshot();
    let invocation = source_review_invocation(&source_review_binding().unwrap(), &exact).unwrap();
    first.live.mark_current(&invocation).unwrap();
    let receipt = first_authority
        .live
        .issue(
            invocation.clone(),
            result_id(&exact).unwrap(),
            EvidenceGrade::Reproducible,
            provenance_id(&exact).unwrap(),
            support_id(&exact).unwrap(),
        )
        .unwrap();

    receipt.verify_current(&first_authority.live).unwrap();
    assert!(matches!(
        receipt.verify_current(&second.authority().live),
        Err(LiveCheckerError::ForeignAuthority)
    ));
    first.live.revoke(&invocation).unwrap();
    assert!(receipt.verify_current(&first_authority.live).is_err());
    drop(first);
    assert!(matches!(
        first_authority.ensure_live(),
        Err(ProjectionSourceReviewError::Authority(
            LiveCheckerError::Unavailable
        ))
    ));
}

#[test]
fn every_consumed_identity_and_custody_join_invalidates_the_invocation() {
    let exact = snapshot();
    let binding = source_review_binding().unwrap();
    let exact_invocation = source_review_invocation(&binding, &exact).unwrap();
    let mut source = exact.clone();
    source.inputs[2].content = id("changed source");
    let mut dependency = exact.clone();
    dependency.inputs.insert(
        3,
        input(
            ConsumedBuildInputOrigin::Toolchain,
            "registry/dependency/src/lib.rs",
            "changed dependency source",
            ReviewedInputRole::RustSource,
        ),
    );
    let mut toolchain = exact.clone();
    toolchain.inputs[3].content = id("changed compiler");
    for changed in [
        source,
        dependency,
        toolchain,
        ReviewedProjectionSource {
            consumed_inputs: id("changed owner ledger"),
            ..exact.clone()
        },
        ReviewedProjectionSource {
            request: id("changed features or target"),
            ..exact.clone()
        },
        ReviewedProjectionSource {
            artifact: id("substituted artifact"),
            ..exact.clone()
        },
        ReviewedProjectionSource {
            build_receipt: id("substituted build receipt"),
            ..exact.clone()
        },
    ] {
        assert_ne!(
            source_review_invocation(&binding, &changed).unwrap(),
            exact_invocation
        );
    }
}

#[test]
fn syntax_aware_forbidden_specimens_are_refused_by_class() {
    let cases = [
        (
            "pub fn run() { let _ = std::fs::read(\"/proc/self/status\"); }",
            ProjectionSourceForbiddenClass::AmbientIo,
        ),
        (
            "pub unsafe fn run() {}",
            ProjectionSourceForbiddenClass::UnsafeRust,
        ),
        (
            "pub extern \"C\" fn callback() {}",
            ProjectionSourceForbiddenClass::ForeignCode,
        ),
        (
            "static CACHE: u64 = 1;",
            ProjectionSourceForbiddenClass::GlobalState,
        ),
        (
            "pub fn run() { let _ = std::time::SystemTime::now(); }",
            ProjectionSourceForbiddenClass::Nondeterminism,
        ),
        (
            "pub fn run() { let _ = include_bytes!(\"secret.bin\"); }",
            ProjectionSourceForbiddenClass::AmbientIo,
        ),
        (
            "#[derive(Clone)] pub struct Projection(u64);",
            ProjectionSourceForbiddenClass::BuildTimeCode,
        ),
    ];
    for (source, expected) in cases {
        assert!(matches!(
            review_rust_source("src/lib.rs", source.as_bytes()),
            Err(ProjectionSourceReviewError::Forbidden { class, .. }) if class == expected
        ));
    }
}

#[test]
fn syntax_not_spelling_controls_policy_and_unknown_semantics_fail_closed() {
    review_rust_source(
        "src/lib.rs",
        br#"pub fn project(value: u64) -> u64 {
            let _description = "unsafe std::fs::read include_bytes!";
            value + 1
        }"#,
    )
    .unwrap();

    for source in [
        "pub fn project(value: u64) -> u64 { helper(value) }",
        "use core::option::Option; pub fn project(value: Option<u64>) -> u64 { 1 }",
        "pub fn project(values: &[u64]) -> u64 { values[0] }",
        "#[cfg(unix)] pub fn project(value: u64) -> u64 { value }",
    ] {
        assert!(matches!(
            review_rust_source("src/lib.rs", source.as_bytes()),
            Err(ProjectionSourceReviewError::SemanticAnalysisUnavailable { .. })
        ));
    }
}

#[test]
fn retained_inputs_have_distinct_roles_and_unknown_inputs_refuse() {
    let cases = [
        (
            ConsumedBuildInputOrigin::Source,
            vec!["Cargo.toml".into()],
            b"[package]\nname='guest'\nversion='0.1.0'\n".as_slice(),
            ReviewedInputRole::CargoManifest,
        ),
        (
            ConsumedBuildInputOrigin::Toolchain,
            vec![
                "registry".into(),
                "dep".into(),
                "src".into(),
                "lib.rs".into(),
            ],
            b"pub fn value(input: u64) -> u64 { input + 1 }".as_slice(),
            ReviewedInputRole::RustSource,
        ),
        (
            ConsumedBuildInputOrigin::Toolchain,
            vec!["bin".into(), "rustc".into()],
            b"ELF compiler bytes".as_slice(),
            ReviewedInputRole::ToolchainTrustRoot,
        ),
    ];
    for (origin, path, bytes, expected) in cases {
        assert_eq!(
            review_retained_input(origin, None, &path, &id("content"), bytes, false)
                .unwrap()
                .role,
            expected
        );
    }

    assert!(matches!(
        review_retained_input(
            ConsumedBuildInputOrigin::Source,
            None,
            &["secret.bin".into()],
            &id("secret"),
            b"ambient data",
            false,
        ),
        Err(ProjectionSourceReviewError::Forbidden {
            class: ProjectionSourceForbiddenClass::AmbientIo,
            ..
        })
    ));
    assert!(matches!(
        review_retained_input(
            ConsumedBuildInputOrigin::Workspace,
            None,
            &["generated.o".into()],
            &id("native"),
            b"native object",
            false,
        ),
        Err(ProjectionSourceReviewError::InputClassificationUnavailable { .. })
    ));
    assert!(matches!(
        review_retained_input(
            ConsumedBuildInputOrigin::Source,
            None,
            &["build.rs".into()],
            &id("build script"),
            b"fn main() {}",
            false,
        ),
        Err(ProjectionSourceReviewError::Forbidden {
            class: ProjectionSourceForbiddenClass::BuildTimeCode,
            ..
        })
    ));

    assert_eq!(
        review_retained_input(
            ConsumedBuildInputOrigin::Workspace,
            None,
            &["target".into(), "deps".into(), "guest.rlib".into()],
            &id("compiler output"),
            b"retained compiler output",
            true,
        )
        .unwrap()
        .role,
        ReviewedInputRole::CompilerUnitOutput
    );
    assert!(matches!(
        review_retained_input(
            ConsumedBuildInputOrigin::Source,
            None,
            &["src".into(), "lib.rs".into()],
            &id("misclassified source"),
            b"pub fn value() {}",
            true,
        ),
        Err(ProjectionSourceReviewError::InputClassificationUnavailable { .. })
    ));
}

#[test]
fn immutable_resource_owner_and_guest_root_are_checker_inputs() {
    let reviewed = review_retained_input(
        ConsumedBuildInputOrigin::ImmutableResource,
        Some(("/vendor", "resolver/vendor/exact")),
        &["crate".into(), "src".into(), "lib.rs".into()],
        &id("vendored source"),
        b"pub fn dependency() -> bool { true }",
        false,
    )
    .unwrap();
    assert_eq!(reviewed.role, ReviewedInputRole::RustSource);

    let mut exact = snapshot();
    exact.inputs.push(reviewed.clone());
    let exact_identity = consumed_inputs_datum(&exact).content_id().unwrap();

    let mut substituted = exact.clone();
    substituted.inputs.last_mut().unwrap().immutable_resource =
        Some(("/registry".into(), "resolver/vendor/substituted".into()));
    assert_ne!(
        consumed_inputs_datum(&substituted).content_id().unwrap(),
        exact_identity
    );

    assert!(matches!(
        review_retained_input(
            ConsumedBuildInputOrigin::ImmutableResource,
            None,
            &["crate".into(), "src".into(), "lib.rs".into()],
            &id("vendored source"),
            b"pub fn dependency() -> bool { true }",
            false,
        ),
        Err(ProjectionSourceReviewError::Substituted(
            "immutable consumed-input owner binding"
        ))
    ));
}

#[test]
fn missing_duplicate_and_typed_owner_records_fail_closed() {
    let exact = snapshot();
    let without_lock = exact
        .inputs
        .iter()
        .filter(|input| input.role != ReviewedInputRole::ResolverLock)
        .cloned()
        .collect::<Vec<_>>();
    assert!(matches!(
        ensure_complete_selected_inputs(&request(), &without_lock),
        Err(ProjectionSourceReviewError::ConsumedInputsPrerequisite {
            kind: ConsumedBuildInputsPrerequisiteKind::ResolverConfiguration,
            ..
        })
    ));
    let without_rust = exact
        .inputs
        .iter()
        .filter(|input| input.role != ReviewedInputRole::RustSource)
        .cloned()
        .collect::<Vec<_>>();
    assert!(matches!(
        ensure_complete_selected_inputs(&request(), &without_rust),
        Err(ProjectionSourceReviewError::ConsumedInputsPrerequisite {
            kind: ConsumedBuildInputsPrerequisiteKind::CompilerDependencyRecord,
            ..
        })
    ));
    let mut duplicate = exact.inputs.clone();
    duplicate.push(exact.inputs[0].clone());
    assert!(matches!(
        ensure_complete_selected_inputs(&request(), &duplicate),
        Err(ProjectionSourceReviewError::Substituted(
            "duplicate consumed input"
        ))
    ));

    for kind in [
        ConsumedBuildInputsPrerequisiteKind::BuildScriptReadTrace,
        ConsumedBuildInputsPrerequisiteKind::NativeInputTrace,
        ConsumedBuildInputsPrerequisiteKind::CargoRecord,
    ] {
        let error = ProjectionSourceReviewError::ConsumedInputsPrerequisite {
            kind,
            detail: "owner record is incomplete".into(),
        };
        assert!(error.to_string().contains(&format!("{kind:?}")));
    }
}
