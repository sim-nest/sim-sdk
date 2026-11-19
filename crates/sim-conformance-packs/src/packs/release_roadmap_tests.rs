// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, RevocationStatus};

use super::super::selection::{SelectedSource, observe_source_selection};
use super::{BEGIN, HUMAN, bind_roadmap_authority, observe_roadmap_authority};

const SUBJECT: &str = "subject/roadmap-authority";

#[test]
fn issued_acts_admit_claim_1_for_the_committed_source() {
    let dir = scratch("pass");
    let source = source_text(18, None);
    let rev = commit(&dir, &source, &acts());
    let observed = observe_roadmap_authority(SUBJECT, &coords(&dir, &rev)).expect("read");
    assert_eq!(observed.result, "completion/claim-1");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.round, 18);
    assert_eq!(observed.source_digest, digest(&source));
    assert_eq!(observed.human_selection, HUMAN);
    assert_eq!(observed.begin_authority, BEGIN);
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/roadmap").expect("scope")
    );
    assert_eq!(observed.revocation, RevocationStatus::Current);
    bind_roadmap_authority(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "source-selection".to_owned();
    assert_eq!(
        bind_roadmap_authority(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
    std::fs::write(dir.join("source.md"), source_text(18, Some(17))).expect("dirty");
    let again = observe_roadmap_authority(SUBJECT, &coords(&dir, &rev)).expect("committed");
    assert_eq!(again.source_digest, digest(&source));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn equal_acts_supply_nothing() {
    let dir = scratch("equal");
    let rev = commit(&dir, &source_text(18, None), &format!("{HUMAN}\n{HUMAN}\n"));
    match observe_roadmap_authority(SUBJECT, &coords(&dir, &rev)) {
        Err(error) => assert_eq!(error.code, "selection-begin"),
        Ok(_) => panic!("equal acts admitted a receipt"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_different_begin_sentence_supplies_nothing() {
    let dir = scratch("begin");
    let rev = commit(
        &dir,
        &source_text(18, None),
        &format!("{HUMAN}\nThis starts a different worker.\n"),
    );
    match observe_roadmap_authority(SUBJECT, &coords(&dir, &rev)) {
        Err(error) => {
            assert_eq!(error.code, "bad-record");
            assert_eq!(error.detail, "begin-authority");
        }
        Ok(_) => panic!("a different begin sentence admitted a receipt"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_sibling_round_supplies_nothing() {
    let dir = scratch("sibling");
    let rev = commit(&dir, &source_text(18, Some(17)), &acts());
    match observe_roadmap_authority(SUBJECT, &coords(&dir, &rev)) {
        Err(error) => assert_eq!(error.code, "normative-sibling"),
        Ok(_) => panic!("sibling round admitted a receipt"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn another_round_supplies_nothing() {
    let dir = scratch("round");
    let rev = commit(&dir, &source_text(17, None), &acts());
    match observe_roadmap_authority(SUBJECT, &coords(&dir, &rev)) {
        Err(error) => {
            assert_eq!(error.code, "bad-record");
            assert_eq!(error.detail, "round");
        }
        Ok(_) => panic!("round 17 admitted a receipt"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn prose_stays_off_source_selection_and_tokens_stay_off_claim_1() {
    let dir = scratch("split");
    let source = source_text(18, None);
    let prose = commit(&dir, &source, &acts());
    match observe_source_selection(SUBJECT, &coords(&dir, &prose)) {
        Err(error) => assert_eq!(error.code, "bad-record"),
        Ok(_) => panic!("prose acts admitted source-selection"),
    }
    std::fs::write(
        dir.join("selection.txt"),
        "human-selection maintainer\nbegin-authority begin-nv12\n",
    )
    .expect("tokens");
    git(&dir, &["add", "selection.txt"]);
    git(&dir, &["commit", "-m", "tokens"]);
    let tokens = head(&dir);
    let selected = observe_source_selection(SUBJECT, &coords(&dir, &tokens)).expect("tokens");
    assert_eq!(selected.result, "source-selection");
    assert_ne!(selected.result, "completion/claim-1");
    match observe_roadmap_authority(SUBJECT, &coords(&dir, &tokens)) {
        Err(error) => assert_eq!(error.code, "bad-record"),
        Ok(_) => panic!("token records admitted claim 1"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_replacement_ref_does_not_change_the_admitted_blob() {
    let dir = scratch("replace");
    let original = commit(&dir, &source_text(18, None), &acts());
    std::fs::write(dir.join("source.md"), source_text(18, Some(17))).expect("sibling");
    git(&dir, &["add", "source.md"]);
    git(&dir, &["commit", "-m", "sibling"]);
    let replacement = head(&dir);
    git(&dir, &["replace", &original, &replacement]);
    let observed = observe_roadmap_authority(SUBJECT, &coords(&dir, &original)).expect("original");
    assert_eq!(observed.round, 18);
    assert_eq!(observed.result, "completion/claim-1");
    assert_eq!(observed.source_digest, digest(&source_text(18, None)));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_roadmap_scope_is_implemented() {
    let spec = crate::find_pack("checker/c-release").expect("release");
    assert!(spec.allowed_scopes.contains(&"completion/roadmap"));
    assert!(spec.implemented_scopes.contains(&"completion/roadmap"));
    assert!(spec.allowed_scopes.contains(&"release/source-selection"));
}

fn acts() -> String {
    format!("{HUMAN}\n{BEGIN}\n")
}

fn coords(dir: &Path, rev: &str) -> SelectedSource {
    SelectedSource {
        repository: dir.to_path_buf(),
        commit: rev.to_owned(),
        source: "source.md".to_owned(),
        selection: "selection.txt".to_owned(),
    }
}

fn source_text(round: u32, sibling: Option<u32>) -> String {
    let mut text = format!("# NO_V3_{round}: example\n");
    for claim in 1..=25 {
        text.push_str(&format!("{claim}. claim {claim}\n"));
    }
    if let Some(other) = sibling {
        text.push_str(&format!("See NO_V3_{other} for the requirement.\n"));
    }
    text
}

fn digest(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nv12-roadmap-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch");
    dir
}

fn commit(dir: &Path, source: &str, selection: &str) -> String {
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "nv12@example.com"]);
    git(dir, &["config", "user.name", "nv12"]);
    std::fs::write(dir.join("source.md"), source).expect("source");
    std::fs::write(dir.join("selection.txt"), selection).expect("selection");
    git(dir, &["add", "source.md", "selection.txt"]);
    git(dir, &["commit", "-m", "selection"]);
    head(dir)
}

fn head(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["--no-replace-objects", "rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("rev-parse");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("utf8")
        .trim()
        .to_owned()
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git");
    assert!(status.success(), "{args:?}");
}
