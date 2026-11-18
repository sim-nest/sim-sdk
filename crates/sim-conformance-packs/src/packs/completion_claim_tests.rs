// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use sim_conformance_v02::{CheckScopeId, RevocationStatus};
use sim_kernel::Symbol;
use sim_lib_journal::{Journal, JournalEntry, JournalObject, MemoryBackend};

use crate::packs::release::{ExecutedEntry, ExecutedJournal, format_executed_journal};

const SUBJECT: &str = "subject/replay-example";

fn executed_export(subject: &str) -> String {
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
    let qualified = Symbol::qualified("world", "fact").as_qualified_str();
    let (namespace, name) = qualified.split_once('/').expect("kind");
    format_executed_journal(&ExecutedJournal {
        subject: subject.to_owned(),
        objects: vec![object.bytes],
        entries: vec![
            ExecutedEntry {
                sequence: 0,
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                object: 0,
            },
            ExecutedEntry {
                sequence: 1,
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                object: 0,
            },
        ],
        telemetry: vec!["side-channel".to_owned()],
    })
}

fn manifest() -> String {
    "\
[package]
publish = false
[dependencies]
sim-example = { version = \"0.1.0\", git = \"https://example.invalid/sim-example.git\", rev = \"0123456789abcdef0123456789abcdef01234567\" }
"
    .to_owned()
}

#[test]
fn journal_binding_keeps_the_replay_receipt() {
    let export = executed_export(SUBJECT);
    let observed =
        crate::packs::release::observe_journal_completion(SUBJECT, &export).expect("journal");
    let bound = super::bind_journal_receipt(observed).expect("bound");
    assert_eq!(bound.observation.subject, SUBJECT);
    assert_eq!(bound.observation.result, "completion/claim-17");
    assert_eq!(bound.observation.revocation, RevocationStatus::Current);
    assert_eq!(
        bound.observation.scope,
        CheckScopeId::from_text("completion/replay").expect("scope")
    );
    assert!(!bound.observation.result.contains(' '));
}

#[test]
fn source_binding_stays_out_of_claim_21() {
    let observed =
        crate::packs::release::observe_source_identity(SUBJECT, &manifest()).expect("source");
    let bound = super::bind_source_receipt(observed).expect("bound");
    assert_eq!(bound.observation.subject, SUBJECT);
    assert!(bound.observation.result.starts_with("source-identity "));
    assert!(!bound.observation.result.contains("completion/claim-"));
    assert_eq!(bound.observation.revocation, RevocationStatus::Current);
    assert_eq!(
        bound.observation.scope,
        CheckScopeId::from_text("release/source-identity").expect("scope")
    );
}

#[test]
fn a_relabelled_journal_result_is_refused() {
    let export = executed_export(SUBJECT);
    let observed =
        crate::packs::release::observe_journal_completion(SUBJECT, &export).expect("journal");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-20".to_owned();
    match super::bind_journal_receipt(relabelled) {
        Err(error) => assert_eq!(error.code, "claim-relabelled"),
        Ok(_) => panic!("relabelled journal result was bound"),
    }
}

#[test]
fn a_source_result_that_names_a_claim_is_refused() {
    let observed =
        crate::packs::release::observe_source_identity(SUBJECT, &manifest()).expect("source");
    let mut relabelled = observed;
    relabelled.result =
        "completion/claim-21 git+0123456789abcdef0123456789abcdef01234567:sim-example@0.1.0"
            .to_owned();
    match super::bind_source_receipt(relabelled) {
        Err(error) => assert_eq!(error.code, "claim-relabelled"),
        Ok(_) => panic!("source identity was promoted to claim 21"),
    }
}

#[test]
fn a_process_exit_is_the_status_the_process_returned() {
    let observed = super::execute_process("/bin/true", &[]).expect("ran");
    assert_eq!(observed.program, "/bin/true");
    assert_eq!(observed.status, 0);
    assert!(observed.stdout.is_empty());
}

#[test]
fn a_failed_process_is_not_a_passing_observation() {
    match super::execute_process("/bin/false", &[]) {
        Err(error) => assert_eq!(error.code, "process-failed"),
        Ok(_) => panic!("non-zero exit was observed as success"),
    }
}
