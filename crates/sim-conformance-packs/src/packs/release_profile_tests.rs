// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_v02::CheckScopeId;
use sim_lib_world::completion::{SuppliedOwnerClaim, final_catalog, supply_owner_claim};

use super::{SCOPE, bind_owner_profile, observe_owner_profile};
use crate::{all_packs, find_pack};

const SUBJECT: &str = "subject/owner-profile";
const OBLIGATIONS: [&str; 6] = [
    "validation",
    "docs",
    "integration",
    "package",
    "generated",
    "standalone",
];

#[test]
fn the_scope_stays_on_the_release_pack() {
    let spec = find_pack("checker/c-release").expect("release");
    assert!(spec.implemented_scopes.contains(&SCOPE));
    assert!(
        spec.implemented_scopes
            .contains(&"release/owner-validation")
    );
    assert_eq!(all_packs().len(), 21);
    assert_eq!(
        spec.activation_binding,
        "core/sha256-datum-v1:f12e44d0825b1b54bec8ddfab96007eccc97b9dbcf99ac8ddfe6cb34fc720cf3"
    );
}

#[test]
fn portable_commands_admit_claim_20() {
    let dir = scratch("pass");
    let commands = marks(&dir);
    let manifest = write_manifest(&dir, "portable", &commands, "");
    let observed = observe_owner_profile(SUBJECT, &manifest, &["alpha"]).expect("run");
    for name in OBLIGATIONS {
        assert!(
            dir.join(format!("{name}.mark")).is_file(),
            "{name} did not run"
        );
    }
    assert_eq!(observed.result, "completion/claim-20");
    assert_eq!(observed.repos, vec!["alpha".to_owned()]);
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text(SCOPE).expect("scope")
    );
    bind_owner_profile(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "owner-validation".to_owned();
    assert_eq!(
        bind_owner_profile(&relabelled).expect_err("relabel").code,
        "claim-relabelled"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_receipt_supplies_claim_20_and_leaves_the_seal_unclaimed() {
    let dir = scratch("supply");
    let commands = marks(&dir);
    let manifest = write_manifest(&dir, "portable", &commands, "");
    let before = final_catalog("").expect("catalog");
    let subject = field(&before, "subject").expect("subject");
    assert!(
        !claims(&before).iter().any(|claim| claim == "20"),
        "claim 20 is not proved before the profile runs"
    );
    let observed = observe_owner_profile(&subject, &manifest, &["alpha"]).expect("run");
    assert_eq!(observed.subject, subject);
    let mut revocations = BTreeMap::new();
    revocations.insert(observed.checker.clone(), observed.revocation);
    supply_owner_claim(SuppliedOwnerClaim {
        subject: observed.subject,
        claim: 20,
        admitted: observed.admitted,
        checker: observed.checker,
        catalog: observed.catalog,
        scope: observed.scope,
        revocations,
    });
    let after = final_catalog("").expect("after");
    assert_eq!(claims(&after), vec!["20".to_owned()]);
    assert_eq!(field(&after, "subject").as_deref(), Some(subject.as_str()));
    assert_eq!(field(&after, "seal").as_deref(), Some("unclaimed"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_focused_selection_does_not_run() {
    let dir = scratch("focused");
    let commands = marks(&dir);
    let manifest = write_manifest(&dir, "portable", &commands, " --lib");
    match observe_owner_profile(SUBJECT, &manifest, &["alpha"]) {
        Err(error) => assert_eq!(error.code, "focused-profile"),
        Ok(_) => panic!("focused selection admitted a receipt"),
    }
    for name in OBLIGATIONS {
        assert!(!dir.join(format!("{name}.mark")).exists());
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn linux_does_not_run() {
    let dir = scratch("linux");
    let commands = marks(&dir);
    let manifest = write_manifest(&dir, "linux", &commands, "");
    match observe_owner_profile(SUBJECT, &manifest, &["alpha"]) {
        Err(error) => assert_eq!(error.code, "linux-deferred"),
        Ok(_) => panic!("linux profile admitted a receipt"),
    }
    for name in OBLIGATIONS {
        assert!(!dir.join(format!("{name}.mark")).exists());
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_failing_command_produces_no_receipt() {
    let dir = scratch("fail");
    let mut commands = marks(&dir);
    commands[0] = script(&dir, "validation.sh", "exit 1");
    let manifest = write_manifest(&dir, "portable", &commands, "");
    match observe_owner_profile(SUBJECT, &manifest, &["alpha"]) {
        Err(error) => assert_eq!(error.code, "process-failed"),
        Ok(_) => panic!("failing command admitted a receipt"),
    }
    assert!(!dir.join("docs.mark").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_claim_label_is_refused() {
    match observe_owner_profile("completion/claim-20", Path::new("unused"), &["alpha"]) {
        Err(error) => assert_eq!(error.code, "bad-record"),
        Ok(_) => panic!("claim label admitted a receipt"),
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("nv12-owner-profile-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch");
    dir
}

fn marks(dir: &Path) -> [PathBuf; 6] {
    OBLIGATIONS.map(|name| {
        let mark = dir.join(format!("{name}.mark"));
        script(
            dir,
            &format!("{name}.sh"),
            &format!("touch {}", mark.display()),
        )
    })
}

fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("script");
    let mut permissions = std::fs::metadata(&path).expect("meta").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("mode");
    path
}

fn write_manifest(
    dir: &Path,
    profile: &str,
    commands: &[PathBuf; 6],
    validation_suffix: &str,
) -> PathBuf {
    let path = dir.join("repos.toml");
    let keys = [
        "validation_command",
        "docs_command",
        "integration_command",
        "package_command",
        "generated_output_command",
        "standalone_ci_command",
    ];
    let mut text =
        format!("[[repo]]\nname = \"alpha\"\nlocal_path = \".\"\nowner_profile = \"{profile}\"\n");
    for (key, command) in keys.iter().zip(commands) {
        let suffix = if *key == "validation_command" {
            validation_suffix
        } else {
            ""
        };
        text.push_str(&format!("{key} = \"{}{suffix}\"\n", command.display()));
    }
    std::fs::write(&path, text).expect("manifest");
    path
}

fn claims(datum: &sim_kernel::Datum) -> Vec<String> {
    use sim_kernel::Datum;
    let Datum::Node { fields, .. } = datum else {
        return Vec::new();
    };
    let Some(Datum::Vector(items)) = fields
        .iter()
        .find(|(symbol, _)| symbol.as_qualified_str() == "admissions")
        .map(|(_, value)| value)
    else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| match item {
            Datum::String(text) => text.clone(),
            _ => String::new(),
        })
        .collect()
}

fn field(datum: &sim_kernel::Datum, name: &str) -> Option<String> {
    use sim_kernel::Datum;
    let Datum::Node { fields, .. } = datum else {
        return None;
    };
    fields.iter().find_map(|(symbol, value)| {
        (symbol.as_qualified_str() == name).then(|| match value {
            Datum::String(text) => text.clone(),
            _ => String::new(),
        })
    })
}
