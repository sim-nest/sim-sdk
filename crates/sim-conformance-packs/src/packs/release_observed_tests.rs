// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use sim_conformance_v02::CheckScopeId;

use super::{
    PublishedObligation, ReleaseObligations, bind_release_obligations, observe_release_obligations,
};
use crate::packs::release::GitPackage;
use crate::{MemorySubject, PackRequest};

const SUBJECT: &str = "subject/release-obligations";
const TOP: &str = "\
[package]
name = \"sim-top\"
version = \"0.1.0\"
edition = \"2021\"
publish = true

[dependencies]
serde = \"1\"
sim-base = \"0.1.0\"
";

struct Lab {
    root: PathBuf,
    repo: PathBuf,
    mirror: PathBuf,
    mirror_left: PathBuf,
    index: PathBuf,
    krate: PathBuf,
    pin: PathBuf,
    checkout: PathBuf,
    boot: PathBuf,
    mark: PathBuf,
    commit: String,
    cycle: bool,
}

impl Drop for Lab {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Lab {
    fn open(name: &str, cycle: bool) -> Self {
        let root =
            std::env::temp_dir().join(format!("nv12-obligations-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("repo");
        std::fs::create_dir_all(repo.join("sim-base")).expect("repo");
        let mut lab = Self {
            mirror: root.join("mirror.git"),
            mirror_left: repo.clone(),
            index: root.join("index"),
            krate: root.join("sim-top.crate"),
            pin: root.join("repos.toml"),
            checkout: repo.clone(),
            boot: root.join("boot"),
            mark: root.join("boot.mark"),
            commit: String::new(),
            cycle,
            root,
            repo,
        };
        lab.manifests();
        git(&lab.repo, &["init", "-b", "main"]);
        git(&lab.repo, &["config", "user.email", "nv12@example.com"]);
        git(&lab.repo, &["config", "user.name", "nv12"]);
        git(&lab.repo, &["add", "Cargo.toml", "sim-base"]);
        if cycle {
            git(&lab.repo, &["add", "sim-other"]);
        }
        git(&lab.repo, &["commit", "-m", "packages"]);
        let commit = head(&lab.repo);
        git(&lab.repo, &["tag", "sim-top-v0.1.0", &commit]);
        std::fs::write(
            &lab.pin,
            format!("[[repo]]\nname = \"sim-top\"\ncommit = \"{commit}\"\n"),
        )
        .expect("pin");
        pack_crate(&lab.root.join("stage"), &lab.krate, &commit);
        let sum = sha256_file(&lab.krate);
        write_index(&lab.index, &sum, false);
        write_boot(
            &lab.boot,
            &format!("touch {}\nprintf 'booted\\n'\n", lab.mark.display()),
        );
        let cloned = Command::new("git")
            .args([
                "clone",
                "--bare",
                lab.repo.to_str().expect("repo"),
                lab.mirror.to_str().expect("mirror"),
            ])
            .output()
            .expect("clone");
        assert!(
            cloned.status.success(),
            "{}",
            String::from_utf8_lossy(&cloned.stderr)
        );
        lab.commit = commit;
        lab
    }

    fn manifests(&self) {
        std::fs::write(self.repo.join("Cargo.toml"), TOP).expect("top");
        let deps = if self.cycle {
            "\n[dependencies]\nsim-other = \"0.1.0\"\n"
        } else {
            "\n"
        };
        std::fs::write(
            self.repo.join("sim-base/Cargo.toml"),
            format!(
                "[package]\nname = \"sim-base\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n{deps}"
            ),
        )
        .expect("base");
        if self.cycle {
            std::fs::create_dir_all(self.repo.join("sim-other")).expect("other");
            std::fs::write(
                self.repo.join("sim-other/Cargo.toml"),
                "[package]\nname = \"sim-other\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\nsim-base = \"0.1.0\"\n",
            )
            .expect("other manifest");
        }
    }

    fn obligations(&self) -> ReleaseObligations {
        let mut unpublished = vec![package(&self.repo, &self.commit, "sim-base/Cargo.toml")];
        if self.cycle {
            unpublished.push(package(&self.repo, &self.commit, "sim-other/Cargo.toml"));
        }
        ReleaseObligations {
            unpublished,
            published: vec![self.published()],
            boot_program: self.boot.clone(),
            boot_args: Vec::new(),
        }
    }

    fn published(&self) -> PublishedObligation {
        PublishedObligation {
            index: self.index.clone(),
            krate: self.krate.clone(),
            repository: self.repo.clone(),
            pin_manifest: self.pin.clone(),
            checkout: self.checkout.clone(),
            pin_name: "sim-top".to_owned(),
            mirror_left: self.mirror_left.display().to_string(),
            mirror_right: self.mirror.display().to_string(),
        }
    }

    fn point_tag_elsewhere(&self) {
        std::fs::write(self.repo.join("side.txt"), b"side").expect("side");
        git(&self.repo, &["add", "side.txt"]);
        git(&self.repo, &["commit", "-m", "side"]);
        let side = head(&self.repo);
        git(&self.repo, &["reset", "--hard", &self.commit]);
        git(&self.repo, &["tag", "-f", "sim-top-v0.1.0", &side]);
    }

    fn advance_selected(&self) -> String {
        std::fs::write(self.repo.join("later.txt"), b"later").expect("later");
        git(&self.repo, &["add", "later.txt"]);
        git(&self.repo, &["commit", "-m", "later"]);
        let selected = head(&self.repo);
        self.write_pin(&selected);
        git(
            &self.repo,
            &[
                "push",
                self.mirror.to_str().expect("mirror"),
                &format!("{selected}:refs/heads/main"),
            ],
        );
        selected
    }

    fn select_foreign_repository(&mut self) -> String {
        let foreign = self.root.join("foreign");
        std::fs::create_dir_all(&foreign).expect("foreign");
        git(&foreign, &["init", "-b", "main"]);
        git(&foreign, &["config", "user.email", "nv12@example.com"]);
        git(&foreign, &["config", "user.name", "nv12"]);
        std::fs::write(foreign.join("note"), b"foreign").expect("note");
        git(&foreign, &["add", "note"]);
        git(&foreign, &["commit", "-m", "foreign"]);
        let selected = head(&foreign);
        let mirror = self.root.join("foreign.git");
        let cloned = Command::new("git")
            .args([
                "clone",
                "--bare",
                foreign.to_str().expect("foreign"),
                mirror.to_str().expect("mirror"),
            ])
            .output()
            .expect("clone");
        assert!(
            cloned.status.success(),
            "{}",
            String::from_utf8_lossy(&cloned.stderr)
        );
        self.checkout = foreign.clone();
        self.mirror_left = foreign;
        self.mirror = mirror;
        self.write_pin(&selected);
        selected
    }

    fn select_unrelated_revision(&self) -> String {
        git(&self.repo, &["checkout", "--orphan", "unrelated"]);
        std::fs::write(self.repo.join("unrelated.txt"), b"unrelated").expect("unrelated");
        git(&self.repo, &["add", "unrelated.txt"]);
        git(&self.repo, &["commit", "-m", "unrelated"]);
        let selected = head(&self.repo);
        git(&self.repo, &["branch", "-f", "main", "HEAD"]);
        git(&self.repo, &["checkout", "main"]);
        git(&self.repo, &["branch", "-D", "unrelated"]);
        self.write_pin(&selected);
        git(
            &self.repo,
            &[
                "push",
                "--force",
                self.mirror.to_str().expect("mirror"),
                &format!("{selected}:refs/heads/main"),
            ],
        );
        selected
    }

    fn write_pin(&self, commit: &str) {
        std::fs::write(
            &self.pin,
            format!("[[repo]]\nname = \"sim-top\"\ncommit = \"{commit}\"\n"),
        )
        .expect("pin");
    }

    fn advertise_other_main(&self) {
        std::fs::write(self.repo.join("side.txt"), b"side").expect("side");
        git(&self.repo, &["add", "side.txt"]);
        git(&self.repo, &["commit", "-m", "side"]);
        let side = head(&self.repo);
        git(&self.repo, &["reset", "--hard", &self.commit]);
        git(
            &self.repo,
            &[
                "push",
                self.mirror.to_str().expect("mirror"),
                &format!("{side}:refs/heads/main"),
            ],
        );
    }
}

#[test]
fn obligations_read_checksum_tag_mirrors_pin_order_and_boot() {
    let lab = Lab::open("pass", false);
    let observed = observe_release_obligations(SUBJECT, &lab.obligations()).expect("obligations");
    let identity = format!("git+{}:sim-base@0.1.0", lab.commit);
    assert_eq!(observed.result, format!("completion/claim-21 {identity}"));
    assert_eq!(observed.identities, vec![identity]);
    assert!(!observed.result.contains("sim-top"));
    assert_eq!(
        observed.order,
        vec!["sim-base".to_owned(), "sim-top".to_owned()]
    );
    assert_eq!(observed.published.len(), 1);
    assert_eq!(observed.published[0].name, "sim-top");
    assert_eq!(observed.published[0].version, "0.1.0");
    assert_eq!(observed.published[0].commit, lab.commit);
    assert_eq!(observed.published[0].selected, lab.commit);
    assert_eq!(observed.published[0].checksum, sha256_file(&lab.krate));
    assert!(lab.mark.is_file(), "boot program did not run");
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/observed-obligations").expect("scope")
    );
    bind_release_obligations(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-21".to_owned();
    assert_eq!(
        bind_release_obligations(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
}

#[test]
fn obligations_refuse_a_checksum_that_misses_the_index() {
    let lab = Lab::open("checksum", false);
    write_index(&lab.index, &"a".repeat(64), false);
    refuse(&lab.obligations(), "checksum-mismatch");
}

#[test]
fn obligations_refuse_a_yanked_index_line() {
    let lab = Lab::open("yanked", false);
    write_index(&lab.index, &sha256_file(&lab.krate), true);
    refuse(&lab.obligations(), "yanked");
}

#[test]
fn obligations_refuse_a_tag_on_another_commit() {
    let lab = Lab::open("tag", false);
    lab.point_tag_elsewhere();
    refuse(&lab.obligations(), "tag-mismatch");
}

#[test]
fn obligations_refuse_the_same_mirror_twice() {
    let mut lab = Lab::open("mirror-same", false);
    let link = lab.root.join("same-mirror");
    std::os::unix::fs::symlink(&lab.repo, &link).expect("symlink");
    lab.mirror = link;
    refuse(&lab.obligations(), "mirror-same");
}

#[test]
fn obligations_refuse_a_mirror_that_advertises_another_commit() {
    let lab = Lab::open("mirror", false);
    lab.advertise_other_main();
    refuse(&lab.obligations(), "mirror-mismatch");
}

#[test]
fn obligations_accept_a_release_contained_in_a_later_revision() {
    let lab = Lab::open("later", false);
    let selected = lab.advance_selected();
    assert_ne!(selected, lab.commit);
    let observed = observe_release_obligations(SUBJECT, &lab.obligations()).expect("later");
    assert_eq!(observed.published[0].commit, lab.commit);
    assert_eq!(observed.published[0].selected, selected);
    assert_eq!(
        observed.identities,
        vec![format!("git+{}:sim-base@0.1.0", lab.commit)]
    );
    assert!(lab.mark.is_file(), "boot program did not run");
}

#[test]
fn obligations_refuse_a_release_outside_the_selected_revision() {
    let lab = Lab::open("uncontained", false);
    let selected = lab.select_unrelated_revision();
    assert_ne!(selected, lab.commit);
    refuse(&lab.obligations(), "release-uncontained");
    assert!(!lab.mark.exists(), "boot ran for an uncontained release");
}

#[test]
fn obligations_refuse_a_release_absent_from_the_selected_repository() {
    let mut lab = Lab::open("foreign", false);
    let selected = lab.select_foreign_repository();
    assert_ne!(selected, lab.commit);
    refuse(&lab.obligations(), "release-uncontained");
    assert!(!lab.mark.exists(), "boot ran for an uncontained release");
}

#[test]
fn obligations_refuse_a_checkout_off_the_pinned_revision() {
    let lab = Lab::open("pin", false);
    lab.write_pin(&"0".repeat(40));
    refuse(&lab.obligations(), "checkout-mismatch");
}

#[test]
fn obligations_refuse_a_checkout_on_another_commit() {
    let mut lab = Lab::open("checkout", false);
    let other = lab.root.join("other");
    std::fs::create_dir_all(&other).expect("other");
    git(&other, &["init", "-b", "main"]);
    git(&other, &["config", "user.email", "nv12@example.com"]);
    git(&other, &["config", "user.name", "nv12"]);
    std::fs::write(other.join("note"), b"other").expect("note");
    git(&other, &["add", "note"]);
    git(&other, &["commit", "-m", "other"]);
    lab.checkout = other;
    refuse(&lab.obligations(), "checkout-mismatch");
}

#[test]
fn obligations_refuse_a_boot_that_exits_nonzero() {
    let lab = Lab::open("boot", false);
    write_boot(
        &lab.boot,
        &format!("touch {}\nexit 1\n", lab.mark.display()),
    );
    refuse(&lab.obligations(), "process-failed");
    assert!(lab.mark.is_file(), "failing boot did not run");
}

#[test]
fn obligations_refuse_a_dependency_cycle() {
    let lab = Lab::open("cycle", true);
    refuse(&lab.obligations(), "dependency-cycle");
}

#[test]
fn obligations_refuse_git_identities_without_a_published_crate() {
    let lab = Lab::open("git-only", false);
    let mut input = lab.obligations();
    input.published.clear();
    refuse(&input, "missing-source");
    assert!(!lab.mark.exists(), "boot ran without a published crate");
}

#[test]
fn obligations_refuse_claim_text_in_the_fact() {
    for body in ["completion/claim-21", "git+not-an-identity"] {
        let evidence = MemorySubject::default().with("release.obligations", body);
        let request = PackRequest {
            checker: "checker/c-release",
            binding: "binding",
            subject: SUBJECT,
            scope: "release/observed-obligations",
            evidence: &evidence,
        };
        let error = super::observations(&request).expect_err(body);
        assert_eq!(error.code, "claim-relabelled");
    }
}

fn refuse(input: &ReleaseObligations, code: &str) {
    match observe_release_obligations(SUBJECT, input) {
        Err(error) => assert_eq!(error.code, code, "{}", error.detail),
        Ok(receipt) => panic!("{code} admitted {}", receipt.result),
    }
}

fn package(dir: &Path, rev: &str, manifest: &str) -> GitPackage {
    GitPackage {
        repository: dir.to_path_buf(),
        commit: rev.to_owned(),
        manifest: manifest.to_owned(),
    }
}

fn pack_crate(stage: &Path, krate: &Path, sha: &str) {
    let member = stage.join("sim-top-0.1.0");
    std::fs::create_dir_all(&member).expect("stage");
    std::fs::write(member.join("Cargo.toml"), TOP).expect("crate manifest");
    std::fs::write(
        member.join(".cargo_vcs_info.json"),
        format!(r#"{{"git":{{"sha1":"{sha}"}},"path_in_vcs":""}}"#),
    )
    .expect("vcs");
    let packed = Command::new("tar")
        .args([
            "-czf",
            krate.to_str().expect("crate"),
            "-C",
            stage.to_str().expect("stage"),
            "sim-top-0.1.0",
        ])
        .output()
        .expect("tar");
    assert!(
        packed.status.success(),
        "{}",
        String::from_utf8_lossy(&packed.stderr)
    );
}

fn write_index(path: &Path, checksum: &str, yanked: bool) {
    let yanked = if yanked { "true" } else { "false" };
    std::fs::write(
        path,
        format!(r#"{{"name":"sim-top","vers":"0.1.0","cksum":"{checksum}","yanked":{yanked}}}"#),
    )
    .expect("index");
}

fn write_boot(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}")).expect("boot");
    let mut permissions = std::fs::metadata(path).expect("meta").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("mode");
}

fn sha256_file(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).expect("bytes"))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn head(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["--no-replace-objects", "rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("rev-parse");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf8")
        .trim()
        .to_owned()
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "{args:?} {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
