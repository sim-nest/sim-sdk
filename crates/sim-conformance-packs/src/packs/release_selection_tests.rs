// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use sim_conformance_v02::CheckScopeId;

use super::{SelectedSource, bind_source_selection, observe_source_selection};

const SUBJECT: &str = "subject/source-selection";

#[test]
fn source_selection_reads_the_committed_blobs() {
    let dir = scratch("pass");
    let source = source_text(18, None);
    let rev = commit(
        &dir,
        &source,
        "human-selection maintainer\nbegin-authority begin-nv12\n",
    );
    let observed = observe_source_selection(SUBJECT, &coords(&dir, &rev)).expect("read");
    let digest = Sha256::digest(source.as_bytes());
    let digest = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(observed.result, "source-selection");
    assert_eq!(observed.source_digest, digest);
    assert_eq!(observed.round, 18);
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/source-selection").expect("scope")
    );
    bind_source_selection(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-1".to_owned();
    assert_eq!(
        bind_source_selection(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
    std::fs::write(dir.join("source.md"), "dirty").expect("dirty");
    let again = observe_source_selection(SUBJECT, &coords(&dir, &rev)).expect("committed");
    assert_eq!(again.source_digest, digest);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn source_selection_refuses_equal_authorities() {
    let dir = scratch("equal");
    let rev = commit(
        &dir,
        &source_text(18, None),
        "human-selection same\nbegin-authority same\n",
    );
    match observe_source_selection(SUBJECT, &coords(&dir, &rev)) {
        Err(error) => assert_eq!(error.code, "selection-begin"),
        Ok(_) => panic!("equal authorities admitted a receipt"),
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn source_selection_refuses_a_sibling_round() {
    let dir = scratch("sibling");
    let rev = commit(
        &dir,
        &source_text(18, Some(17)),
        "human-selection maintainer\nbegin-authority begin-nv12\n",
    );
    match observe_source_selection(SUBJECT, &coords(&dir, &rev)) {
        Err(error) => assert_eq!(error.code, "normative-sibling"),
        Ok(_) => panic!("sibling round admitted a receipt"),
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn source_selection_ignores_a_replacement_ref() {
    let dir = scratch("replace");
    let original = commit(
        &dir,
        &source_text(18, None),
        "human-selection maintainer\nbegin-authority begin-nv12\n",
    );
    std::fs::write(dir.join("source.md"), source_text(18, Some(17))).expect("sibling");
    git(&dir, &["add", "source.md"]);
    git(&dir, &["commit", "-m", "sibling"]);
    let replacement = head(&dir);
    git(&dir, &["replace", &original, &replacement]);
    let observed = observe_source_selection(SUBJECT, &coords(&dir, &original)).expect("original");
    assert_eq!(observed.round, 18);
    assert_eq!(observed.result, "source-selection");
    std::fs::remove_dir_all(dir).ok();
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

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nv12-selection-{name}-{}", std::process::id()));
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
