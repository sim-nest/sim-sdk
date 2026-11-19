// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, RevocationStatus};

use super::super::selection::{SelectedSource, observe_source_selection};
use super::{
    BEGIN, HUMAN, SOURCE_DIGEST, SOURCE_PATH, act_path, bind_roadmap_authority,
    observe_roadmap_authority,
};

const SUBJECT: &str = "subject/roadmap-authority";
const RUN_PATH: &str = super::RUN_PATH;

#[test]
fn copied_sentences_and_an_example_source_supply_nothing() {
    let dir = scratch("copied");
    let source = source_text(18, None);
    let rev = commit_files(
        &dir,
        &[
            ("source.md", &source),
            ("selection.txt", &format!("{HUMAN}\n{BEGIN}\n")),
        ],
    );
    match observe_roadmap_authority(SUBJECT, &coords(&dir, &rev, "source.md", "selection.txt")) {
        Err(error) => assert_eq!(error.code, "bad-record"),
        Ok(_) => panic!("copied sentences admitted claim 1"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_example_at_the_owner_paths_supplies_nothing() {
    let dir = scratch("example");
    let source = source_text(18, None);
    let digest = digest(&source);
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, &source),
            (act_path(HUMAN).expect("path"), &handoff(&digest)),
            (RUN_PATH, &format!("{BEGIN}\n")),
        ],
    );
    match observe_roadmap_authority(SUBJECT, &owner_coords(&dir, &rev)) {
        Err(error) => {
            assert_eq!(error.code, "bad-record");
            assert_eq!(error.detail, "source-digest");
        }
        Ok(_) => panic!("an example source admitted claim 1"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn equal_acts_supply_nothing() {
    let dir = scratch("equal");
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, &source_text(18, None)),
            (act_path(HUMAN).expect("path"), &handoff(SOURCE_DIGEST)),
            (RUN_PATH, &format!("{HUMAN}\n")),
        ],
    );
    match observe_roadmap_authority(SUBJECT, &owner_coords(&dir, &rev)) {
        Err(error) => assert_eq!(error.code, "selection-begin"),
        Ok(_) => panic!("equal acts admitted a receipt"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_substituted_run_record_supplies_nothing() {
    let dir = scratch("run");
    let other = "docs/workbench/NV12_RUN_deadbeef.md";
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, &source_text(18, None)),
            (
                act_path(HUMAN).expect("path"),
                &handoff_for(SOURCE_DIGEST, other),
            ),
            (other, &format!("{BEGIN}\n")),
        ],
    );
    match observe_roadmap_authority(SUBJECT, &owner_coords(&dir, &rev)) {
        Err(error) => {
            assert_eq!(error.code, "bad-record");
            assert_eq!(error.detail, "begin-authority");
        }
        Ok(_) => panic!("a substituted run record admitted claim 1"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_different_begin_sentence_supplies_nothing() {
    let dir = scratch("begin");
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, &source_text(18, None)),
            (act_path(HUMAN).expect("path"), &handoff(SOURCE_DIGEST)),
            (RUN_PATH, "This starts a different worker.\n"),
        ],
    );
    match observe_roadmap_authority(SUBJECT, &owner_coords(&dir, &rev)) {
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
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, &source_text(18, Some(17))),
            (act_path(HUMAN).expect("path"), &handoff(SOURCE_DIGEST)),
            (RUN_PATH, &format!("{BEGIN}\n")),
        ],
    );
    match observe_roadmap_authority(SUBJECT, &owner_coords(&dir, &rev)) {
        Err(error) => assert_eq!(error.code, "normative-sibling"),
        Ok(_) => panic!("sibling round admitted a receipt"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn another_round_supplies_nothing() {
    let dir = scratch("round");
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, &source_text(17, None)),
            (act_path(HUMAN).expect("path"), &handoff(SOURCE_DIGEST)),
            (RUN_PATH, &format!("{BEGIN}\n")),
        ],
    );
    match observe_roadmap_authority(SUBJECT, &owner_coords(&dir, &rev)) {
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
    let prose = commit_files(
        &dir,
        &[
            ("source.md", &source),
            ("selection.txt", &format!("{HUMAN}\n{BEGIN}\n")),
        ],
    );
    match observe_source_selection(SUBJECT, &coords(&dir, &prose, "source.md", "selection.txt")) {
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
    let selected = observe_source_selection(
        SUBJECT,
        &coords(&dir, &tokens, "source.md", "selection.txt"),
    )
    .expect("tokens");
    assert_eq!(selected.result, "source-selection");
    assert_ne!(selected.result, "completion/claim-1");
    match observe_roadmap_authority(
        SUBJECT,
        &coords(&dir, &tokens, "source.md", "selection.txt"),
    ) {
        Err(error) => assert_eq!(error.code, "bad-record"),
        Ok(_) => panic!("token records admitted claim 1"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_replacement_ref_does_not_admit_an_example() {
    let dir = scratch("replace");
    let original = commit_files(
        &dir,
        &[
            (SOURCE_PATH, &source_text(18, None)),
            (act_path(HUMAN).expect("path"), &handoff(SOURCE_DIGEST)),
            (RUN_PATH, &format!("{BEGIN}\n")),
        ],
    );
    std::fs::write(dir.join(SOURCE_PATH), source_text(18, Some(17))).expect("sibling");
    git(&dir, &["add", SOURCE_PATH]);
    git(&dir, &["commit", "-m", "sibling"]);
    let replacement = head(&dir);
    git(&dir, &["replace", &original, &replacement]);
    match observe_roadmap_authority(SUBJECT, &owner_coords(&dir, &original)) {
        Err(error) => assert_eq!(error.detail, "source-digest"),
        Ok(_) => panic!("a replaced example admitted claim 1"),
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_roadmap_scope_is_implemented() {
    let spec = crate::find_pack("checker/c-release").expect("release");
    assert!(spec.allowed_scopes.contains(&"completion/roadmap"));
    assert!(spec.implemented_scopes.contains(&"completion/roadmap"));
    assert!(spec.allowed_scopes.contains(&"release/source-selection"));
}

#[test]
fn copied_wording_is_refused_and_owner_records_bind_their_source() {
    let dir = scratch("wording");
    let source = source_text(18, None);
    let rev = commit_files(
        &dir,
        &[
            (SOURCE_PATH, &source),
            ("selection.txt", &format!("{HUMAN}\n{BEGIN}\n")),
            (RUN_PATH, &format!("{BEGIN}\n")),
        ],
    );
    let copied = SelectedSource {
        repository: dir.clone(),
        commit: rev,
        source: SOURCE_PATH.to_owned(),
        selection: "selection.txt".to_owned(),
    };
    match observe_roadmap_authority(SUBJECT, &copied) {
        Err(error) => assert_eq!(error.detail, "human-selection"),
        Ok(_) => panic!("a copied selection file admitted claim 1"),
    }
    std::fs::remove_dir_all(&dir).ok();

    let Ok(repo) = std::env::var("NV12_OWNER_REPOSITORY") else {
        if std::env::var_os("NV12_OWNER_REQUIRED").is_some() {
            panic!("NV12_OWNER_REPOSITORY is required");
        }
        return;
    };
    let owner = PathBuf::from(repo);
    let rev = head(&owner);
    let observed = observe_roadmap_authority(SUBJECT, &owner_coords(&owner, &rev)).expect("owner");
    assert_eq!(observed.result, "completion/claim-1");
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.round, 18);
    assert_eq!(observed.source_digest, SOURCE_DIGEST);
    assert_eq!(observed.human_selection, HUMAN);
    assert_eq!(observed.begin_authority, BEGIN);
    assert_ne!(observed.human_selection, observed.begin_authority);
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
}

fn handoff(stated_digest: &str) -> String {
    handoff_for(stated_digest, RUN_PATH)
}

fn handoff_for(stated_digest: &str, run_path: &str) -> String {
    format!(
        "{HUMAN}\n\n`NO_V3_18.md` remains the selected product contract, under `AGENTS.md`.\n\nIts record is `{run_path}`.\n\nSHA-256 `{stated_digest}`\n"
    )
}

fn owner_coords(dir: &Path, rev: &str) -> SelectedSource {
    SelectedSource {
        repository: dir.to_path_buf(),
        commit: rev.to_owned(),
        source: SOURCE_PATH.to_owned(),
        selection: act_path(HUMAN).expect("path").to_owned(),
    }
}

fn coords(dir: &Path, rev: &str, source: &str, selection: &str) -> SelectedSource {
    SelectedSource {
        repository: dir.to_path_buf(),
        commit: rev.to_owned(),
        source: source.to_owned(),
        selection: selection.to_owned(),
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

fn commit_files(dir: &Path, files: &[(&str, &str)]) -> String {
    git(dir, &["init", "-b", "main"]);
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
    git(dir, &["commit", "-m", "records"]);
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
