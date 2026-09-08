use super::{MemorySubject, PackVerdict, packs, request};

fn true_subject(keys: &[&str]) -> MemorySubject {
    keys.iter().fold(MemorySubject::default(), |subject, key| {
        subject.with(*key, "true")
    })
}

#[test]
fn nv12_06_projection_scopes_are_funded_and_fail_closed() {
    let boundary = true_subject(&[
        "boundary.kernel-unchanged",
        "boundary.projection-registry-open",
        "boundary.config-shape-checked",
        "boundary.inputs-immutable-and-selected",
        "boundary.mediated-reads-exact",
        "boundary.semantic-envelope-separated",
        "boundary.qualification-separate-from-confinement",
        "boundary.native-source-and-dependencies-reviewed",
        "boundary.native-loaded-code-identity-exact",
        "boundary.untrusted-native-refused",
        "boundary.native-proc-read-refused",
        "boundary.unprojected-mount-read-refused",
        "boundary.wasm-import-manifest-complete",
        "boundary.wasm-imports-exact",
        "boundary.wasm-clock-random-wasi-refused",
        "boundary.wasm-runtime-semantics-qualified",
        "boundary.wasm-instance-fresh",
        "boundary.wasm-budgets-bound",
        "boundary.bwrap-confinement-not-purity",
        "boundary.absent-membrane-typed-unavailable",
        "boundary.baseline-provider-kinds-complete",
        "boundary.federated-owner-closure-sealed",
    ]);
    assert!(matches!(
        packs::boundary::check(&request(
            "checker/c-boundary",
            "boundary/projection-admission",
            &boundary,
        )),
        PackVerdict::Pass { .. }
    ));
    assert!(matches!(
        packs::boundary::check(&request(
            "checker/c-boundary",
            "boundary/projection-admission",
            &boundary
                .clone()
                .with("boundary.untrusted-native-refused", "false"),
        )),
        PackVerdict::Refused(_)
    ));

    let disclosure = true_subject(&[
        "disclosure.audit-rules-exact",
        "disclosure.policy-value-canonical",
        "disclosure.provider-kind-open",
        "disclosure.policy-is-supplied-observation",
        "disclosure.nonconsumer-excludes-policy-digest",
        "disclosure.consumer-binds-policy-digest",
        "disclosure.allowlist-change-invalidates-exact-consumers",
        "disclosure.unrelated-conclusions-reused",
        "disclosure.disagreement-remains-unknown",
        "disclosure.hostile-public-sink-refused",
    ]);
    assert!(matches!(
        packs::disclosure::check(&request(
            "checker/c-disclose",
            "disclosure/projection",
            &disclosure,
        )),
        PackVerdict::Pass { .. }
    ));

    let evidence = true_subject(&[
        "evidence.projection-domain-tag-unique",
        "evidence.provider-package-version-bound",
        "evidence.provider-code-identity-bound",
        "evidence.projector-policy-bound",
        "evidence.checked-config-bound",
        "evidence.consumed-semantic-inputs-bound",
        "evidence.diagnostic-envelope-excluded",
        "evidence.equal-input-cross-host-stable",
        "evidence.semantic-delta-changes-digest",
        "evidence.affected-closure-independently-expected",
        "evidence.explanation-path-exact",
        "evidence.qualification-revocation-scoped",
    ]);
    assert!(matches!(
        packs::evidence::check(&request(
            "checker/c-evidence",
            "evidence/projection",
            &evidence,
        )),
        PackVerdict::Pass { .. }
    ));
}

#[test]
fn nv12_06_product_latency_and_produced_ownership_are_funded() {
    let world = true_subject(&[
        "product.world-loads-through-sim-run",
        "product.world-verb-not-kernel-or-frame",
        "product.world-project-stable-value",
        "product.world-diff-stable-value",
        "product.world-why-stable-value",
        "product.world-read-only",
        "product.world-no-observation-port",
        "product.world-no-proof-execution-port",
        "product.world-no-mutation-capability",
        "product.world-index-route",
        "product.world-recipe-checked",
        "product.world-brochure-present",
        "product.world-boot-smoke-passed",
    ]);
    assert!(matches!(
        packs::product::check(&request("checker/c-product", "product/world", &world)),
        PackVerdict::Pass { .. }
    ));
    let latency = true_subject(&[
        "product.query-warmups-20",
        "product.query-samples-200",
        "product.query-p95-at-most-200ms",
        "product.query-effect-calls-zero",
    ])
    .with("product.query-command", "world/why");
    assert!(matches!(
        packs::product::check(&request(
            "checker/c-product",
            "product/query-latency",
            &latency,
        )),
        PackVerdict::Pass { .. }
    ));

    let produced = true_subject(&[
        "ownership.target-was-planned-before-construction",
        "ownership.packet-dependencies-released",
        "ownership.unimplemented-external-dependency-refused",
        "ownership.target-source-qualified",
        "ownership.target-api-qualified",
        "ownership.target-release-qualified",
        "ownership.owner-validation-and-docs-exact",
        "ownership.index-route-present",
        "ownership.crate-admission-complete",
        "ownership.support-graph-acyclic",
        "ownership.overlaps-zero",
        "ownership.kernel-head-unchanged",
    ])
    .with("ownership.produced-target-surfaces", "3")
    .with("ownership.qualified-target-surfaces", "3");
    assert!(matches!(
        packs::ownership::check(&request("checker/c-own", "ownership/produced", &produced)),
        PackVerdict::Pass { .. }
    ));
}

#[test]
fn nv12_06_release_scope_requires_exact_closure() {
    let release = true_subject(&[
        "release.owner-validation-passed",
        "release.owner-docs-passed",
        "release.packages-assembled",
        "release.audit-passed",
        "release.authorship-passed",
        "release.tags-exact",
        "release.pins-exact",
        "release.mirrors-current",
        "release.generated-converged",
        "release.boot-smoke-passed",
        "release.publication-confirmed",
        "release.standalone-ci-green",
    ]);
    assert!(matches!(
        packs::release::check(&request("checker/c-release", "release/nv12-06", &release,)),
        PackVerdict::Pass { .. }
    ));
    assert!(matches!(
        packs::release::check(&request(
            "checker/c-release",
            "release/nv12-06",
            &release.with("release.pins-exact", "false"),
        )),
        PackVerdict::Refused(_)
    ));
}
