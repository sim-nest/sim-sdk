// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use sim_conformance_v02::{
    CheckScopeId, CheckedSubjectId, CheckerResultId, EvidenceGrade, RevocationStatus,
};
use sim_kernel::Symbol;
use sim_lib_journal::{Journal, JournalEntry, JournalObject, MemoryBackend};
use sim_lib_roadmap_runner::{EvidenceDecision, ProofLeaf};

use crate::{MemorySubject, PackRequest, PackVerdict};

use super::observations;

const SUBJECT: &str = "subject/replay-example";
const RESULT: &str = "completion/claim-17";

fn executed_body(subject: &str) -> String {
    let backend = Arc::new(MemoryBackend::new());
    let journal = Journal::new(Arc::clone(&backend));
    let lease = journal.acquire_lease().expect("lease");
    let object = JournalObject::from_bytes(subject.as_bytes().to_vec());
    let kind = Symbol::qualified("world", "fact");
    let first = JournalEntry::new(0, None, kind.clone(), vec![object.id.clone()]);
    let head = journal
        .publish(&lease, None, vec![object.clone()], vec![first.clone()])
        .expect("publish");
    let second = JournalEntry::new(1, Some(first.id), kind, vec![object.id.clone()]);
    journal
        .publish(&lease, Some(&head), Vec::new(), vec![second])
        .expect("reference");
    let qualified = object_kind();
    super::format_executed_journal(&super::ExecutedJournal {
        subject: subject.to_owned(),
        objects: vec![object.bytes],
        entries: vec![entry(0, &qualified), entry(1, &qualified)],
        telemetry: vec!["side-channel".to_owned()],
    })
}

fn object_kind() -> (String, String) {
    let qualified = Symbol::qualified("world", "fact").as_qualified_str();
    let (namespace, name) = qualified.split_once('/').expect("kind");
    (namespace.to_owned(), name.to_owned())
}

fn entry(sequence: u64, kind: &(String, String)) -> super::ExecutedEntry {
    super::ExecutedEntry {
        sequence,
        namespace: kind.0.clone(),
        name: kind.1.clone(),
        object: 0,
    }
}

fn digest_list() -> String {
    (0..29)
        .map(|index| format!("phase/{index:02} {index:064x}\n"))
        .collect()
}

fn refuses(text: &str, code: &str) {
    refuses_for(SUBJECT, text, code);
}

fn refuses_for(subject: &str, text: &str, code: &str) {
    match super::observe_journal_completion(subject, text) {
        Err(error) => assert_eq!(error.code, code, "{subject}: {text}"),
        Ok(_) => panic!("{code} must not admit a receipt for {subject}"),
    }
}

fn subject_object(subject: &str) -> String {
    subject.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn rendered_subject(subject: &str) -> String {
    let checked = CheckedSubjectId::from_text(subject).expect("subject");
    super::render(checked.content_id())
}

fn public_check(subject: &str, body: &str) -> PackVerdict {
    let spec = crate::find_pack("checker/c-release").expect("checker");
    let binding = spec.checker_binding().expect("binding");
    let binding = super::render(binding.id().content_id());
    let rendered = rendered_subject(subject);
    let evidence = MemorySubject::default().with("journal.export", body);
    super::super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &binding,
        subject: &rendered,
        scope: "completion/replay",
        evidence: &evidence,
    })
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
fn an_executed_journal_admits_claim_17() {
    let text = executed_body(SUBJECT);
    assert!(text.contains(&format!("subject {SUBJECT}\n")));
    assert!(text.contains(&format!("object {}\n", subject_object(SUBJECT))));
    let observed = super::observe_journal_completion(SUBJECT, &text).expect("admitted");
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
    match observed.catalog.leaf("journal-replay").expect("leaf") {
        ProofLeaf::ArtifactEquality { left, right, .. } => {
            assert_eq!(left, right);
            assert_ne!(left.as_slice(), text.as_bytes());
            assert_ne!(right.as_slice(), text.as_bytes());
        }
        other => panic!("expected two replay encodings, got {other:?}"),
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
fn a_fabricated_digest_list_cannot_discharge_claim_17() {
    refuses(&digest_list(), "fabricated-digest-list");
    refuses(
        "phase/00 0000000000000000000000000000000000000000000000000000000000000001\n",
        "fabricated-digest-list",
    );
    let evidence = MemorySubject::default().with("release.replay-body", digest_list());
    match observations(&request(&evidence)) {
        Err(error) => assert_eq!(error.code, "unexpected-fact"),
        Ok(observations) => panic!("digest fact admitted {observations:?}"),
    }
}

#[test]
fn one_export_admits_only_its_bound_subject() {
    let text = executed_body(SUBJECT);
    let first = super::observe_journal_completion(SUBJECT, &text).expect("first");
    refuses_for("subject/other", &text, "subject-mismatch");
    let other = "subject/other";
    let other_text = executed_body(other);
    refuses_for(SUBJECT, &other_text, "subject-mismatch");
    let second = super::observe_journal_completion(other, &other_text).expect("second");
    assert_eq!(first.result, RESULT);
    assert_eq!(second.result, RESULT);
    assert_ne!(first.checker, second.checker);
    match (
        first.catalog.leaf("journal-replay").expect("first leaf"),
        second.catalog.leaf("journal-replay").expect("second leaf"),
    ) {
        (
            ProofLeaf::ArtifactEquality { left: bound, .. },
            ProofLeaf::ArtifactEquality { left: other, .. },
        ) => assert_ne!(bound, other),
        other => panic!("expected replay encodings, got {other:?}"),
    }
}

#[test]
fn an_unbound_export_admits_no_subject() {
    let probe = "\
journal-export-v1
object 09
entry 0 world fact 0
entry 1 world fact 0
telemetry side-channel
";
    refuses_for("subject/never-executed-a", probe, "bad-record");
    refuses_for("subject/never-executed-b", probe, "bad-record");
    let forged = "\
journal-export-v1
subject subject/never-executed-a
object 09
entry 0 world fact 0
entry 1 world fact 0
telemetry side-channel
";
    refuses_for("subject/never-executed-a", forged, "subject-mismatch");
    refuses_for("subject/never-executed-b", forged, "subject-mismatch");
}

#[test]
fn a_changed_object_refuses_the_bound_export() {
    let text = executed_body(SUBJECT);
    let marker = format!("object {}\n", subject_object(SUBJECT));
    let changed = text.replacen(&marker, "object 0a\n", 1);
    assert_ne!(changed, text);
    refuses(&changed, "subject-mismatch");
}

#[test]
fn refused_exports_produce_no_receipt() {
    refuses("", "missing-journal");
    refuses("\n", "missing-journal");
    refuses("shade-1\n", "construction-specimen");
    let mut shaded = executed_body(SUBJECT);
    shaded.push_str("shade-1\n");
    refuses(&shaded, "construction-specimen");
    refuses("PortableClosed\n", "local-publication-claim");
    refuses("publication-confirmed\n", "local-publication-claim");
    refuses("completion/claim-17\n", "local-publication-claim");
    let mut declared = executed_body(SUBJECT);
    declared.push_str("completion/claim-17\n");
    refuses(&declared, "local-publication-claim");
    refuses("not-a-journal\n", "bad-record");
    let mut bare = executed_body(SUBJECT);
    bare.pop();
    refuses(&bare, "bad-record");
    refuses(&executed_body(SUBJECT).replace('\n', "\r\n"), "bad-record");
    let kind = object_kind();
    let one = super::format_executed_journal(&super::ExecutedJournal {
        subject: SUBJECT.to_owned(),
        objects: vec![SUBJECT.as_bytes().to_vec()],
        entries: vec![entry(0, &kind)],
        telemetry: vec!["side-channel".to_owned()],
    });
    refuses(&one, "missing-journal");
    let silent = super::format_executed_journal(&super::ExecutedJournal {
        subject: SUBJECT.to_owned(),
        objects: vec![SUBJECT.as_bytes().to_vec()],
        entries: vec![entry(0, &kind), entry(1, &kind)],
        telemetry: Vec::new(),
    });
    refuses(&silent, "missing-journal");
}

#[test]
fn an_unexpected_fact_produces_no_observations() {
    let evidence = MemorySubject::default()
        .with("journal.export", executed_body(SUBJECT))
        .with("release.manifest", "[package]\npublish = false\n");
    match observations(&request(&evidence)) {
        Err(error) => assert_eq!(error.code, "unexpected-fact"),
        Ok(observations) => panic!("unexpected fact admitted {observations:?}"),
    }
}

#[test]
fn observations_name_the_executed_journal_laws() {
    let text = executed_body(SUBJECT);
    let evidence = MemorySubject::default().with("journal.export", text.clone());
    let rendered = rendered_subject(SUBJECT);
    let seen = observations(&PackRequest {
        checker: "checker/c-release",
        binding: "core/sha256-datum-v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        subject: &rendered,
        scope: "completion/replay",
        evidence: &evidence,
    })
    .expect("observations");
    let keys = seen
        .iter()
        .map(|item| item.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        vec![
            "journal.replay",
            "journal.reopen",
            "journal.snapshot",
            "journal.reference",
            "journal.telemetry",
        ]
    );
    assert!(
        seen.iter()
            .all(|item| item.value != text && !item.value.is_empty())
    );
    assert_eq!(seen[0].value, seen[1].value);
    assert_eq!(seen[0].value, seen[2].value);
    assert_eq!(seen[0].value, seen[4].value);
    assert_ne!(seen[3].value, seen[0].value);
}

#[test]
fn the_registered_checker_refuses_a_second_subject() {
    let body = executed_body("subject/never-executed-a");
    assert!(
        matches!(
            public_check("subject/never-executed-a", &body),
            PackVerdict::Pass { .. }
        ),
        "the bound subject must still pass the registered checker"
    );
    match public_check("subject/never-executed-b", &body) {
        PackVerdict::Refused(error) => assert_eq!(error.code, "subject-mismatch"),
        other => panic!("one export must not pass for a second subject, got {other:?}"),
    }
    let parent = "\
journal-export-v1
object 09
entry 0 world fact 0
entry 1 world fact 0
telemetry side-channel
";
    for subject in ["subject/never-executed-a", "subject/never-executed-b"] {
        assert!(
            matches!(public_check(subject, parent), PackVerdict::Refused(_)),
            "{subject} must refuse an export with no subject line"
        );
    }
    let evidence = MemorySubject::default().with("journal.export", body);
    match observations(&request(&evidence)) {
        Err(error) => assert_eq!(error.code, "subject-mismatch"),
        Ok(observations) => panic!("unbound request subject admitted {observations:?}"),
    }
}

#[test]
fn the_replay_scope_is_not_release_nv12_17() {
    let spec = crate::find_pack("checker/c-release").expect("checker");
    assert!(spec.allowed_scopes.contains(&"completion/replay"));
    assert!(spec.implemented_scopes.contains(&"completion/replay"));
    assert!(!spec.implemented_scopes.contains(&"release/nv12-17"));
}
