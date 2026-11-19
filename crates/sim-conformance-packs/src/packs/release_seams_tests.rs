// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use crate::{MemorySubject, PackRequest, PackVerdict, compiled_executor_closure, find_pack};

const SUBJECT: &str = "subject/seam-example";

#[test]
fn a_frozen_seam_specimen_is_admitted_as_claim_19() {
    let freeze = compiled_executor_closure();
    let observed = super::observe_portable_seams(SUBJECT, &freeze).expect("seams");
    assert_eq!(observed.result, "completion/claim-19");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.freeze, freeze);
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/seams").expect("scope")
    );
    assert_eq!(observed.revocation, RevocationStatus::Current);
    assert!(!observed.result.contains("completion/claim-17"));
    assert!(!observed.result.contains("completion/claim-21"));
    super::bind_portable_seams(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-18".to_owned();
    assert_eq!(
        super::bind_portable_seams(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
}

#[test]
fn a_freeze_mismatch_supplies_nothing() {
    let wrong = "0".repeat(64);
    let error = super::observe_portable_seams(SUBJECT, &wrong).expect_err("mismatch");
    assert_eq!(error.code, "freeze-mismatch");
}

#[test]
fn a_claim_label_supplies_nothing() {
    let error = super::observe_portable_seams(SUBJECT, "completion/claim-19").expect_err("label");
    assert_eq!(error.code, "bad-record");
    let spec = find_pack("checker/c-release").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = super::super::receipt::render(binding.id().content_id());
    let checked = CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let rendered = super::super::receipt::render(checked.content_id());
    let evidence = MemorySubject::default().with(
        "completion.seams",
        "subject subject/seam-example\nfreeze completion/claim-19\n",
    );
    match super::super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &binding,
        subject: &rendered,
        scope: "completion/seams",
        evidence: &evidence,
    }) {
        PackVerdict::Refused(failure) => assert_eq!(failure.code, "claim-relabelled"),
        other => panic!("claim text must be refused, got {other:?}"),
    }
}

#[test]
fn the_seam_scope_is_implemented() {
    let spec = find_pack("checker/c-release").expect("release");
    assert!(spec.allowed_scopes.contains(&"completion/seams"));
    assert!(spec.implemented_scopes.contains(&"completion/seams"));
}
