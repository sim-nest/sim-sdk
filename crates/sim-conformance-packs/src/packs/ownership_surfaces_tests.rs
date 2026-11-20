// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::ProofLeaf;

use crate::packs::release::receipt;
use crate::{MemorySubject, PackRequest, PackVerdict, all_packs, find_pack};

const SUBJECT: &str = "subject/ownership-final";
const ACTIVATION: &str =
    "core/sha256-datum-v1:f794a7d0c15f8b9fe55e9a2de0f46b547491005d41909d4ede2968ff3632a95f";
const REASONS: &str = "exact planned unresolved-owner cycle overlap unimplemented";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn the_loaded_surfaces_are_admitted_as_claim_2() {
    let loaded = sim_lib_world::surfaces::execute().expect("world seal");
    assert_ne!(loaded.planned_seal, loaded.exact_seal);
    assert_ne!(loaded.unresolved_owner_seal, loaded.exact_seal);
    assert_ne!(loaded.cycle_seal, loaded.exact_seal);
    assert_ne!(loaded.overlap_seal, loaded.exact_seal);
    assert_ne!(loaded.unimplemented_seal, loaded.exact_seal);
    let observed = super::observe_ownership_final(SUBJECT).expect("ownership surfaces");
    let carried = receipt_seal(&observed);
    let execution = format!("{REASONS} {}", carried);
    let exact = sim_lib_world::surfaces::surfaces_digest("exact").expect("assess exact");
    let planned = sim_lib_world::surfaces::surfaces_digest("planned").expect("assess planned");
    let unresolved =
        sim_lib_world::surfaces::surfaces_digest("unresolved-owner").expect("assess owner");
    let cycle = sim_lib_world::surfaces::surfaces_digest("cycle").expect("assess cycle");
    let overlap = sim_lib_world::surfaces::surfaces_digest("overlap").expect("assess overlap");
    let unimplemented =
        sim_lib_world::surfaces::surfaces_digest("unimplemented").expect("assess gap");
    assert_eq!(carried, exact);
    assert_eq!(loaded.exact_seal, exact);
    assert_eq!(loaded.planned_seal, planned);
    assert_eq!(loaded.unresolved_owner_seal, unresolved);
    assert_eq!(loaded.cycle_seal, cycle);
    assert_eq!(loaded.overlap_seal, overlap);
    assert_eq!(loaded.unimplemented_seal, unimplemented);
    assert_ne!(carried, planned);
    assert_ne!(carried, CONSTANT_SEAL);
    assert_eq!(observed.result, "completion/claim-2");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.execution, execution);
    assert!(observed.execution.ends_with(&carried));
    assert!(!observed.execution.ends_with(CONSTANT_SEAL));
    assert!(!observed.execution.contains("completion/claim-"));
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/ownership-final").expect("scope")
    );
    assert_eq!(observed.revocation, RevocationStatus::Current);
    super::bind_ownership_final(&observed).expect("bind");
    let mut forged = observed;
    forged.execution = format!("{REASONS} {CONSTANT_SEAL}");
    assert_eq!(
        super::bind_ownership_final(&forged)
            .expect_err("constant seal")
            .code,
        "evidence-mismatch"
    );
    forged.execution = "scripted-result".to_owned();
    assert_eq!(
        super::bind_ownership_final(&forged)
            .expect_err("forged")
            .code,
        "evidence-mismatch"
    );
    forged.execution = "completion/claim-2".to_owned();
    assert_eq!(
        super::bind_ownership_final(&forged)
            .expect_err("labelled execution")
            .code,
        "claim-relabelled"
    );
    forged.execution = format!("{REASONS} {}", loaded.exact_seal);
    forged.result = "completion/claim-3".to_owned();
    assert_eq!(
        super::bind_ownership_final(&forged)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
}

#[test]
fn a_claim_label_supplies_nothing() {
    let error = super::observe_ownership_final("completion/claim-2").expect_err("label");
    assert_eq!(error.code, "bad-record");
    let spec = find_pack("checker/c-own").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    let evidence = MemorySubject::default().with(super::FACT, "completion/claim-2");
    match super::super::check(&PackRequest {
        checker: "checker/c-own",
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
    let spec = find_pack("checker/c-own").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    match super::super::check(&PackRequest {
        checker: "checker/c-own",
        binding: &binding,
        subject: &rendered,
        scope: super::SCOPE,
        evidence: &MemorySubject::default(),
    }) {
        PackVerdict::Refused(failure) => assert_eq!(failure.code, "missing-ownership-final"),
        other => panic!("missing fact must be refused, got {other:?}"),
    }
}

#[test]
fn a_succession_record_is_not_an_ownership_final_fact() {
    let spec = find_pack("checker/c-own").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    for (key, value) in [
        ("ownership.succession-record", "record"),
        ("ownership.edge.m2", "m6"),
    ] {
        let evidence = MemorySubject::default().with(key, value);
        match super::super::check(&PackRequest {
            checker: "checker/c-own",
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
fn the_ownership_final_scope_is_implemented_without_changing_activation() {
    let spec = find_pack("checker/c-own").expect("ownership");
    assert!(spec.allowed_scopes.contains(&"completion/ownership-final"));
    assert!(
        spec.implemented_scopes
            .contains(&"completion/ownership-final")
    );
    assert!(spec.implemented_scopes.contains(&"ownership/roadmap-final"));
    assert_eq!(spec.activation_binding, ACTIVATION);
    assert_eq!(all_packs().len(), 21);
}

fn receipt_seal(observed: &super::ObservedOwnershipFinal) -> String {
    match observed.catalog.leaf("ownership-final").expect("leaf") {
        ProofLeaf::ArtifactEquality { left, right, .. } => {
            assert_eq!(left, right);
            String::from_utf8(left.clone()).expect("seal text")
        }
        _ => panic!("ownership final leaf is not the production seal"),
    }
}
