// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{
    CheckScopeId, CheckedSubjectId, CheckerResultId, EvidenceGrade, RevocationStatus,
};
use sim_lib_roadmap_runner::EvidenceDecision;

use crate::{CheckObservation, MemorySubject, PackRequest};

use super::{admit_parsed, observations};

const SUBJECT: &str = "subject/source-example";
const REV: &str = "0123456789abcdef0123456789abcdef01234567";

fn manifest() -> String {
    format!(
        "publish = false\nsim-example = {{ version = \"0.1.0\", git = \"https://example.invalid/sim-example.git\", rev = \"{REV}\" }}\n"
    )
}

fn identity() -> String {
    format!("git+{REV}:sim-example@0.1.0")
}

fn request<'a>(evidence: &'a MemorySubject) -> PackRequest<'a> {
    PackRequest {
        checker: "checker/c-release",
        binding: "core/sha256-datum-v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        subject: "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        scope: "release/source-identity",
        evidence,
    }
}

#[test]
fn a_parsed_manifest_admits_its_source_receipt() {
    let text = manifest();
    let observed = super::observe_source_identity(SUBJECT, &text).expect("admitted");
    let expected = format!("completion/claim-21 {}", identity());
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.result, expected);
    assert_eq!(observed.revocation, RevocationStatus::Current);
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/source-identity").expect("scope")
    );
    let receipt = observed
        .catalog
        .checker_receipt(&observed.checker, RevocationStatus::Current)
        .expect("receipt");
    assert_eq!(
        receipt.subject(),
        &CheckedSubjectId::from_text(SUBJECT).expect("subject")
    );
    assert_ne!(
        receipt.subject(),
        &CheckedSubjectId::from_text("subject/other").expect("other")
    );
    assert_eq!(
        receipt.result(),
        &CheckerResultId::from_text(&expected).expect("result")
    );
    match observed
        .catalog
        .decide_evidence(&observed.admitted.key, EvidenceGrade::Release, |_| {
            RevocationStatus::Current
        }) {
        EvidenceDecision::Reuse { receipt } => assert_eq!(receipt, observed.admitted.id),
        other => panic!("expected release reuse, got {other:?}"),
    }
}

#[test]
fn shade_1_produces_no_receipt() {
    let text = format!("publish = false\nshade-1\n{}", manifest());
    match super::observe_source_identity(SUBJECT, &text) {
        Err(error) => assert_eq!(error.code, "construction-specimen"),
        Ok(_) => panic!("shade-1 must not admit a receipt"),
    }
}

#[test]
fn publish_true_produces_no_receipt() {
    let text = manifest().replacen("publish = false", "publish = true", 1);
    match super::observe_source_identity(SUBJECT, &text) {
        Err(error) => assert_eq!(error.code, "local-publication-claim"),
        Ok(_) => panic!("publish = true must not admit a receipt"),
    }
}

#[test]
fn an_extra_fact_is_refused() {
    let evidence = MemorySubject::default()
        .with("release.manifest", manifest())
        .with("release.extra", "1");
    let error = observations(&request(&evidence)).expect_err("extra");
    assert_eq!(error.code, "unexpected-fact");
}

#[test]
fn a_forged_source_observation_is_refused() {
    let text = manifest();
    let evidence = MemorySubject::default().with("release.manifest", text.clone());
    let mut seen = observations(&request(&evidence)).expect("observations");
    let source = seen
        .iter_mut()
        .find(|item| item.key == "release.source")
        .expect("source");
    source.value = "git+not-an-identity".to_owned();
    match admit_parsed(SUBJECT, &text, &seen) {
        Err(error) => assert_eq!(error.code, "evidence-mismatch"),
        Ok(_) => panic!("a forged source observation must not admit a receipt"),
    }
}

#[test]
fn another_subject_is_another_receipt() {
    let text = manifest();
    let first = super::observe_source_identity(SUBJECT, &text).expect("first");
    let second = super::observe_source_identity("subject/other", &text).expect("second");
    assert_eq!(first.result, second.result);
    assert_ne!(first.checker, second.checker);
    assert_ne!(first.admitted.id, second.admitted.id);
}

#[test]
fn observations_name_only_parsed_identities() {
    let evidence = MemorySubject::default().with("release.manifest", manifest());
    let seen = observations(&request(&evidence)).expect("observations");
    assert_eq!(
        seen,
        vec![
            CheckObservation {
                key: "release.unpublished".to_owned(),
                value: "publish-false".to_owned(),
            },
            CheckObservation {
                key: "release.source".to_owned(),
                value: identity(),
            },
        ]
    );
}
