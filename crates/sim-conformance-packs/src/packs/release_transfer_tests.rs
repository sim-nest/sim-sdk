// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::process::Command;

use sim_conformance_v02::CheckScopeId;

use super::{TransferSource, bind_authority_transfer, observe_authority_transfer};

const SUBJECT: &str = "subject/authority-transfer";
const RECORD: &str = "scope portable\nnative-credential false\ncaller-zero true\n";

#[test]
fn authority_transfer_reads_the_record_and_scans_the_directory() {
    let dir = scratch("pass");
    let provider = dir.join("provider");
    std::fs::create_dir(&provider).expect("provider");
    let rev = commit(&dir, RECORD);
    let observed =
        observe_authority_transfer(SUBJECT, &coords(&dir, &rev, &provider)).expect("read");
    assert_eq!(observed.result, "authority-transfer");
    assert_eq!(observed.retained_role, None);
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/authority-transfer").expect("scope")
    );
    bind_authority_transfer(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-22".to_owned();
    assert_eq!(
        bind_authority_transfer(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn authority_transfer_refuses_a_credential_file() {
    let dir = scratch("credential");
    let provider = dir.join("provider");
    std::fs::create_dir(&provider).expect("provider");
    std::fs::write(provider.join("credentials.toml"), "token = \"x\"\n").expect("credential");
    let rev = commit(&dir, RECORD);
    match observe_authority_transfer(SUBJECT, &coords(&dir, &rev, &provider)) {
        Err(error) => assert_eq!(error.code, "provider-credential"),
        Ok(_) => panic!("credential file admitted a receipt"),
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn authority_transfer_refuses_credential_like_names() {
    for name in ["credentials", "credentials.yaml", "id_rsa"] {
        let dir = scratch(name);
        let provider = dir.join("provider");
        std::fs::create_dir(&provider).expect("provider");
        std::fs::write(provider.join(name), b"secret").expect("credential");
        let rev = commit(&dir, RECORD);
        match observe_authority_transfer(SUBJECT, &coords(&dir, &rev, &provider)) {
            Err(error) => assert_eq!(error.code, "provider-credential"),
            Ok(_) => panic!("{name} admitted a receipt"),
        }
        std::fs::remove_dir_all(dir).ok();
    }
}

#[test]
fn authority_transfer_refuses_a_native_scope() {
    let dir = scratch("scope");
    let provider = dir.join("provider");
    std::fs::create_dir(&provider).expect("provider");
    let rev = commit(
        &dir,
        "scope native\nnative-credential false\ncaller-zero true\n",
    );
    match observe_authority_transfer(SUBJECT, &coords(&dir, &rev, &provider)) {
        Err(error) => assert_eq!(error.code, "transfer-scope"),
        Ok(_) => panic!("native scope admitted a receipt"),
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn authority_transfer_keeps_a_named_role() {
    let dir = scratch("role");
    let provider = dir.join("provider");
    std::fs::create_dir(&provider).expect("provider");
    let rev = commit(
        &dir,
        "scope portable\nnative-credential false\ncaller-zero false\nretained-role ordinary-tool\n",
    );
    let observed =
        observe_authority_transfer(SUBJECT, &coords(&dir, &rev, &provider)).expect("role");
    assert_eq!(observed.retained_role.as_deref(), Some("ordinary-tool"));
    std::fs::remove_dir_all(dir).ok();
}

fn coords(dir: &Path, rev: &str, provider: &Path) -> TransferSource {
    TransferSource {
        repository: dir.to_path_buf(),
        commit: rev.to_owned(),
        record: "transfer.txt".to_owned(),
        provider_config: provider.to_path_buf(),
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nv12-transfer-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch");
    dir
}

fn commit(dir: &Path, record: &str) -> String {
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "nv12@example.com"]);
    git(dir, &["config", "user.name", "nv12"]);
    std::fs::write(dir.join("transfer.txt"), record).expect("record");
    git(dir, &["add", "transfer.txt"]);
    git(dir, &["commit", "-m", "transfer"]);
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
