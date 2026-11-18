// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_v02::CheckScopeId;

use super::{bind_retirement_guard, observe_retirement_guard};

const SUBJECT: &str = "subject/retirement-guard";

#[test]
fn retirement_guard_runs_the_script_in_the_root() {
    let dir = scratch("pass");
    let mark = dir.join("ran.mark");
    write_script(&dir, &format!("touch {}\n", mark.display()));
    let observed = observe_retirement_guard(SUBJECT, &dir).expect("run");
    assert!(mark.is_file(), "retirement script did not run");
    assert_eq!(observed.result, "retirement-guard");
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/retirement-guard").expect("scope")
    );
    bind_retirement_guard(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-3".to_owned();
    assert_eq!(
        bind_retirement_guard(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn retirement_guard_refuses_a_non_zero_script() {
    let dir = scratch("fail");
    let mark = dir.join("ran.mark");
    write_script(&dir, &format!("touch {}\nexit 1\n", mark.display()));
    match observe_retirement_guard(SUBJECT, &dir) {
        Err(error) => assert_eq!(error.code, "process-failed"),
        Ok(_) => panic!("failing script admitted a receipt"),
    }
    assert!(mark.is_file(), "the failing script did not run");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn retirement_guard_refuses_a_missing_script() {
    let dir = scratch("missing");
    match observe_retirement_guard(SUBJECT, &dir) {
        Err(error) => assert_eq!(error.code, "missing-source"),
        Ok(_) => panic!("missing script admitted a receipt"),
    }
    std::fs::remove_dir_all(dir).ok();
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nv12-retirement-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("scripts")).expect("scratch");
    dir
}

fn write_script(dir: &Path, body: &str) {
    let path = dir.join("scripts/check-v3-retired.sh");
    std::fs::write(&path, format!("#!/bin/sh\n{body}")).expect("script");
    let mut permissions = std::fs::metadata(&path).expect("meta").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("mode");
}
