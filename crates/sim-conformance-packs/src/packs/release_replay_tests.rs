// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{
    CheckScopeId, CheckedSubjectId, CheckerResultId, EvidenceGrade, RevocationStatus,
};
use sim_lib_roadmap_runner::{EvidenceDecision, ProofLeaf};

use crate::{CheckObservation, MemorySubject, PackRequest};

use super::observations;

const SUBJECT: &str = "subject/replay-example";
const RESULT: &str = "completion/claim-17";

fn digest(index: usize) -> String {
    format!("{index:064x}")
}

fn line(index: usize, digest: &str) -> String {
    format!("phase/{index:02} {digest}")
}

fn body_from(lines: &[String]) -> String {
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    text
}

fn full_lines() -> Vec<String> {
    (0..29).map(|index| line(index, &digest(index))).collect()
}

fn full_body() -> String {
    body_from(&full_lines())
}

fn refuses(text: &str, code: &str) {
    match super::observe_phase_replay(SUBJECT, text) {
        Err(error) => assert_eq!(error.code, code, "{text}"),
        Ok(_) => panic!("{code} must not admit a receipt"),
    }
}

fn request<'a>(evidence: &'a MemorySubject) -> PackRequest<'a> {
    PackRequest {
        checker: "checker/c-release",
        binding: "core/sha256-datum-v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        subject: "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        scope: "completion/replay",
        evidence,
    }
}

#[test]
fn an_executed_replay_body_admits_claim_17() {
    let text = full_body();
    let observed = super::observe_phase_replay(SUBJECT, &text).expect("admitted");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.result, RESULT);
    assert!(!observed.result.contains(' '));
    assert!(!text.contains("completion/claim-"));
    assert_eq!(observed.revocation, RevocationStatus::Current);
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/replay").expect("scope")
    );
    assert_ne!(
        observed.scope,
        CheckScopeId::from_text("release/nv12-17").expect("other scope")
    );
    let receipt = observed
        .catalog
        .checker_receipt(&observed.checker, RevocationStatus::Current)
        .expect("receipt");
    assert_eq!(
        receipt.subject(),
        &CheckedSubjectId::from_text(SUBJECT).expect("subject")
    );
    assert_eq!(
        receipt.result(),
        &CheckerResultId::from_text(RESULT).expect("result")
    );
    assert_eq!(receipt.scope(), &observed.scope);
    match observed.catalog.leaf("replay-digest").expect("leaf") {
        ProofLeaf::ArtifactEquality { left, right, .. } => {
            assert_eq!(left, text.as_bytes());
            assert_eq!(right, text.as_bytes());
        }
        other => panic!("expected the replay body leaf, got {other:?}"),
    }
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
fn the_receipt_names_the_caller_subject() {
    let text = full_body();
    let first = super::observe_phase_replay(SUBJECT, &text).expect("first");
    let second = super::observe_phase_replay("subject/other", &text).expect("second");
    assert_eq!(first.result, RESULT);
    assert_eq!(second.result, RESULT);
    let first_receipt = first
        .catalog
        .checker_receipt(&first.checker, RevocationStatus::Current)
        .expect("first receipt");
    let second_receipt = second
        .catalog
        .checker_receipt(&second.checker, RevocationStatus::Current)
        .expect("second receipt");
    assert_ne!(first_receipt.subject(), second_receipt.subject());
    assert_ne!(first.checker, second.checker);
}

#[test]
fn a_changed_digest_changes_the_receipt() {
    let text = full_body();
    let first = super::observe_phase_replay(SUBJECT, &text).expect("first");
    let mut lines = full_lines();
    lines[4] = line(4, &digest(40));
    let changed = body_from(&lines);
    let second = super::observe_phase_replay(SUBJECT, &changed).expect("second");
    assert_eq!(first.result, second.result);
    assert_ne!(first.checker, second.checker);
    match second.catalog.leaf("replay-digest").expect("leaf") {
        ProofLeaf::ArtifactEquality { left, right, .. } => {
            assert_eq!(left, changed.as_bytes());
            assert_eq!(right, changed.as_bytes());
            assert_ne!(left, text.as_bytes());
        }
        other => panic!("expected the replay body leaf, got {other:?}"),
    }
}

#[test]
fn refused_bodies_produce_no_receipt() {
    refuses("", "missing-replay");
    refuses("\n", "missing-replay");
    refuses("shade-1\n", "construction-specimen");
    let mut shaded = full_body();
    shaded.push_str("shade-1\n");
    refuses(&shaded, "construction-specimen");
    refuses("PortableClosed\n", "local-publication-claim");
    refuses("publication-confirmed\n", "local-publication-claim");
    refuses("completion/claim-17\n", "local-publication-claim");
    let mut declared = full_body();
    declared.push_str("completion/claim-17\n");
    refuses(&declared, "local-publication-claim");
    refuses(&body_from(&full_lines()[..28]), "bad-record");
    let mut extra = full_lines();
    extra.push(line(29, &digest(29)));
    refuses(&body_from(&extra), "bad-record");
    let mut skipped = full_lines();
    skipped[5] = line(6, &digest(5));
    refuses(&body_from(&skipped), "bad-record");
    let mut upper = full_lines();
    upper[1] = line(1, &"A".repeat(64));
    refuses(&body_from(&upper), "bad-record");
    let mut bare = full_body();
    bare.pop();
    refuses(&bare, "bad-record");
    refuses(&full_body().replace('\n', "\r\n"), "bad-record");
    refuses("phase/00\n", "bad-record");
}

#[test]
fn an_unexpected_fact_produces_no_observations() {
    let evidence = MemorySubject::default()
        .with("release.replay-body", full_body())
        .with("release.manifest", "[package]\npublish = false\n");
    match observations(&request(&evidence)) {
        Err(error) => assert_eq!(error.code, "unexpected-fact"),
        Ok(observations) => panic!("unexpected fact admitted {observations:?}"),
    }
}

#[test]
fn observations_name_each_executed_leaf() {
    let evidence = MemorySubject::default().with("release.replay-body", full_body());
    let seen = observations(&request(&evidence)).expect("observations");
    let expected = full_lines()
        .into_iter()
        .map(|value| CheckObservation {
            key: "release.replay".to_owned(),
            value,
        })
        .collect::<Vec<_>>();
    assert_eq!(seen, expected);
}

#[test]
fn the_replay_scope_is_not_release_nv12_17() {
    let spec = crate::find_pack("checker/c-release").expect("checker");
    assert!(spec.allowed_scopes.contains(&"completion/replay"));
    assert!(spec.implemented_scopes.contains(&"completion/replay"));
    assert!(!spec.implemented_scopes.contains(&"release/nv12-17"));
}
