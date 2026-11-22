// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Executed observations for the portable product and release scopes.

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SEAL: &str =
    "core/sha256-datum-v1:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn request<'a>(checker: &'a str, scope: &'a str, evidence: &'a MemorySubject) -> PackRequest<'a> {
    let binding = find_pack(checker).unwrap().checker_binding().unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    PackRequest {
        checker,
        binding,
        subject: SUBJECT,
        scope,
        evidence,
    }
}

fn entry(checker: &str) -> fn(&PackRequest<'_>) -> PackVerdict {
    match checker {
        "checker/c-deliver" => packs::delivery::check,
        "checker/c-author" => packs::authoring::check,
        "checker/c-product" => packs::product::check,
        "checker/c-release" => packs::release::check,
        "checker/c-id" => packs::identity::check,
        "checker/c-boundary" => packs::boundary::check,
        "checker/c-evidence" => packs::evidence::check,
        "checker/c-op" => packs::operation::check,
        "checker/c-journal" => packs::journal::check,
        "checker/c-work" => packs::work::check,
        "checker/c-facet" => packs::facet::check,
        "checker/c-disclose" => packs::disclosure::check,
        "checker/c-drive" => packs::drive::check,
        "checker/c-v3" => packs::retirement::check,
        other => panic!("no pack entry for {other}"),
    }
}

fn execute(checker: &str, scope: &str, evidence: &MemorySubject) -> PackVerdict {
    entry(checker)(&request(checker, scope, evidence))
}

fn truths(keys: impl IntoIterator<Item = impl AsRef<str>>) -> MemorySubject {
    keys.into_iter()
        .fold(MemorySubject::default(), |subject, key| {
            subject.with(key.as_ref(), "true")
        })
}

fn refusal(verdict: &PackVerdict) -> (String, String) {
    match verdict {
        PackVerdict::Refused(failure) => (failure.code.to_owned(), failure.detail.clone()),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn assert_missing(checker: &str, scope: &str, evidence: &MemorySubject, key: &str) {
    let (code, detail) = refusal(&execute(checker, scope, evidence));
    assert_eq!(code, "missing-evidence", "{checker} {scope}: {detail}");
    assert_eq!(detail, key, "{checker} {scope}");
}

fn assert_unimplemented(checker: &str, scope: &str) {
    match execute(checker, scope, &MemorySubject::default()) {
        PackVerdict::UnimplementedPack { .. } => {}
        other => panic!("{scope} must stay unimplemented, got {other:?}"),
    }
}

fn pass_keys(verdict: &PackVerdict) -> Vec<&str> {
    match verdict {
        PackVerdict::Pass { observations, .. } => {
            observations.iter().map(|item| item.key.as_str()).collect()
        }
        other => panic!("expected a pass, got {other:?}"),
    }
}

fn assert_pass(checker: &str, scope: &str, evidence: &MemorySubject, keys: &[&str]) -> PackVerdict {
    let verdict = execute(checker, scope, evidence);
    let mut expected = keys.to_vec();
    expected.sort_unstable();
    expected.dedup();
    assert_eq!(pass_keys(&verdict), expected, "{checker} {scope}");
    verdict
}

#[test]
fn funded_delivery_and_authoring_scopes_execute_their_scenarios() {
    for scope in [
        "delivery/observation",
        "delivery/plan",
        "delivery/adapters",
        "delivery/real-canary",
        "delivery/composed",
    ] {
        let (code, detail) = refusal(&execute(
            "checker/c-deliver",
            scope,
            &MemorySubject::default(),
        ));
        assert_eq!(code, "missing-source", "{scope} {detail}");
        assert_eq!(detail, "delivery.command", "{scope}");
        let truths = truths(packs::delivery::SCENARIOS);
        let (code, detail) = refusal(&execute("checker/c-deliver", scope, &truths));
        assert_eq!(code, "unexpected-fact", "{scope} {detail}");
        assert_eq!(detail, "delivery.already-true", "{scope}");
    }

    for scope in ["authoring/family", "authoring/composed"] {
        let (code, detail) = refusal(&execute(
            "checker/c-author",
            scope,
            &MemorySubject::default(),
        ));
        assert_eq!(code, "missing-source", "{scope} {detail}");
        assert_eq!(detail, "authoring.command", "{scope}");
        let authoring = truths(packs::authoring::SCENARIOS);
        let (code, detail) = refusal(&execute("checker/c-author", scope, &authoring));
        assert_eq!(code, "unexpected-fact", "{scope} {detail}");
        assert_eq!(
            detail, "authoring.consultation-not-a-requirement",
            "{scope}"
        );
    }
}

#[test]
fn portable_product_scopes_do_not_accept_caller_supplied_truths() {
    let world = truths([
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
    let (code, _) = refusal(&execute("checker/c-product", "product/world", &world));
    assert_eq!(code, "unexpected-fact");
    let portable = truths([
        "product.loads-through-sim-run",
        "product.stable-value",
        "product.index-route",
        "product.recipe-checked",
        "product.brochure-present",
        "product.boot-smoke-passed",
        "product.verb-not-baked",
    ]);
    for scope in [
        "product/prove-plan",
        "product/prove",
        "product/deliver",
        "product/roadmap",
        "product/landing-queries",
        "product/convergence-queries",
        "product/installed",
        "product/composed",
    ] {
        let (code, detail) = refusal(&execute("checker/c-product", scope, &portable));
        assert_eq!(code, "unexpected-fact", "{scope} {detail}");
    }
}

#[test]
fn release_phases_15_through_26_require_the_closure_facts() {
    let closure = truths(packs::release::CLOSURE_FACTS);
    for phase in 15..=26 {
        let scope = format!("release/nv12-{phase:02}");
        assert_missing(
            "checker/c-release",
            &scope,
            &MemorySubject::default(),
            "release.owner-validation-passed",
        );
        let verdict = assert_pass(
            "checker/c-release",
            &scope,
            &closure,
            &packs::release::CLOSURE_FACTS,
        );
        let keys = pass_keys(&verdict);
        assert!(keys.iter().all(|key| key.starts_with("release.")));
        assert!(!keys.iter().any(|key| key.starts_with("journal.")));
    }
    let (code, detail) = refusal(&execute(
        "checker/c-release",
        "release/nv12-17",
        &closure.with("release.publication-confirmed", "false"),
    ));
    assert_eq!(code, "evidence-mismatch", "{detail}");
    let (code, detail) = refusal(&execute(
        "checker/c-release",
        "completion/replay",
        &MemorySubject::default(),
    ));
    assert_eq!(code, "missing-journal", "{detail}");
}

#[test]
fn final_seal_requires_a_content_identity_and_refuses_an_open_gap() {
    assert_missing(
        "checker/c-id",
        "identity/final-seal",
        &MemorySubject::default(),
        "identity.final-seal",
    );
    for planted in ["true", "PortableClosed"] {
        let (code, detail) = refusal(&execute(
            "checker/c-id",
            "identity/final-seal",
            &MemorySubject::default().with("identity.final-seal", planted),
        ));
        assert_eq!(code, "malformed-content-identity", "{planted} {detail}");
    }
    let partial = MemorySubject::default().with("identity.final-seal", SEAL);
    assert_missing(
        "checker/c-id",
        "identity/final-seal",
        &partial,
        "identity.final-seal-binds-claims",
    );
    let mut keys = vec!["identity.final-seal"];
    keys.extend(packs::identity::FINAL_SEAL_FACTS);
    assert_pass(
        "checker/c-id",
        "identity/final-seal",
        &truths(packs::identity::FINAL_SEAL_FACTS).with("identity.final-seal", SEAL),
        &keys,
    );
    let (code, _) = refusal(&execute(
        "checker/c-id",
        "identity/final-seal",
        &truths(packs::identity::FINAL_SEAL_FACTS)
            .with("identity.final-seal", SEAL)
            .with("identity.open-gap-refuses-seal", "false"),
    ));
    assert_eq!(code, "evidence-mismatch");
}

#[test]
fn composed_scopes_run_their_leaf_checks() {
    let boundary = truths(packs::boundary::COMPOSED_FACTS);
    assert_missing(
        "checker/c-boundary",
        "boundary/composed",
        &MemorySubject::default().with("boundary.kernel-unchanged", "true"),
        "boundary.composed-inventory-complete",
    );
    assert_pass(
        "checker/c-boundary",
        "boundary/composed",
        &boundary,
        &packs::boundary::COMPOSED_FACTS,
    );

    let mut evidence_keys = packs::evidence::PROJECTION_FACTS.to_vec();
    evidence_keys.extend(packs::evidence::PREDICTED_CLOSURE_FACTS);
    evidence_keys.push("evidence.composed-partial-refused");
    assert_missing(
        "checker/c-evidence",
        "evidence/composed",
        &MemorySubject::default().with("evidence.composed-partial-refused", "true"),
        "evidence.projection-domain-tag-unique",
    );
    assert_pass(
        "checker/c-evidence",
        "evidence/composed",
        &truths(evidence_keys.iter().copied()),
        &evidence_keys,
    );

    let mut disclosure = DISCLOSURE_FACTS.to_vec();
    disclosure.push("disclosure.composed-partial-refused");
    assert_missing(
        "checker/c-disclose",
        "disclosure/composed",
        &MemorySubject::default().with("disclosure.composed-partial-refused", "true"),
        "disclosure.audit-rules-exact",
    );
    assert_pass(
        "checker/c-disclose",
        "disclosure/composed",
        &truths(disclosure.iter().copied()),
        &disclosure,
    );

    let mut operation_keys = OPERATION_TRUE.to_vec();
    operation_keys.extend([
        "operation.crash-cuts",
        "operation.durable-states",
        "operation.crash-boundaries",
    ]);
    let operation = truths(OPERATION_TRUE)
        .with("operation.crash-cuts", "6")
        .with("operation.durable-states", "3")
        .with("operation.crash-boundaries", "8");
    assert_missing(
        "checker/c-op",
        "operation/composed",
        &MemorySubject::default().with("operation.composed-partial-refused", "true"),
        "operation.intent-is-canonical-datum",
    );
    assert_pass(
        "checker/c-op",
        "operation/composed",
        &operation,
        &operation_keys,
    );
    let (code, detail) = refusal(&execute(
        "checker/c-op",
        "operation/composed",
        &operation.with("operation.crash-boundaries", "7"),
    ));
    assert_eq!(code, "evidence-mismatch", "{detail}");

    let mut work_keys = WORK_EFFECTS.to_vec();
    work_keys.extend([
        "work.composed-partial-refused",
        "work.packet-pure-scenarios",
    ]);
    assert_missing(
        "checker/c-work",
        "work/composed",
        &MemorySubject::default().with("work.composed-partial-refused", "true"),
        "work.effect-port-is-local-check-port",
    );
    assert_pass(
        "checker/c-work",
        "work/composed",
        &truths(WORK_EFFECTS).with("work.composed-partial-refused", "true"),
        &work_keys,
    );

    assert_missing(
        "checker/c-facet",
        "facet/composed",
        &MemorySubject::default(),
        "facet.composed-partial-refused",
    );
    let facet = assert_pass(
        "checker/c-facet",
        "facet/composed",
        &MemorySubject::default().with("facet.composed-partial-refused", "true"),
        &["facet.composed-partial-refused", "facet.pure-scenarios"],
    );
    match facet {
        PackVerdict::Pass { observations, .. } => {
            let value = observations
                .iter()
                .find(|item| item.key == "facet.pure-scenarios")
                .expect("law observation")
                .value
                .as_str();
            assert_eq!(
                value,
                "already-true,unchanged,apply,disjoint,overlap,owner,region"
            );
        }
        other => panic!("facet composed did not pass: {other:?}"),
    }

    let ceiling = packs::journal::COLD_REPLAY_100K_CEILING_NS.to_string();
    let journal_numbers = MemorySubject::default()
        .with("journal.performance.records", "100000")
        .with("journal.performance.samples", "5")
        .with("journal.performance.maximum-ns", ceiling.clone())
        .with("journal.performance.ceiling-ns", ceiling);
    let journal = truths(JOURNAL_TRUE)
        .with("journal.performance.records", "100000")
        .with("journal.performance.samples", "5")
        .with(
            "journal.performance.maximum-ns",
            packs::journal::COLD_REPLAY_100K_CEILING_NS.to_string(),
        )
        .with(
            "journal.performance.ceiling-ns",
            packs::journal::COLD_REPLAY_100K_CEILING_NS.to_string(),
        );
    assert_missing(
        "checker/c-journal",
        "journal/composed",
        &journal_numbers,
        "journal.old-only-replay",
    );
    let (code, detail) = refusal(&execute(
        "checker/c-journal",
        "journal/composed",
        &truths(JOURNAL_TRUE)
            .with("journal.performance.records", "1")
            .with("journal.performance.samples", "5")
            .with("journal.performance.maximum-ns", "1")
            .with("journal.performance.ceiling-ns", "1"),
    ));
    assert_eq!(code, "performance-sample-contract", "{detail}");
    let mut journal_keys = JOURNAL_TRUE.to_vec();
    journal_keys.extend([
        "journal.performance.records",
        "journal.performance.samples",
        "journal.performance.maximum-ns",
        "journal.performance.ceiling-ns",
    ]);
    assert_pass(
        "checker/c-journal",
        "journal/composed",
        &journal,
        &journal_keys,
    );
}

#[test]
fn facet_transaction_runs_the_published_law() {
    let verdict = execute(
        "checker/c-facet",
        "facet/transaction",
        &MemorySubject::default(),
    );
    let PackVerdict::Pass { observations, .. } = verdict else {
        panic!("facet transaction did not pass: {verdict:?}");
    };
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].key, "facet.transaction");
    let value = observations[0].value.as_str();
    assert!(!value.contains("completion/claim-"));
    for token in [
        "apply-preimage",
        "apply-postimage",
        "apply-second",
        "wrong-head",
        "dirt",
        "private-flow",
        "generated-owner-bypass",
        "stale-base",
        "region-escape",
        "invalid-region",
        "overlap",
        "recover-applied",
        "recover-held",
        "rollback-restored",
        "rollback-kept",
    ] {
        assert!(value.contains(token), "{token} missing from {value}");
    }
    let (code, _) = refusal(&execute(
        "checker/c-facet",
        "facet/transaction",
        &MemorySubject::default().with("facet.transaction", "true"),
    ));
    assert_eq!(code, "unexpected-fact");
}

#[test]
fn scopes_left_out_of_this_change_stay_unimplemented() {
    for (checker, scope) in [
        ("checker/c-drive", "drive/native"),
        ("checker/c-facet", "facet/landing"),
        ("checker/c-facet", "facet/generated"),
        ("checker/c-work", "work/native-dispatch"),
        ("checker/c-work", "work/convergence"),
        ("checker/c-boundary", "boundary/observation"),
        ("checker/c-boundary", "boundary/source-closure"),
        ("checker/c-id", "identity/fixed-point"),
        ("checker/c-id", "identity/world-seal"),
        ("checker/c-id", "identity/evidence"),
        ("checker/c-evidence", "evidence/identity-grade"),
        ("checker/c-evidence", "evidence/cold-oracle"),
        ("checker/c-evidence", "evidence/convergence"),
        ("checker/c-op", "operation/delivery-adapters"),
        ("checker/c-disclose", "disclosure/transaction"),
        ("checker/c-disclose", "disclosure/landing"),
        ("checker/c-v3", "retirement/final"),
    ] {
        assert_unimplemented(checker, scope);
    }
}

const DISCLOSURE_FACTS: [&str; 10] = [
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
];

const OPERATION_TRUE: [&str; 54] = [
    "operation.intent-is-canonical-datum",
    "operation.operation-id-binds-target",
    "operation.operation-id-binds-intended-result",
    "operation.operation-id-binds-replay-policy",
    "operation.grant-recorded-separately",
    "operation.attempt-recorded-separately",
    "operation.lease-excluded-from-operation-id",
    "operation.contradictory-intent-refused",
    "operation.intent-before-dispatch",
    "operation.dispatch-before-performance",
    "operation.raw-receipt-after-performance",
    "operation.all-crash-cuts-reconstruct",
    "operation.recorded-dispatch-not-repeated",
    "operation.external-counter-survives-replay",
    "operation.missing-acknowledgement-not-failure",
    "operation.fake-performers-only",
    "operation.real-process-network-disabled",
    "operation.reconciliation-unimplemented",
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
    "operation.composed-partial-refused",
];

const WORK_EFFECTS: [&str; 14] = [
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
];

const JOURNAL_TRUE: [&str; 31] = [
    "journal.old-only-replay",
    "journal.new-only-replay",
    "journal.mixed-replay",
    "journal.v1-prefix-byte-identical",
    "journal.corrupt-prefix-refused",
    "journal.corrupt-payload-refused",
    "journal.truncated-transition-refused",
    "journal.head-mismatch-refused",
    "journal.stale-fence-refused",
    "journal.all-upgrade-crashes-recover",
    "journal.all-append-crashes-recover",
    "journal.paused-old-writer-cas-loses",
    "journal.old-residue-inert",
    "journal.old-winner-included",
    "journal.competing-upgrades-one-winner",
    "journal.conflicting-new-writers-one-head",
    "journal.progress-after-every-recovery",
    "journal.correspondence-index-rebuilds",
    "journal.object-index-rebuilds",
    "journal.no-v3-schema-reader",
    "journal.performance.isolated-processes",
    "journal.performance.active-reference",
    "journal.semantic-deltas-cause-only",
    "journal.evidence-set-roots-persistent",
    "journal.snapshots-verified",
    "journal.snapshot-damage-refused",
    "journal.retention-roots-explicit",
    "journal.telemetry-separate",
    "journal.unchanged-10000-no-records",
    "journal.unchanged-10000-no-linear-evidence-copy",
    "journal.composed-partial-refused",
];
