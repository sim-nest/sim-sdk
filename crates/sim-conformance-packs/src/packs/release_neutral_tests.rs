// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use super::{
    CONSUMPTION, FACT, HANDOFF_PATH, RESULT, SCOPE, SLOT_DIGEST, SLOT_PATH, SOURCE_DIGEST,
    SOURCE_PATH, bind_neutral_input, observe_neutral_input,
};
use crate::{MemorySubject, PackRequest};

const SUBJECT: &str = "subject/neutral-input";

#[test]
fn the_reader_does_not_embed_private_measures() {
    let source = include_str!("release_neutral.rs");
    assert!(!source.contains("2027988000"));
    assert!(!source.contains("563.33"));
    assert!(!source.contains("neutral-input.sx\n"));
    assert!(source.contains(SLOT_PATH));
    assert!(source.contains(CONSUMPTION));
}

#[test]
fn the_scope_is_implemented_on_the_existing_release_checker() {
    let spec = crate::find_pack("checker/c-release").expect("pack");
    assert!(spec.allowed_scopes.contains(&SCOPE));
    assert!(spec.implemented_scopes.contains(&SCOPE));
    assert_eq!(crate::all_packs().len(), 21);
}

#[test]
fn a_claim_label_in_the_subject_supplies_nothing() {
    let dir = scratch("subject");
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, "example\n"),
            (HANDOFF_PATH, "handoff\n"),
            (SLOT_PATH, "slot\n"),
        ],
    );
    match observe_neutral_input("completion/claim-4", &coords(&dir, &rev)) {
        Err(error) => assert_eq!(error.code, "bad-record"),
        Ok(_) => panic!("a claim label admitted claim 4"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_relabelled_fact_supplies_nothing() {
    let evidence = MemorySubject::default().with(FACT, "completion/claim-4");
    let request = PackRequest {
        checker: "checker/c-release",
        binding: "binding",
        subject: SUBJECT,
        scope: SCOPE,
        evidence: &evidence,
    };
    let error = super::observations(&request).expect_err("relabel");
    assert_eq!(error.code, "claim-relabelled");
}

#[test]
fn another_source_path_supplies_nothing() {
    let dir = scratch("path");
    let rev = commit_files(
        &dir,
        &[("other.md", "example\n"), (HANDOFF_PATH, "handoff\n")],
    );
    let mut selected = coords(&dir, &rev);
    selected.source = "other.md".to_owned();
    match observe_neutral_input(SUBJECT, &selected) {
        Err(error) => {
            assert_eq!(error.code, "bad-record");
            assert_eq!(error.detail, "source");
        }
        Ok(_) => panic!("another source path admitted claim 4"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn recorded_digests_do_not_admit_other_bytes() {
    let dir = scratch("bytes");
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, "example\n"),
            (HANDOFF_PATH, &format!("{SOURCE_DIGEST}\n{SLOT_DIGEST}\n")),
            (SLOT_PATH, "example-slot\n"),
        ],
    );
    match observe_neutral_input(SUBJECT, &coords(&dir, &rev)) {
        Err(error) => {
            assert_eq!(error.code, "bad-record");
            assert_eq!(error.detail, "source-digest");
        }
        Ok(_) => panic!("other bytes admitted claim 4"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_slot_supplies_nothing() {
    let dir = scratch("missing");
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, "example\n"),
            (HANDOFF_PATH, &format!("{SOURCE_DIGEST}\n{SLOT_DIGEST}\n")),
        ],
    );
    match observe_neutral_input(SUBJECT, &coords(&dir, &rev)) {
        Err(error) => assert_eq!(error.code, "process-failed"),
        Ok(_) => panic!("a missing slot admitted claim 4"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_owner_slot_is_admitted_as_claim_4() {
    let Some(repo) = owner_repo() else {
        return;
    };
    let rev = head(&repo);
    let observed = observe_neutral_input(SUBJECT, &coords(&repo, &rev)).expect("owner slot");
    assert_eq!(observed.result, RESULT);
    assert_eq!(observed.source_digest, SOURCE_DIGEST);
    assert_eq!(observed.slot_digest, SLOT_DIGEST);
    assert!(!observed.result.contains("2027988000"));
    bind_neutral_input(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-1".to_owned();
    assert_eq!(
        bind_neutral_input(&relabelled).expect_err("relabel").code,
        "claim-relabelled"
    );
}

#[test]
fn a_flipped_owner_slot_byte_supplies_nothing() {
    let Some(repo) = owner_repo() else {
        return;
    };
    let rev = head(&repo);
    let mut slot = blob(&repo, &rev, SLOT_PATH);
    let index = slot
        .iter()
        .position(|byte| *byte == b'a')
        .expect("ascii byte");
    slot[index] = b'b';
    let dir = scratch("flip");
    let committed = commit_bytes(
        &dir,
        &[
            (SOURCE_PATH, &blob(&repo, &rev, SOURCE_PATH)),
            (HANDOFF_PATH, &blob(&repo, &rev, HANDOFF_PATH)),
            (SLOT_PATH, &slot),
        ],
    );
    match observe_neutral_input(SUBJECT, &coords(&dir, &committed)) {
        Err(error) => {
            assert_eq!(error.code, "bad-record");
            assert_eq!(error.detail, "slot-digest");
        }
        Ok(_) => panic!("a flipped slot admitted claim 4"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_slot_bytes_without_the_handoff_digest_supply_nothing() {
    let Some(repo) = owner_repo() else {
        return;
    };
    let rev = head(&repo);
    let slot = blob(&repo, &rev, SLOT_PATH);
    assert_eq!(sha256_hex(&slot), SLOT_DIGEST);
    let handoff = String::from_utf8(blob(&repo, &rev, HANDOFF_PATH)).expect("handoff utf-8");
    let stripped = handoff.replace(SLOT_DIGEST, &"0".repeat(64));
    assert!(!stripped.contains(SLOT_DIGEST));
    let dir = scratch("handoff");
    let committed = commit_bytes(
        &dir,
        &[
            (SOURCE_PATH, &blob(&repo, &rev, SOURCE_PATH)),
            (HANDOFF_PATH, stripped.as_bytes()),
            (SLOT_PATH, &slot),
        ],
    );
    match observe_neutral_input(SUBJECT, &coords(&dir, &committed)) {
        Err(error) => {
            assert_eq!(error.code, "bad-record");
            assert_eq!(error.detail, "slot-digest");
        }
        Ok(_) => panic!("slot bytes without the handoff digest admitted claim 4"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

fn owner_repo() -> Option<PathBuf> {
    let Ok(repo) = std::env::var("NV12_OWNER_REPOSITORY") else {
        if std::env::var_os("NV12_OWNER_REQUIRED").is_some() {
            panic!("NV12_OWNER_REPOSITORY is required");
        }
        return None;
    };
    if repo.is_empty() {
        panic!("NV12_OWNER_REPOSITORY is empty");
    }
    Some(PathBuf::from(repo))
}

fn coords(dir: &Path, rev: &str) -> super::super::selection::SelectedSource {
    super::super::selection::SelectedSource {
        repository: dir.to_owned(),
        commit: rev.to_owned(),
        source: SOURCE_PATH.to_owned(),
        selection: HANDOFF_PATH.to_owned(),
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nv12-neutral-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch");
    dir
}

fn commit_files(dir: &Path, files: &[(&str, &str)]) -> String {
    let bytes = files
        .iter()
        .map(|(path, body)| (*path, body.as_bytes()))
        .collect::<Vec<_>>();
    commit_bytes(dir, &bytes)
}

fn commit_bytes(dir: &Path, files: &[(&str, &[u8])]) -> String {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "nv12@example.com"]);
    git(dir, &["config", "user.name", "nv12"]);
    for (path, body) in files {
        let full = dir.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(full, body).expect("write");
        git(dir, &["add", path]);
    }
    git(dir, &["commit", "-q", "-m", "fixture"]);
    head(dir)
}

fn blob(repo: &Path, rev: &str, path: &str) -> Vec<u8> {
    let output = Command::new("git")
        .args([
            "--no-replace-objects",
            "-C",
            repo.to_str().expect("repo"),
            "show",
            &format!("{rev}:{path}"),
        ])
        .output()
        .expect("git show");
    assert!(output.status.success(), "git show {path}");
    output.stdout
}

fn head(dir: &Path) -> String {
    let output = Command::new("git")
        .args([
            "--no-replace-objects",
            "-C",
            dir.to_str().expect("repo"),
            "rev-parse",
            "HEAD",
        ])
        .output()
        .expect("rev-parse");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .trim()
        .to_owned()
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?}");
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
