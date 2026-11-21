// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, ConformancePackId, RevocationStatus};
use sim_lib_roadmap_runner::ProofLeaf;

use crate::packs::release::receipt;
use crate::{MemorySubject, PackRequest, PackVerdict, all_packs, find_pack};

const SUBJECT: &str = "subject/grades-final";
const ACTIVATION: &str =
    "core/sha256-datum-v1:dfe5edd6c822eb896aa73f30cfbbde2a53a9fe9f5dd941f3e158d211631447ca";
const REASONS: &str =
    "held weak-grade revoked-support unstable-key frontier-open continuation-open";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn the_loaded_grades_are_admitted_as_claim_11() {
    let loaded = sim_lib_world::grades::execute().expect("world seal");
    assert_ne!(loaded.weak_grade_seal, loaded.held_seal);
    assert_ne!(loaded.revoked_seal, loaded.held_seal);
    assert_ne!(loaded.unstable_key_seal, loaded.held_seal);
    assert_ne!(loaded.frontier_open_seal, loaded.held_seal);
    assert_ne!(loaded.continuation_open_seal, loaded.held_seal);
    let observed = super::observe_grades_final(SUBJECT).expect("grades");
    let owned = find_pack("checker/c-evidence").expect("evidence");
    let receipt = observed
        .catalog
        .checker_receipt(&observed.checker, RevocationStatus::Current)
        .expect("retained receipt");
    let evidence_pack = ConformancePackId::from_text(&format!("pack/evidence@{ACTIVATION}"))
        .expect("evidence pack");
    assert_eq!(receipt.pack(), &evidence_pack);
    assert_eq!(owned.pack, "pack/evidence");
    assert_eq!(owned.activation_binding, ACTIVATION);
    let carried = receipt_seal(&observed);
    let execution = format!("{REASONS} {}", carried);
    let held = sim_lib_world::grades::grades_digest("held").expect("held");
    let weak_grade = sim_lib_world::grades::grades_digest("weak-grade").expect("weak-grade");
    let revoked = sim_lib_world::grades::grades_digest("revoked").expect("revoked");
    let unstable_key = sim_lib_world::grades::grades_digest("unstable-key").expect("unstable-key");
    let frontier_open =
        sim_lib_world::grades::grades_digest("frontier-open").expect("frontier-open");
    let continuation_open =
        sim_lib_world::grades::grades_digest("continuation-open").expect("continuation-open");
    assert_eq!(carried, held);
    assert_eq!(loaded.held_seal, held);
    assert_eq!(loaded.weak_grade_seal, weak_grade);
    assert_eq!(loaded.revoked_seal, revoked);
    assert_eq!(loaded.unstable_key_seal, unstable_key);
    assert_eq!(loaded.frontier_open_seal, frontier_open);
    assert_eq!(loaded.continuation_open_seal, continuation_open);
    assert_ne!(carried, weak_grade);
    assert_ne!(carried, CONSTANT_SEAL);
    assert_eq!(observed.result, "completion/claim-11");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.execution, execution);
    assert!(observed.execution.ends_with(&carried));
    assert!(!observed.execution.ends_with(CONSTANT_SEAL));
    assert!(!observed.execution.contains("completion/claim-"));
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/grades-final").expect("scope")
    );
    assert_eq!(observed.revocation, RevocationStatus::Current);
    super::bind_grades_final(&observed).expect("bind");
    let mut forged = observed;
    forged.execution = format!("{REASONS} {CONSTANT_SEAL}");
    assert_eq!(
        super::bind_grades_final(&forged)
            .expect_err("constant seal")
            .code,
        "evidence-mismatch"
    );
    forged.execution = "scripted-result".to_owned();
    assert_eq!(
        super::bind_grades_final(&forged).expect_err("forged").code,
        "evidence-mismatch"
    );
    forged.execution = "completion/claim-11".to_owned();
    assert_eq!(
        super::bind_grades_final(&forged)
            .expect_err("labelled execution")
            .code,
        "claim-relabelled"
    );
    forged.execution = format!("{REASONS} {}", loaded.held_seal);
    forged.result = "completion/claim-5".to_owned();
    assert_eq!(
        super::bind_grades_final(&forged).expect_err("relabel").code,
        "claim-relabelled"
    );
}

#[test]
fn a_claim_label_supplies_nothing() {
    let error = super::observe_grades_final("completion/claim-11").expect_err("label");
    assert_eq!(error.code, "bad-record");
    let spec = find_pack("checker/c-evidence").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    let evidence = MemorySubject::default().with(super::FACT, "completion/claim-11");
    match super::super::check(&PackRequest {
        checker: "checker/c-evidence",
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
    let spec = find_pack("checker/c-evidence").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    match super::super::check(&PackRequest {
        checker: "checker/c-evidence",
        binding: &binding,
        subject: &rendered,
        scope: super::SCOPE,
        evidence: &MemorySubject::default(),
    }) {
        PackVerdict::Refused(failure) => assert_eq!(failure.code, "missing-grades-final"),
        other => panic!("missing fact must be refused, got {other:?}"),
    }
}

#[test]
fn an_identity_grade_fact_is_not_a_grades_final_fact() {
    let spec = find_pack("checker/c-evidence").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    for key in [
        "evidence.identity-grade",
        "evidence.projection-domain-tag-unique",
    ] {
        let evidence = MemorySubject::default().with(key, "true");
        match super::super::check(&PackRequest {
            checker: "checker/c-evidence",
            binding: &binding,
            subject: &rendered,
            scope: super::SCOPE,
            evidence: &evidence,
        }) {
            PackVerdict::Refused(failure) => assert_eq!(failure.code, "unexpected-fact", "{key}"),
            other => panic!("{key} must be refused, got {other:?}"),
        }
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
fn the_grades_final_scope_is_implemented_without_changing_activation() {
    let spec = find_pack("checker/c-evidence").expect("evidence");
    assert!(spec.allowed_scopes.contains(&"completion/grades-final"));
    assert!(spec.implemented_scopes.contains(&"completion/grades-final"));
    assert!(spec.allowed_scopes.contains(&"evidence/identity-grade"));
    assert!(!spec.implemented_scopes.contains(&"evidence/identity-grade"));
    assert!(spec.allowed_scopes.contains(&"evidence/cold-oracle"));
    assert!(!spec.implemented_scopes.contains(&"evidence/cold-oracle"));
    assert!(spec.allowed_scopes.contains(&"evidence/convergence"));
    assert!(!spec.implemented_scopes.contains(&"evidence/convergence"));
    assert_eq!(spec.activation_binding, ACTIVATION);
    assert_eq!(all_packs().len(), 21);
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    match super::super::check(&PackRequest {
        checker: "checker/c-evidence",
        binding: &binding,
        subject: &receipt::render(checked.content_id()),
        scope: "evidence/identity-grade",
        evidence: &MemorySubject::default(),
    }) {
        PackVerdict::UnimplementedPack { .. } => {}
        other => panic!("evidence/identity-grade must stay unimplemented, got {other:?}"),
    }
}

fn receipt_seal(observed: &super::ObservedGradesFinal) -> String {
    match observed.catalog.leaf("grades-final").expect("leaf") {
        ProofLeaf::ArtifactEquality { left, right, .. } => {
            assert_eq!(left, right);
            String::from_utf8(left.clone()).expect("seal text")
        }
        _ => panic!("grades final leaf is not the production seal"),
    }
}
