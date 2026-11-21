// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, ConformancePackId, RevocationStatus};
use sim_lib_roadmap_runner::ProofLeaf;

use crate::packs::release::receipt;
use crate::{MemorySubject, PackRequest, PackVerdict, all_packs, find_pack};

const SUBJECT: &str = "subject/constructor-final";
const ACTIVATION: &str =
    "core/sha256-datum-v1:9c5981c7fc65df78db1079ee58429821ea93bb36dd18c241a03ed63e24d2b75b";
const REASONS: &str = "canonical noncanonical unverified storage-id compatibility-id fingerprint";
const CONSTANT_SEAL: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn the_loaded_constructors_are_admitted_as_claim_5() {
    let loaded = sim_lib_world::constructors::execute().expect("world seal");
    assert_ne!(loaded.noncanonical_seal, loaded.canonical_seal);
    assert_ne!(loaded.unverified_seal, loaded.canonical_seal);
    assert_ne!(loaded.storage_id_seal, loaded.canonical_seal);
    assert_ne!(loaded.compatibility_id_seal, loaded.canonical_seal);
    assert_ne!(loaded.fingerprint_seal, loaded.canonical_seal);
    let observed = super::observe_constructor_final(SUBJECT).expect("constructors");
    let owned = find_pack("checker/c-id").expect("identity");
    let receipt = observed
        .catalog
        .checker_receipt(&observed.checker, RevocationStatus::Current)
        .expect("retained receipt");
    let identity_pack = ConformancePackId::from_text(&format!("pack/identity@{ACTIVATION}"))
        .expect("identity pack");
    assert_eq!(receipt.pack(), &identity_pack);
    assert_eq!(owned.pack, "pack/identity");
    assert_eq!(owned.activation_binding, ACTIVATION);
    let carried = receipt_seal(&observed);
    let execution = format!("{REASONS} {}", carried);
    let canonical =
        sim_lib_world::constructors::constructors_digest("canonical").expect("canonical");
    let noncanonical =
        sim_lib_world::constructors::constructors_digest("noncanonical").expect("noncanonical");
    let unverified =
        sim_lib_world::constructors::constructors_digest("unverified").expect("unverified");
    let storage_id =
        sim_lib_world::constructors::constructors_digest("storage-id").expect("storage");
    let compatibility_id = sim_lib_world::constructors::constructors_digest("compatibility-id")
        .expect("compatibility");
    let fingerprint =
        sim_lib_world::constructors::constructors_digest("fingerprint").expect("fingerprint");
    assert_eq!(carried, canonical);
    assert_eq!(loaded.canonical_seal, canonical);
    assert_eq!(loaded.noncanonical_seal, noncanonical);
    assert_eq!(loaded.unverified_seal, unverified);
    assert_eq!(loaded.storage_id_seal, storage_id);
    assert_eq!(loaded.compatibility_id_seal, compatibility_id);
    assert_eq!(loaded.fingerprint_seal, fingerprint);
    assert_ne!(carried, noncanonical);
    assert_ne!(carried, CONSTANT_SEAL);
    assert_eq!(observed.result, "completion/claim-5");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.execution, execution);
    assert!(observed.execution.ends_with(&carried));
    assert!(!observed.execution.ends_with(CONSTANT_SEAL));
    assert!(!observed.execution.contains("completion/claim-"));
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/constructor-final").expect("scope")
    );
    assert_eq!(observed.revocation, RevocationStatus::Current);
    super::bind_constructor_final(&observed).expect("bind");
    let mut forged = observed;
    forged.execution = format!("{REASONS} {CONSTANT_SEAL}");
    assert_eq!(
        super::bind_constructor_final(&forged)
            .expect_err("constant seal")
            .code,
        "evidence-mismatch"
    );
    forged.execution = "scripted-result".to_owned();
    assert_eq!(
        super::bind_constructor_final(&forged)
            .expect_err("forged")
            .code,
        "evidence-mismatch"
    );
    forged.execution = "completion/claim-5".to_owned();
    assert_eq!(
        super::bind_constructor_final(&forged)
            .expect_err("labelled execution")
            .code,
        "claim-relabelled"
    );
    forged.execution = format!("{REASONS} {}", loaded.canonical_seal);
    forged.result = "completion/claim-2".to_owned();
    assert_eq!(
        super::bind_constructor_final(&forged)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
}

#[test]
fn a_claim_label_supplies_nothing() {
    let error = super::observe_constructor_final("completion/claim-5").expect_err("label");
    assert_eq!(error.code, "bad-record");
    let spec = find_pack("checker/c-id").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    let evidence = MemorySubject::default().with(super::FACT, "completion/claim-5");
    match super::super::check(&PackRequest {
        checker: "checker/c-id",
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
    let spec = find_pack("checker/c-id").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    match super::super::check(&PackRequest {
        checker: "checker/c-id",
        binding: &binding,
        subject: &rendered,
        scope: super::SCOPE,
        evidence: &MemorySubject::default(),
    }) {
        PackVerdict::Refused(failure) => assert_eq!(failure.code, "missing-constructor-final"),
        other => panic!("missing fact must be refused, got {other:?}"),
    }
}

#[test]
fn an_identity_roundtrip_is_not_a_constructor_final_fact() {
    let spec = find_pack("checker/c-id").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = receipt::render(checked.content_id());
    for key in [
        "identity.roundtrip-semantic-algorithm",
        "identity-roundtrip",
    ] {
        let evidence = MemorySubject::default().with(key, "true");
        match super::super::check(&PackRequest {
            checker: "checker/c-id",
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
fn the_constructor_final_scope_is_implemented_without_changing_activation() {
    let spec = find_pack("checker/c-id").expect("identity");
    assert!(
        spec.allowed_scopes
            .contains(&"completion/constructor-final")
    );
    assert!(
        spec.implemented_scopes
            .contains(&"completion/constructor-final")
    );
    assert!(
        spec.implemented_scopes
            .contains(&"identity/store-roundtrip")
    );
    assert!(spec.allowed_scopes.contains(&"identity/fixed-point"));
    assert!(!spec.implemented_scopes.contains(&"identity/fixed-point"));
    assert_eq!(spec.activation_binding, ACTIVATION);
    assert_eq!(all_packs().len(), 21);
    let binding = spec.checker_binding().expect("binding");
    let binding = receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    match super::super::check(&PackRequest {
        checker: "checker/c-id",
        binding: &binding,
        subject: &receipt::render(checked.content_id()),
        scope: "identity/fixed-point",
        evidence: &MemorySubject::default(),
    }) {
        PackVerdict::UnimplementedPack { .. } => {}
        other => panic!("identity/fixed-point must stay unimplemented, got {other:?}"),
    }
}

fn receipt_seal(observed: &super::ObservedConstructorFinal) -> String {
    match observed.catalog.leaf("constructor-final").expect("leaf") {
        ProofLeaf::ArtifactEquality { left, right, .. } => {
            assert_eq!(left, right);
            String::from_utf8(left.clone()).expect("seal text")
        }
        _ => panic!("constructor final leaf is not the production seal"),
    }
}
