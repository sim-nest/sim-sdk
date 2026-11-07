use super::*;
use sim_lib_exec::{SandboxCompilerSemanticReport, SandboxRefusal};
use sim_lib_hotload::{CargoResolverIdentity, NativeArtifactTarget, ToolchainIdentity};

fn id(label: &str) -> ContentId {
    Datum::String(label.into()).content_id().unwrap()
}

fn snapshot() -> ReviewedCompilerSemantics {
    ReviewedCompilerSemantics {
        selection: id("selection"),
        report_channel: id("report-channel"),
        report_set: id("report-set"),
        classification: id("classification"),
        entry_identity: id("entry-identity"),
        entry_definition: ENTRY_DEFINITION.into(),
        final_unit: id("final-unit"),
        artifact: id("artifact"),
        build_receipt: id("build-receipt"),
        input_ledger: id("input-ledger"),
        implementation: id("artifact"),
        complete_build_inputs: id("complete-build-inputs"),
        runtime_closure: id("runtime-closure"),
        platform_generation: id("platform-generation"),
        native_confinement: id("native-confinement"),
        selected_policy_set: id("selected-policy-set"),
        bound_policy: id("bound-policy"),
        selected_policies: id("selected-policies"),
        terminal_binding: id("terminal-binding"),
    }
}

const TARGET: &str = "x86_64-unknown-linux-gnu";
const ENTRY_DEFINITION: &str = "projection_entry";
const ENTRY_SYMBOL: &str = "_ZN6sample16projection_entry17h0123456789abcdefE";
const ANALYZER: [u8; 32] = [0x6a; 32];
const LOCK: &[u8] = b"version = 4\n";
const CONFIGURATION: &[u8] = b"[net]\noffline = true\n[source.crates-io]\nreplace-with = 'vendored-sources'\n[source.vendored-sources]\ndirectory = '/toolchain/vendor'\n";

struct FakeReport {
    value: Value,
    analyzer: [u8; 32],
    analyzer_pid: u32,
    analyzer_start: u64,
    cargo_pid: u32,
    cargo_start: u64,
}

impl SandboxCompilerSemanticReport for FakeReport {
    fn read(&self) -> Result<Vec<u8>, Box<SandboxRefusal>> {
        Ok(serde_json::to_vec(&self.value).unwrap())
    }

    fn analyzer_sha256(&self) -> [u8; 32] {
        self.analyzer
    }

    fn analyzer_pid(&self) -> u32 {
        self.analyzer_pid
    }

    fn analyzer_start_time_ticks(&self) -> u64 {
        self.analyzer_start
    }

    fn cargo_pid(&self) -> u32 {
        self.cargo_pid
    }

    fn cargo_start_time_ticks(&self) -> u64 {
        self.cargo_start
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
    let mut environment = vec![
        ("RUSTC_WRAPPER".into(), "/toolchain/bin/analyzer".into()),
        (
            "SIM_PROJECTION_TOOLCHAIN_ID".into(),
            "toolchain-sha256-test".into(),
        ),
        (
            "SIM_PROJECTION_SEMANTICS_SOCKET".into(),
            "/work/.sim-sr".into(),
        ),
    ];
    environment.extend(resolver.environment_bindings());
    NativeBuildRequest {
        source_mount: "source".into(),
        manifest: "Cargo.toml".into(),
        package: "sample".into(),
        target_triple: TARGET.into(),
        features: BTreeSet::new(),
        profile: sim_lib_hotload::NativeBuildProfile::Development,
        resolver,
        read_only_resources: std::collections::BTreeMap::new(),
        artifact: NativeArtifactTarget::Executable {
            binary: "sample".into(),
        },
        toolchain: ToolchainIdentity {
            content: "toolchain-sha256-test".into(),
            cargo_program: "/toolchain/bin/cargo".into(),
            environment,
        },
    }
}

fn selection() -> ProjectionCompilerSemanticSelection {
    ProjectionCompilerSemanticSelection::new(
        "/toolchain/bin/analyzer",
        ANALYZER,
        SCHEMA,
        [8; 32],
        0,
        ENTRY_DEFINITION,
        1024 * 1024,
        [ProjectionCompilerUnitRoute::new(
            0,
            "sample 0.1.0 (path+file:///source)",
            "sample",
            "sample",
            TARGET,
            ["bin".into()],
            ["bin".into()],
            Vec::<String>::new(),
            "/source/src/main.rs",
            "2024",
            "{}",
            false,
            false,
            false,
            std::iter::empty(),
            true,
        )
        .unwrap()],
    )
    .unwrap()
}

fn classification(count_overrides: &[(&str, u64)]) -> Value {
    let mut counts = POLICY
        .iter()
        .map(|(category, _)| ((*category).to_owned(), 0_u64))
        .collect::<BTreeMap<_, _>>();
    for (category, count) in count_overrides {
        *counts.get_mut(*category).unwrap() = *count;
    }
    let entries = POLICY
        .iter()
        .map(|(category, disposition)| {
            json!({
                "category": category,
                "disposition": match disposition {
                    Disposition::Allow => "allow",
                    Disposition::RefuseLocalSlice => "refuse-local-slice",
                },
            })
        })
        .collect::<Vec<_>>();
    let inspected = counts.values().copied().sum::<u64>();
    let refused = POLICY
        .iter()
        .filter(|(_, disposition)| *disposition == Disposition::RefuseLocalSlice)
        .map(|(category, _)| counts[*category])
        .sum::<u64>();
    let identity_input = json!({
        "taxonomy": TAXONOMY,
        "policy": CLASSIFICATION_POLICY,
        "policy_entries": entries,
        "counts": counts,
        "inspected_operations": inspected,
        "classified_operations": inspected,
        "refused_operations": refused,
    });
    let identity = hex(&Sha256::digest(
        serde_json::to_vec(&identity_input).unwrap(),
    ));
    let mut output = identity_input;
    output
        .as_object_mut()
        .unwrap()
        .insert("identity_sha256".into(), Value::String(identity));
    output
}

fn report_value() -> Value {
    json!({
        "schema": SCHEMA,
        "analyzer": {"executable_sha256": hex(&ANALYZER), "schema": SCHEMA},
        "invocation": {
            "arguments": [
                "/toolchain/bin/rustc", "--crate-name", "sample", "--target", TARGET,
                "/source/src/main.rs"
            ],
            "environment": {
                "RUSTC_WRAPPER": "/toolchain/bin/analyzer",
                "SIM_PROJECTION_TOOLCHAIN_ID": "toolchain-sha256-test"
            },
            "target": TARGET,
            "crate_types": ["Bin"],
            "toolchain_owner": "toolchain-sha256-test"
        },
        "unit": {"crate_name": "sample", "stable_crate_id": "0123456789abcdef", "crate_hash": "fedcba9876543210"},
        "entry_definition": ENTRY_DEFINITION,
        "entry_matches": [{"definition": ENTRY_DEFINITION, "symbol": ENTRY_SYMBOL}],
        "instances": [{"symbol": ENTRY_SYMBOL, "definition": ENTRY_DEFINITION, "kind": "item", "local": true}],
        "edges": [],
        "hazards": [],
        "statics": [],
        "global_assembly": [],
        "operation_classification": classification(&[
            ("mono-item.function", 1),
            ("instance.mir-body", 1),
            ("instance-kind.item", 1),
            ("terminator.return", 1),
        ]),
        "completeness": {
            "local_slice_resolved": true,
            "admission_complete": false,
            "unresolved": [],
            "admission_prerequisites": [
                "owner-held-report-channel",
                "complete-external-summary-closure",
                "build-script-and-proc-macro-linkage",
                "final-native-link-participants",
                "artifact-build-receipt-and-input-ledger-owner-binding"
            ],
            "instance_limit": 16_384,
            "edge_limit": 131_072
        }
    })
}

fn report(value: Value) -> FakeReport {
    FakeReport {
        value,
        analyzer: ANALYZER,
        analyzer_pid: 101,
        analyzer_start: 201,
        cargo_pid: 301,
        cargo_start: 401,
    }
}

#[test]
fn closed_policy_is_unique_and_identity_bearing() {
    let categories = POLICY.iter().map(|entry| entry.0).collect::<BTreeSet<_>>();
    assert_eq!(categories.len(), POLICY.len());
    assert!(
        POLICY
            .iter()
            .any(|(_, disposition)| *disposition == Disposition::RefuseLocalSlice)
    );
    assert_ne!(policy_id().unwrap().content_id(), &id("forged-policy"));
}

#[test]
fn checker_invocation_binds_every_owner_join_identity() {
    let binding = checker_binding().unwrap();
    let exact = snapshot();
    let expected = invocation(&binding, &exact).unwrap();
    let substitutions = [
        ReviewedCompilerSemantics {
            report_set: id("other-report-set"),
            ..exact.clone()
        },
        ReviewedCompilerSemantics {
            classification: id("other-classification"),
            ..exact.clone()
        },
        ReviewedCompilerSemantics {
            entry_identity: id("other-entry-identity"),
            ..exact.clone()
        },
        ReviewedCompilerSemantics {
            final_unit: id("other-final-unit"),
            ..exact.clone()
        },
        ReviewedCompilerSemantics {
            implementation: id("other-implementation"),
            ..exact.clone()
        },
        ReviewedCompilerSemantics {
            complete_build_inputs: id("other-inputs"),
            ..exact.clone()
        },
        ReviewedCompilerSemantics {
            runtime_closure: id("other-runtime"),
            ..exact
        },
    ];
    for substituted in substitutions {
        assert_ne!(invocation(&binding, &substituted).unwrap(), expected);
    }
}

#[test]
fn owner_receipt_is_generation_bound_revocable_and_dies_with_owner() {
    let owner = ProjectionCompilerReviewOwner::boot(id("generation")).unwrap();
    let authority = owner.authority();
    let snapshot = snapshot();
    let invocation = invocation(&checker_binding().unwrap(), &snapshot).unwrap();
    owner.live.mark_current(&invocation).unwrap();
    let receipt = authority
        .live
        .issue(
            invocation.clone(),
            result_id(&snapshot).unwrap(),
            EvidenceGrade::Reproducible,
            provenance_id(&snapshot).unwrap(),
            support_id(&snapshot).unwrap(),
        )
        .unwrap();
    receipt.verify_current(&authority.live).unwrap();
    owner.live.revoke(&invocation).unwrap();
    assert!(receipt.verify_current(&authority.live).is_err());
    drop(owner);
    assert!(authority.ensure_live().is_err());
}

#[test]
fn exact_v3_report_recomputes_closed_classification_and_definition_route() {
    let selection = selection();
    let parsed = parse_report(&report(report_value()), &selection, &request()).unwrap();
    let routes = match_routes(&selection, std::slice::from_ref(&parsed)).unwrap();
    validate_process_set(std::slice::from_ref(&parsed)).unwrap();
    let entry =
        validate_reachable_closure(ENTRY_DEFINITION, routes[&0], std::slice::from_ref(&parsed))
            .unwrap();
    entry_identity_id(selection.entry_unit(), &parsed, &entry).unwrap();
}

#[test]
fn compiler_entry_identity_binds_owner_unit_crate_and_resolved_symbol() {
    let selection = selection();
    let parsed = parse_report(&report(report_value()), &selection, &request()).unwrap();
    let entry = parsed.entry_matches[0].clone();
    let exact = entry_identity_id(selection.entry_unit(), &parsed, &entry).unwrap();

    let mut substituted_crate = report_value();
    substituted_crate["unit"]["crate_hash"] = Value::String("substituted-crate-hash".into());
    let substituted_crate =
        parse_report(&report(substituted_crate), &selection, &request()).unwrap();
    assert_ne!(
        entry_identity_id(
            selection.entry_unit(),
            &substituted_crate,
            &substituted_crate.entry_matches[0]
        )
        .unwrap(),
        exact
    );

    let mut substituted_symbol = parsed.clone();
    substituted_symbol.entry_matches[0].symbol = "_ZN11substitutedE".into();
    assert_ne!(
        entry_identity_id(
            selection.entry_unit(),
            &substituted_symbol,
            &substituted_symbol.entry_matches[0]
        )
        .unwrap(),
        exact
    );
}

#[test]
fn entry_match_must_come_from_the_exact_selected_owner_unit() {
    let root = selection().entry_unit().clone();
    let dependency = ProjectionCompilerUnitRoute::new(
        1,
        "dependency 0.1.0 (path+file:///source/dependency)",
        "dependency",
        "dependency",
        TARGET,
        ["lib".into()],
        ["rlib".into()],
        Vec::<String>::new(),
        "/source/dependency/src/lib.rs",
        "2024",
        "{}",
        false,
        false,
        false,
        std::iter::empty(),
        false,
    )
    .unwrap();
    let selected_root = ProjectionCompilerSemanticSelection::new(
        "/toolchain/bin/analyzer",
        ANALYZER,
        SCHEMA,
        [8; 32],
        0,
        ENTRY_DEFINITION,
        1024 * 1024,
        [root.clone(), dependency.clone()],
    )
    .unwrap();
    let mut root_report = parse_report(&report(report_value()), &selection(), &request()).unwrap();
    root_report.entry_matches.clear();
    root_report.instances.clear();
    let mut dependency_report =
        parse_report(&report(report_value()), &selection(), &request()).unwrap();
    dependency_report.crate_name = "dependency".into();
    dependency_report.source = "/source/dependency/src/lib.rs".into();
    dependency_report.crate_types = BTreeSet::from(["rlib".into()]);
    dependency_report.stable_crate_id = "1111111111111111".into();
    dependency_report.crate_hash = "dependency-crate-hash".into();
    let reports = [root_report, dependency_report];
    let routes = match_routes(&selected_root, &reports).unwrap();
    assert!(
        validate_reachable_closure(ENTRY_DEFINITION, routes[&0], &reports).is_err(),
        "a dependency-unit match must not satisfy a root-unit selection"
    );

    let selected_dependency = ProjectionCompilerSemanticSelection::new(
        "/toolchain/bin/analyzer",
        ANALYZER,
        SCHEMA,
        [8; 32],
        1,
        ENTRY_DEFINITION,
        1024 * 1024,
        [root, dependency],
    )
    .unwrap();
    let routes = match_routes(&selected_dependency, &reports).unwrap();
    assert!(
        validate_reachable_closure(ENTRY_DEFINITION, routes[&1], &reports).is_ok(),
        "the same report set satisfies the exact dependency-unit selection"
    );
}

#[test]
fn v1_and_local_completion_claims_do_not_qualify() {
    let selection = selection();
    let request = request();
    let mut v1 = report_value();
    v1["schema"] = Value::String("sim-rustc-projection-semantics-v1".into());
    assert!(parse_report(&report(v1), &selection, &request).is_err());

    let mut complete = report_value();
    complete["completeness"]["admission_complete"] = Value::Bool(true);
    assert!(parse_report(&report(complete), &selection, &request).is_err());
}

#[test]
fn taxonomy_count_identity_and_unresolved_substitutions_are_refused() {
    let selection = selection();
    let request = request();
    for value in [
        report_value(),
        report_value(),
        report_value(),
        report_value(),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, mut value)| {
        match index {
            0 => value["operation_classification"]["taxonomy"] = Value::String("forged".into()),
            1 => value["operation_classification"]["counts"]["terminator.return"] = json!(2),
            2 => {
                value["operation_classification"]["identity_sha256"] = Value::String(hex(&[9; 32]))
            }
            3 => value["completeness"]["unresolved"] = json!(["external summary missing"]),
            _ => unreachable!(),
        }
        value
    }) {
        assert!(parse_report(&report(value), &selection, &request).is_err());
    }
}

#[test]
fn analyzer_route_environment_and_process_substitutions_are_refused() {
    let selection = selection();
    let request = request();
    let mut wrong_analyzer = report(report_value());
    wrong_analyzer.analyzer = [0x7b; 32];
    assert!(parse_report(&wrong_analyzer, &selection, &request).is_err());

    let mut wrong_environment = report_value();
    wrong_environment["invocation"]["environment"]["RUSTFLAGS"] =
        Value::String("-Cpanic=abort".into());
    assert!(parse_report(&report(wrong_environment), &selection, &request).is_err());

    let mut wrong_route = report_value();
    wrong_route["invocation"]["target"] = Value::String("aarch64-unknown-linux-gnu".into());
    let parsed = parse_report(&report(wrong_route), &selection, &request).unwrap();
    assert!(match_routes(&selection, &[parsed]).is_err());

    let mut missing_process = report(report_value());
    missing_process.analyzer_start = 0;
    assert!(parse_report(&missing_process, &selection, &request).is_err());
}

#[test]
fn missing_external_closure_and_replayed_report_incarnation_are_refused() {
    let mut parsed = parse_report(&report(report_value()), &selection(), &request()).unwrap();
    parsed.edges.push(ParsedEdge {
        from: ENTRY_SYMBOL.into(),
        to: "absent_external".into(),
        kind: "resolved-direct-call".into(),
    });
    assert!(validate_reachable_closure(ENTRY_DEFINITION, 0, &[parsed]).is_err());

    let first = parse_report(&report(report_value()), &selection(), &request()).unwrap();
    let replay = parse_report(&report(report_value()), &selection(), &request()).unwrap();
    assert!(validate_process_set(&[first, replay]).is_err());
}
