// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use super::super::receipt;
use crate::{MemorySubject, PackRequest, PackVerdict, all_packs, find_pack};

const SUBJECT: &str = "subject/sealed-convergence";
const ACTIVATION: &str =
    "core/sha256-datum-v1:f12e44d0825b1b54bec8ddfab96007eccc97b9dbcf99ac8ddfe6cb34fc720cf3";
const EXECUTION: &str = concat!(
    "sealed one-owner exact-postimage zero-work gap ",
    "duplicate-generator split-generator"
);

#[test]
fn a_sealed_fixed_point_is_admitted_as_claim_12() {
    let observed = super::observe_sealed_convergence(SUBJECT).expect("convergence");
    assert_eq!(observed.result, "completion/claim-12");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.execution, EXECUTION);
    assert!(observed.execution.contains("sealed"));
    assert!(observed.execution.contains("one-owner"));
    assert!(observed.execution.contains("exact-postimage"));
    assert!(observed.execution.contains("zero-work"));
    assert!(observed.execution.contains("gap"));
    assert!(observed.execution.contains("duplicate-generator"));
    assert!(observed.execution.contains("split-generator"));
    assert!(!observed.execution.contains("completion/claim-"));
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/sealed-convergence").expect("scope")
    );
    assert_eq!(observed.revocation, RevocationStatus::Current);
    super::bind_sealed_convergence(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-11".to_owned();
    assert_eq!(
        super::bind_sealed_convergence(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
}

#[test]
fn a_claim_label_supplies_nothing() {
    let error = super::observe_sealed_convergence("completion/claim-12").expect_err("label");
    assert_eq!(error.code, "bad-record");
    let spec = find_pack("checker/c-release").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    let evidence = MemorySubject::default().with(super::FACT, "completion/claim-12");
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
        PackVerdict::Refused(failure) => assert_eq!(failure.code, "missing-convergence"),
        other => panic!("missing fact must be refused, got {other:?}"),
    }
}

#[test]
fn the_convergence_scope_is_implemented_without_changing_activation() {
    let spec = find_pack("checker/c-release").expect("release");
    assert!(
        spec.allowed_scopes
            .contains(&"completion/sealed-convergence")
    );
    assert!(
        spec.implemented_scopes
            .contains(&"completion/sealed-convergence")
    );
    assert_eq!(spec.activation_binding, ACTIVATION);
    assert_eq!(all_packs().len(), 21);
}
