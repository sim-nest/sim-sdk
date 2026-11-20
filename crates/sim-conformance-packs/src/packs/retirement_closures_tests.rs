// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, ConformancePackId, RevocationStatus};
use sim_lib_roadmap_runner::ProofLeaf;

use crate::packs::release::receipt;
use crate::{MemorySubject, PackRequest, PackVerdict, all_packs, find_pack};

const SUBJECT: &str = "subject/closure-final";
const ACTIVATION: &str =
    "core/sha256-datum-v1:683334ada045fdb5fdfd3709100a703557df210835040bfe623d7b94ec00b01f";
const REASONS: &str = "closed executable reader schema oracle parser live-fact";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn the_loaded_closures_are_admitted_as_claim_3() {
    let loaded = sim_lib_world::closures::execute().expect("world seal");
    assert_ne!(loaded.executable_seal, loaded.closed_seal);
    assert_ne!(loaded.reader_seal, loaded.closed_seal);
    assert_ne!(loaded.schema_seal, loaded.closed_seal);
    assert_ne!(loaded.oracle_seal, loaded.closed_seal);
    assert_ne!(loaded.parser_seal, loaded.closed_seal);
    assert_ne!(loaded.live_fact_seal, loaded.closed_seal);
    let observed = super::observe_closure_final(SUBJECT).expect("closures");
    let owned = find_pack("checker/c-v3").expect("retirement");
    let receipt = observed
        .catalog
        .checker_receipt(&observed.checker, RevocationStatus::Current)
        .expect("retained receipt");
    let retirement_pack = ConformancePackId::from_text(&format!("pack/retirement@{ACTIVATION}"))
        .expect("retirement pack");
    assert_eq!(receipt.pack(), &retirement_pack);
    assert_eq!(owned.pack, "pack/retirement");
    assert_eq!(owned.activation_binding, ACTIVATION);
    let carried = receipt_seal(&observed);
    let execution = format!("{REASONS} {}", carried);
    let closed = sim_lib_world::closures::closures_digest("closed").expect("assess closed");
    let executable =
        sim_lib_world::closures::closures_digest("executable").expect("assess executable");
    let reader = sim_lib_world::closures::closures_digest("reader").expect("assess reader");
    let schema = sim_lib_world::closures::closures_digest("schema").expect("assess schema");
    let oracle = sim_lib_world::closures::closures_digest("oracle").expect("assess oracle");
    let parser = sim_lib_world::closures::closures_digest("parser").expect("assess parser");
    let live_fact =
        sim_lib_world::closures::closures_digest("live-fact").expect("assess live fact");
    assert_eq!(carried, closed);
    assert_eq!(loaded.closed_seal, closed);
    assert_eq!(loaded.executable_seal, executable);
    assert_eq!(loaded.reader_seal, reader);
    assert_eq!(loaded.schema_seal, schema);
    assert_eq!(loaded.oracle_seal, oracle);
    assert_eq!(loaded.parser_seal, parser);
    assert_eq!(loaded.live_fact_seal, live_fact);
    assert_ne!(carried, executable);
    assert_ne!(carried, CONSTANT_SEAL);
    assert_eq!(observed.result, "completion/claim-3");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.execution, execution);
    assert!(observed.execution.ends_with(&carried));
    assert!(!observed.execution.ends_with(CONSTANT_SEAL));
    assert!(!observed.execution.contains("completion/claim-"));
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/closure-final").expect("scope")
    );
    assert_eq!(observed.revocation, RevocationStatus::Current);
    super::bind_closure_final(&observed).expect("bind");
    let mut forged = observed;
    forged.execution = format!("{REASONS} {CONSTANT_SEAL}");
    assert_eq!(
        super::bind_closure_final(&forged)
            .expect_err("constant seal")
            .code,
        "evidence-mismatch"
    );
    forged.execution = "scripted-result".to_owned();
    assert_eq!(
        super::bind_closure_final(&forged).expect_err("forged").code,
        "evidence-mismatch"
    );
    forged.execution = "completion/claim-3".to_owned();
    assert_eq!(
        super::bind_closure_final(&forged)
            .expect_err("labelled execution")
            .code,
        "claim-relabelled"
    );
    forged.execution = format!("{REASONS} {}", loaded.closed_seal);
    forged.result = "completion/claim-2".to_owned();
    assert_eq!(
        super::bind_closure_final(&forged)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
}

#[test]
fn a_claim_label_supplies_nothing() {
    let error = super::observe_closure_final("completion/claim-3").expect_err("label");
    assert_eq!(error.code, "bad-record");
    let spec = find_pack("checker/c-v3").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    let evidence = MemorySubject::default().with(super::FACT, "completion/claim-3");
    match super::super::check(&PackRequest {
        checker: "checker/c-v3",
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
    let spec = find_pack("checker/c-v3").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    match super::super::check(&PackRequest {
        checker: "checker/c-v3",
        binding: &binding,
        subject: &rendered,
        scope: super::SCOPE,
        evidence: &MemorySubject::default(),
    }) {
        PackVerdict::Refused(failure) => assert_eq!(failure.code, "missing-closure-final"),
        other => panic!("missing fact must be refused, got {other:?}"),
    }
}

#[test]
fn a_retirement_guard_is_not_a_closure_final_fact() {
    let spec = find_pack("checker/c-v3").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    for key in ["retirement.guard-passed", "retirement-guard"] {
        let evidence = MemorySubject::default().with(key, "true");
        match super::super::check(&PackRequest {
            checker: "checker/c-v3",
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
fn the_closure_final_scope_is_implemented_without_changing_activation() {
    let spec = find_pack("checker/c-v3").expect("retirement");
    assert!(spec.allowed_scopes.contains(&"completion/closure-final"));
    assert!(
        spec.implemented_scopes
            .contains(&"completion/closure-final")
    );
    assert!(spec.implemented_scopes.contains(&"retirement/bootstrap"));
    assert!(!spec.implemented_scopes.contains(&"retirement/final"));
    assert!(spec.allowed_scopes.contains(&"retirement/final"));
    assert_eq!(spec.activation_binding, ACTIVATION);
    assert_eq!(all_packs().len(), 21);
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    match super::super::check(&PackRequest {
        checker: "checker/c-v3",
        binding: &binding,
        subject: &receipt::render(checked.content_id()),
        scope: "retirement/final",
        evidence: &MemorySubject::default(),
    }) {
        PackVerdict::UnimplementedPack { .. } => {}
        other => panic!("retirement/final must stay unimplemented, got {other:?}"),
    }
}

fn receipt_seal(observed: &super::ObservedClosureFinal) -> String {
    match observed.catalog.leaf("closure-final").expect("leaf") {
        ProofLeaf::ArtifactEquality { left, right, .. } => {
            assert_eq!(left, right);
            String::from_utf8(left.clone()).expect("seal text")
        }
        _ => panic!("closure final leaf is not the production seal"),
    }
}
