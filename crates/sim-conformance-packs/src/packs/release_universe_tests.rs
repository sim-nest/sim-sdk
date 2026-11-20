// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::ProofLeaf;

use super::super::receipt;
use crate::{MemorySubject, PackRequest, PackVerdict, all_packs, find_pack};

const SUBJECT: &str = "subject/source-universe";
const ACTIVATION: &str =
    "core/sha256-datum-v1:f12e44d0825b1b54bec8ddfab96007eccc97b9dbcf99ac8ddfe6cb34fc720cf3";
const REASONS: &str = "complete missing-target missing-resolver missing-checksum missing-feature missing-config missing-toolchain missing-nested unread-policy unbound-fact exact";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn the_loaded_source_universe_is_admitted_as_claim_10() {
    let loaded = sim_lib_world::universe::execute().expect("world seal");
    assert_eq!(loaded.unread_seal, loaded.exact_seal);
    assert_ne!(loaded.changed_fact_seal, loaded.exact_seal);
    assert_ne!(loaded.changed_policy_seal, loaded.exact_seal);
    assert_ne!(loaded.changed_fact_seal, loaded.changed_policy_seal);
    let observed = super::observe_source_universe(SUBJECT).expect("source universe");
    let carried = receipt_seal(&observed);
    let execution = format!("{REASONS} {}", carried);
    assert_eq!(carried, loaded.exact_seal);
    assert_eq!(carried, loaded.unread_seal);
    assert_ne!(carried, loaded.changed_fact_seal);
    assert_ne!(carried, loaded.changed_policy_seal);
    assert_ne!(carried, CONSTANT_SEAL);
    assert_eq!(observed.result, "completion/claim-10");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.execution, execution);
    assert!(observed.execution.ends_with(&carried));
    assert!(!observed.execution.ends_with(CONSTANT_SEAL));
    assert!(!observed.execution.contains("completion/claim-"));
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/source-universe").expect("scope")
    );
    assert_eq!(observed.revocation, RevocationStatus::Current);
    super::bind_source_universe(&observed).expect("bind");
    let mut forged = observed;
    forged.execution = format!("{REASONS} {CONSTANT_SEAL}");
    assert_eq!(
        super::bind_source_universe(&forged)
            .expect_err("constant seal")
            .code,
        "evidence-mismatch"
    );
    forged.execution = "scripted-result".to_owned();
    assert_eq!(
        super::bind_source_universe(&forged)
            .expect_err("forged")
            .code,
        "evidence-mismatch"
    );
    forged.execution = "completion/claim-10".to_owned();
    assert_eq!(
        super::bind_source_universe(&forged)
            .expect_err("labelled execution")
            .code,
        "claim-relabelled"
    );
    forged.execution = format!("{REASONS} {}", loaded.exact_seal);
    forged.result = "completion/claim-11".to_owned();
    assert_eq!(
        super::bind_source_universe(&forged)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
}

#[test]
fn a_claim_label_supplies_nothing() {
    let error = super::observe_source_universe("completion/claim-10").expect_err("label");
    assert_eq!(error.code, "bad-record");
    let spec = find_pack("checker/c-release").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    let evidence = MemorySubject::default().with(super::FACT, "completion/claim-10");
    match super::super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &binding,
        subject: &rendered,
        scope: super::SCOPE,
        evidence: &evidence,
    }) {
        PackVerdict::Refused(failure) => assert_eq!(failure.code, "claim-relabelled"),
        other => panic!("claim text must be refused, got {other:?}"),
    }
}

#[test]
fn a_missing_fact_supplies_nothing() {
    let spec = find_pack("checker/c-release").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    match super::super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &binding,
        subject: &rendered,
        scope: super::SCOPE,
        evidence: &MemorySubject::default(),
    }) {
        PackVerdict::Refused(failure) => assert_eq!(failure.code, "missing-source-universe"),
        other => panic!("missing fact must be refused, got {other:?}"),
    }
}

#[test]
fn a_forged_execution_is_refused() {
    let forged = vec![super::observation(
        "completion.execution",
        "scripted-result",
    )];
    let error = super::admit(SUBJECT, &forged).expect_err("forged");
    assert_eq!(error.code, "evidence-mismatch");
}

#[test]
fn the_source_universe_scope_is_implemented_without_changing_activation() {
    let spec = find_pack("checker/c-release").expect("release");
    assert!(spec.allowed_scopes.contains(&"completion/source-universe"));
    assert!(
        spec.implemented_scopes
            .contains(&"completion/source-universe")
    );
    assert_eq!(spec.activation_binding, ACTIVATION);
    assert_eq!(all_packs().len(), 21);
}

fn receipt_seal(observed: &super::ObservedSourceUniverse) -> String {
    match observed.catalog.leaf("source-universe").expect("leaf") {
        ProofLeaf::ArtifactEquality { left, right, .. } => {
            assert_eq!(left, right);
            String::from_utf8(left.clone()).expect("seal text")
        }
        _ => panic!("source universe leaf is not the production seal"),
    }
}
