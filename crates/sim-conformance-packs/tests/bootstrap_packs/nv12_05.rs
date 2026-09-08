use super::{MemorySubject, PackVerdict, packs, request};

#[test]
fn nv12_05_scopes_bind_recovery_local_effect_and_bootstrap_handoff() {
    let reconcile = [
        "operation.lease-is-bounded",
        "operation.lease-binds-holder-and-writer-fence",
        "operation.observe-before-resume",
        "operation.observer-independent-of-performer",
        "operation.already-true-reduced",
        "operation.verified-reduced",
        "operation.diverged-reduced",
        "operation.uncertain-reduced",
        "operation.idempotent-retry-requires-expiry-and-absence",
        "operation.exactly-once-unknown-ack-not-repeated",
        "operation.lost-receipt-reconciles-or-uncertain",
        "operation.observer-disagreement-uncertain",
        "operation.performer-identity-stable-on-recovery",
        "operation.all-crash-cuts-reconstruct",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    })
    .with("operation.crash-boundaries", "8");
    assert!(matches!(
        packs::operation::check(&request("checker/c-op", "operation/reconcile", &reconcile,)),
        PackVerdict::Pass { .. }
    ));

    let local = [
        "operation.local-port-is-portable",
        "operation.local-command-is-installed-and-allowlisted",
        "operation.local-command-id-binds-complete-spec",
        "operation.local-manifest-script-bytes-exact",
        "operation.local-model-interpolation-refused",
        "operation.local-environment-sealed",
        "operation.local-writable-roots-confined",
        "operation.local-network-absent",
        "operation.local-network-grant-separate",
        "operation.local-release-credentials-absent",
        "operation.local-formatter-mutation-observed",
        "operation.local-test-success-observed",
        "operation.local-test-failure-observed",
        "operation.local-validation-command-exact",
        "operation.local-docs-command-exact",
        "operation.local-timeout-terminates-group",
        "operation.local-cancellation-terminates-group",
        "operation.local-signal-escalation-bounded",
        "operation.local-descendants-zero",
        "operation.local-scratch-zero",
        "operation.local-independent-postcondition",
        "operation.local-later-owner-gates-reachable",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    });
    assert!(matches!(
        packs::operation::check(&request("checker/c-op", "operation/local", &local)),
        PackVerdict::Pass { .. }
    ));

    let boundary = [
        "boundary.kernel-unchanged",
        "boundary.behavior-remains-loaded",
        "boundary.local-port-owned-by-runtime",
        "boundary.local-adapter-owned-by-ubuntu-capsule",
        "boundary.packet-tooling-has-no-native-process",
        "boundary.command-spec-has-no-native-path",
        "boundary.native-paths-boot-resolved",
        "boundary.operation-gate-composed-on-journal",
        "boundary.process-port-reused",
        "boundary.sandbox-launcher-reused",
        "boundary.no-ambient-process",
        "boundary.no-ambient-network",
        "boundary.no-general-checkout-transaction",
        "boundary.operator-retains-integration-and-release",
        "boundary.local-route-and-specimen-indexed",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    });
    assert!(matches!(
        packs::boundary::check(&request(
            "checker/c-boundary",
            "boundary/local-adapter",
            &boundary,
        )),
        PackVerdict::Pass { .. }
    ));

    let work = [
        "work.effect-port-is-local-check-port",
        "work.packet-cannot-construct-native-command",
        "work.packet-names-installed-command-id",
        "work.command-bytes-cannot-be-substituted",
        "work.proposals-remain-pure",
        "work.proposals-apply-only-in-disposable-checkout",
        "work.actual-owner-integration-remains-operator",
        "work.formatter-result-observed",
        "work.test-pass-result-observed",
        "work.test-failure-result-observed",
        "work.validation-and-docs-results-observed",
        "work.operation-outcomes-typed",
        "work.actor-grant-handoff-recorded",
        "work.future-world-source-proof-unassumed",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    });
    assert!(matches!(
        packs::work::check(&request("checker/c-work", "work/packet-effects", &work)),
        PackVerdict::Pass { .. }
    ));

    let ownership = [
        "ownership.bootstrap-closure-complete",
        "ownership.bootstrap-transitive-dependencies-released",
        "ownership.bootstrap-produced-source-qualified",
        "ownership.bootstrap-produced-api-qualified",
        "ownership.bootstrap-produced-release-qualified",
        "ownership.bootstrap-binding-receipts-verified",
        "ownership.bootstrap-dependency-use-sets-qualified",
        "ownership.bootstrap-later-surfaces-planned",
        "ownership.bootstrap-roadmap-final-unclaimed",
        "ownership.bootstrap-handoff-boundary-recorded",
        "ownership.bootstrap-support-graph-acyclic",
        "ownership.bootstrap-overlaps-zero",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    })
    .with("ownership.bootstrap-reached-surfaces", "21")
    .with("ownership.bootstrap-qualified-surfaces", "21");
    assert!(matches!(
        packs::ownership::check(&request("checker/c-own", "ownership/bootstrap", &ownership,)),
        PackVerdict::Pass { .. }
    ));
    let claimed_final = ownership
        .clone()
        .with("ownership.bootstrap-roadmap-final-unclaimed", "false");
    assert!(matches!(
        packs::ownership::check(&request(
            "checker/c-own",
            "ownership/bootstrap",
            &claimed_final,
        )),
        PackVerdict::Refused(ref failure) if failure.code == "evidence-mismatch"
    ));

    let release = [
        "release.audit-passed",
        "release.authorship-passed",
        "release.boot-smoke-passed",
        "release.generated-converged",
        "release.mirrors-current",
        "release.owner-docs-passed",
        "release.owner-validation-passed",
        "release.packages-assembled",
        "release.pins-exact",
        "release.publication-confirmed",
        "release.standalone-ci-green",
        "release.tags-exact",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    });
    assert!(matches!(
        packs::release::check(&request("checker/c-release", "release/nv12-05", &release,)),
        PackVerdict::Pass { .. }
    ));
}
