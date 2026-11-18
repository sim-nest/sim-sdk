// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_v02::CheckScopeId;

use super::{bind_boot_guard, observe_boot_guard};

const SUBJECT: &str = "subject/boot-guard";

#[test]
fn boot_guard_runs_the_script_in_the_root() {
    let dir = scratch("pass");
    let mark = dir.join("ran.mark");
    write_script(&dir, &format!("touch {}\n", mark.display()));
    let observed = observe_boot_guard(SUBJECT, &dir).expect("run");
    assert!(mark.is_file(), "boot script did not run");
    assert_eq!(observed.result, "boot-guard");
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/boot-guard").expect("scope")
    );
    bind_boot_guard(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-18".to_owned();
    assert_eq!(
        bind_boot_guard(&relabelled).expect_err("relabel").code,
        "claim-relabelled"
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn boot_guard_refuses_a_non_zero_script() {
    let dir = scratch("fail");
    let mark = dir.join("ran.mark");
    write_script(&dir, &format!("touch {}\nexit 1\n", mark.display()));
    match observe_boot_guard(SUBJECT, &dir) {
        Err(error) => assert_eq!(error.code, "process-failed"),
        Ok(_) => panic!("failing script admitted a receipt"),
    }
    assert!(mark.is_file(), "the failing script did not run");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn boot_guard_uses_the_root_as_its_working_directory() {
    let dir = scratch("cwd");
    std::fs::write(dir.join("marker"), b"in-root").expect("marker");
    write_script(&dir, "test -f marker\n");
    observe_boot_guard(SUBJECT, &dir).expect("root marker");

    let outside = std::env::current_dir()
        .expect("cwd")
        .join(format!(".nv12-boot-marker-{}", std::process::id()));
    let _ = std::fs::remove_file(&outside);
    std::fs::write(&outside, b"outside").expect("outside");
    let name = outside.file_name().expect("name").to_string_lossy();
    write_script(&dir, &format!("test -f {name}\n"));
    let result = observe_boot_guard(SUBJECT, &dir);
    let _ = std::fs::remove_file(&outside);
    match result {
        Err(error) => assert_eq!(error.code, "process-failed"),
        Ok(_) => panic!("a file outside the root satisfied the guard"),
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn boot_guard_refuses_a_missing_script() {
    let dir = scratch("missing");
    match observe_boot_guard(SUBJECT, &dir) {
        Err(error) => assert_eq!(error.code, "missing-source"),
        Ok(_) => panic!("missing script admitted a receipt"),
    }
    std::fs::remove_dir_all(dir).ok();
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nv12-boot-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("scripts")).expect("scratch");
    dir
}

fn write_script(dir: &Path, body: &str) {
    let path = dir.join("scripts/check-bin-boot.sh");
    std::fs::write(&path, format!("#!/bin/sh\n{body}")).expect("script");
    let mut permissions = std::fs::metadata(&path).expect("meta").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("mode");
}
