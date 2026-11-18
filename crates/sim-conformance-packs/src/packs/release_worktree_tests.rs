// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::process::Command;

use sim_conformance_v02::CheckScopeId;

use super::{GitPackage, bind_git_release, observe_git_release};

const SUBJECT: &str = "subject/git-worktree";

#[test]
fn git_show_names_the_committed_package() {
    let dir = scratch("committed");
    let rev = commit_package(&dir, "sim-example", false);
    let observed = observe_git_release(SUBJECT, &[package(&dir, &rev, "Cargo.toml")]).expect("git");
    let expected = format!("git+{rev}:sim-example@0.1.0");
    assert_eq!(observed.identities, vec![expected.clone()]);
    assert_eq!(observed.result, format!("completion/claim-21 {expected}"));
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/git-worktree").expect("scope")
    );
    bind_git_release(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-20".to_owned();
    assert_eq!(
        bind_git_release(&relabelled).expect_err("relabel").code,
        "claim-relabelled"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_dirty_worktree_does_not_change_the_commit_identity() {
    let dir = scratch("dirty");
    let rev = commit_package(&dir, "sim-example", false);
    std::fs::write(dir.join("Cargo.toml"), package_toml("sim-other", false)).expect("dirty");
    let observed = observe_git_release(SUBJECT, &[package(&dir, &rev, "Cargo.toml")]).expect("git");
    assert_eq!(
        observed.identities,
        vec![format!("git+{rev}:sim-example@0.1.0")]
    );
    assert!(!observed.result.contains("sim-other"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_published_manifest_is_not_claim_21() {
    let dir = scratch("published");
    let rev = commit_package(&dir, "sim-example", true);
    match observe_git_release(SUBJECT, &[package(&dir, &rev, "Cargo.toml")]) {
        Err(error) => assert_eq!(error.code, "local-publication-claim"),
        Ok(_) => panic!("published manifest admitted a receipt"),
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn two_commits_are_sorted_from_their_objects() {
    let left = scratch("left");
    let right = scratch("right");
    let left_rev = commit_package(&left, "sim-left", false);
    let right_rev = commit_package(&right, "sim-right", false);
    let observed = observe_git_release(
        SUBJECT,
        &[
            package(&right, &right_rev, "Cargo.toml"),
            package(&left, &left_rev, "Cargo.toml"),
        ],
    )
    .expect("git");
    let mut expected = vec![
        format!("git+{left_rev}:sim-left@0.1.0"),
        format!("git+{right_rev}:sim-right@0.1.0"),
    ];
    expected.sort();
    assert_eq!(observed.identities, expected);
    assert!(observed.result.starts_with("completion/claim-21 "));
    assert!(!observed.result.contains("source-identity"));
    std::fs::remove_dir_all(left).ok();
    std::fs::remove_dir_all(right).ok();
}

fn package(dir: &Path, rev: &str, manifest: &str) -> GitPackage {
    GitPackage {
        repository: dir.to_path_buf(),
        commit: rev.to_owned(),
        manifest: manifest.to_owned(),
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nv12-git-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch");
    dir
}

fn commit_package(dir: &Path, name: &str, publish: bool) -> String {
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "nv12@example.com"]);
    git(dir, &["config", "user.name", "nv12"]);
    std::fs::write(dir.join("Cargo.toml"), package_toml(name, publish)).expect("manifest");
    git(dir, &["add", "Cargo.toml"]);
    git(dir, &["commit", "-m", "package"]);
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("rev-parse");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("utf8")
        .trim()
        .to_owned()
}

fn package_toml(name: &str, publish: bool) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = {publish}\n"
    )
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git");
    assert!(status.success(), "{args:?}");
}
