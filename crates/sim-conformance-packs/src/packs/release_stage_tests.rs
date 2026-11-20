// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use super::super::receipt;
use crate::{MemorySubject, PackRequest, PackVerdict, all_packs, find_pack};

const SUBJECT: &str = "subject/stage-one";
const ACTIVATION: &str =
    "core/sha256-datum-v1:f12e44d0825b1b54bec8ddfab96007eccc97b9dbcf99ac8ddfe6cb34fc720cf3";
const EXECUTION: &str = concat!(
    "oracle-reviewed denominator-40 bounds-100-95-70 ",
    "copied-refused roles-refused freeze-refused short-refused swapped-refused"
);

#[test]
fn the_reviewed_assay_is_admitted_as_claim_8() {
    let observed = super::observe_stage_one(SUBJECT).expect("stage one");
    assert_eq!(observed.result, "completion/claim-8");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.execution, EXECUTION);
    assert!(!observed.execution.contains("completion/claim-"));
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/stage-one").expect("scope")
    );
    assert_eq!(observed.revocation, RevocationStatus::Current);
    super::bind_stage_one(&observed).expect("bind");
    let mut forged = observed;
    forged.execution = "scripted-result".to_owned();
    assert_eq!(
        super::bind_stage_one(&forged).expect_err("forged").code,
        "evidence-mismatch"
    );
    forged.execution = "completion/claim-8".to_owned();
    assert_eq!(
        super::bind_stage_one(&forged)
            .expect_err("labelled execution")
            .code,
        "claim-relabelled"
    );
    forged.execution = EXECUTION.to_owned();
    forged.result = "completion/claim-9".to_owned();
    assert_eq!(
        super::bind_stage_one(&forged).expect_err("relabel").code,
        "claim-relabelled"
    );
}

#[test]
fn a_claim_label_supplies_nothing() {
    let error = super::observe_stage_one("completion/claim-8").expect_err("label");
    assert_eq!(error.code, "bad-record");
    let spec = find_pack("checker/c-release").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    let evidence = MemorySubject::default().with(super::FACT, "completion/claim-8");
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
        PackVerdict::Refused(failure) => assert_eq!(failure.code, "missing-stage-one"),
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
fn the_stage_one_scope_is_implemented_without_changing_activation() {
    let spec = find_pack("checker/c-release").expect("release");
    assert!(spec.allowed_scopes.contains(&"completion/stage-one"));
    assert!(spec.implemented_scopes.contains(&"completion/stage-one"));
    assert_eq!(spec.activation_binding, ACTIVATION);
    assert_eq!(all_packs().len(), 21);
}
