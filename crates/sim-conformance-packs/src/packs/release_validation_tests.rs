// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_v02::CheckScopeId;

use super::{bind_owner_validation, observe_owner_validation};

const SUBJECT: &str = "subject/owner-validation";

#[test]
fn declared_commands_run_and_a_zero_exit_admits_the_receipt() {
    let dir = scratch("pass");
    let validation_mark = dir.join("validation.mark");
    let docs_mark = dir.join("docs.mark");
    let validation = script(
        &dir,
        "validation.sh",
        &format!("touch {}", validation_mark.display()),
    );
    let docs = script(&dir, "docs.sh", &format!("touch {}", docs_mark.display()));
    let manifest = write_manifest(&dir, &validation, &docs);
    let observed = observe_owner_validation(SUBJECT, &manifest, &["alpha"]).expect("run");
    assert!(validation_mark.is_file(), "validation command did not run");
    assert!(docs_mark.is_file(), "docs command did not run");
    assert_eq!(observed.result, "owner-validation");
    assert_eq!(observed.repos, vec!["alpha".to_owned()]);
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/owner-validation").expect("scope")
    );
    bind_owner_validation(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-20".to_owned();
    assert_eq!(
        bind_owner_validation(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_non_zero_command_produces_no_receipt() {
    let dir = scratch("fail");
    let mark = dir.join("ran.mark");
    let validation = script(
        &dir,
        "validation.sh",
        &format!("touch {}\nexit 1", mark.display()),
    );
    let docs = script(&dir, "docs.sh", "exit 0");
    let manifest = write_manifest(&dir, &validation, &docs);
    match observe_owner_validation(SUBJECT, &manifest, &["alpha"]) {
        Err(error) => assert_eq!(error.code, "process-failed"),
        Ok(_) => panic!("non-zero command admitted a receipt"),
    }
    assert!(mark.is_file(), "the failing command did not run");
    assert!(!dir.join("docs.mark").exists());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn an_unknown_repository_is_not_a_passing_command() {
    let dir = scratch("missing");
    let validation = script(&dir, "validation.sh", "exit 0");
    let docs = script(&dir, "docs.sh", "exit 0");
    let manifest = write_manifest(&dir, &validation, &docs);
    match observe_owner_validation(SUBJECT, &manifest, &["missing"]) {
        Err(error) => assert_eq!(error.code, "missing-source"),
        Ok(_) => panic!("unknown repository admitted a receipt"),
    }
    std::fs::remove_dir_all(dir).ok();
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nv12-validation-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch");
    dir
}

fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("script");
    let mut permissions = std::fs::metadata(&path).expect("meta").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("mode");
    path
}

fn write_manifest(dir: &Path, validation: &Path, docs: &Path) -> PathBuf {
    let path = dir.join("repos.toml");
    let text = format!(
        "[[repo]]\nname = \"alpha\"\nlocal_path = \".\"\nvalidation_command = \"{}\"\ndocs_command = \"{}\"\n",
        validation.display(),
        docs.display()
    );
    std::fs::write(&path, text).expect("manifest");
    path
}
