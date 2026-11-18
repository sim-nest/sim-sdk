// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use sim_conformance_v02::CheckScopeId;

use super::{SourceRoundtrip, bind_source_roundtrip, observe_source_roundtrip};

const SUBJECT: &str = "subject/source-roundtrip";

#[test]
fn source_roundtrip_compiles_the_committed_family() {
    let scratch = Scratch::new("pass");
    let chosen = member(
        "chosen",
        "2",
        "selected",
        "family/chosen.txt",
        Some("semantic-tests"),
        "select this source",
        Some("1"),
    );
    let notes = member(
        "notes",
        "1",
        "consulted",
        "family/notes.txt",
        Some("landing-tests"),
        "keep the note",
        None,
    );
    let old = member(
        "old",
        "0",
        "superseded",
        "family/old.txt",
        Some("retired-tests"),
        "retire this source",
        None,
    );
    let rev = commit(
        &scratch.0,
        &[
            ("family/chosen.txt", &chosen),
            ("family/notes.txt", &notes),
            ("family/old.txt", &old),
        ],
    );
    let observed = observe_source_roundtrip(
        SUBJECT,
        &coords(
            &scratch.0,
            &rev,
            &["family/chosen.txt", "family/notes.txt", "family/old.txt"],
        ),
    )
    .expect("compile");
    assert_eq!(observed.result, "source-roundtrip");
    assert_eq!(observed.source_digest, digest(&chosen));
    assert_eq!(observed.name, "chosen");
    assert_eq!(observed.round, "2");
    assert_eq!(observed.facts, vec!["semantic-tests".to_owned()]);
    assert_eq!(observed.consultations, 1);
    assert!(
        !observed
            .facts
            .iter()
            .any(|fact| fact == "landing-tests" || fact == "retired-tests"),
        "sibling facts are not requirements"
    );
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/source-roundtrip").expect("scope")
    );
    bind_source_roundtrip(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-1".to_owned();
    assert_eq!(
        bind_source_roundtrip(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
    std::fs::write(scratch.0.join("family/chosen.txt"), "dirty").expect("dirty");
    let again = observe_source_roundtrip(
        SUBJECT,
        &coords(
            &scratch.0,
            &rev,
            &["family/chosen.txt", "family/notes.txt", "family/old.txt"],
        ),
    )
    .expect("committed");
    assert_eq!(again.source_digest, digest(&chosen));
}

#[test]
fn source_roundtrip_refuses_begin_authority() {
    let scratch = Scratch::new("authority");
    let chosen = member(
        "chosen",
        "2",
        "selected",
        "family/chosen.txt",
        Some("semantic-tests"),
        "begin authority",
        None,
    );
    let rev = commit(&scratch.0, &[("family/chosen.txt", &chosen)]);
    match observe_source_roundtrip(SUBJECT, &coords(&scratch.0, &rev, &["family/chosen.txt"])) {
        Err(error) => assert_eq!(error.code, "selection-authority"),
        Ok(_) => panic!("begin authority admitted a receipt"),
    }
}

#[test]
fn source_roundtrip_refuses_a_non_family_source() {
    let scratch = Scratch::new("header");
    let rev = commit(
        &scratch.0,
        &[("family/chosen.txt", "# NO_V3_18: example\n1. claim\n")],
    );
    match observe_source_roundtrip(SUBJECT, &coords(&scratch.0, &rev, &["family/chosen.txt"])) {
        Err(error) => assert_eq!(error.code, "bad-record"),
        Ok(_) => panic!("non-family source admitted a receipt"),
    }
}

#[test]
fn source_roundtrip_refuses_a_consulted_only_requirement() {
    let scratch = Scratch::new("consulted");
    let chosen = member(
        "chosen",
        "2",
        "selected",
        "family/chosen.txt",
        None,
        "select this source",
        Some("1"),
    );
    let notes = member(
        "notes",
        "1",
        "consulted",
        "family/notes.txt",
        Some("semantic-tests"),
        "keep the note",
        None,
    );
    let rev = commit(
        &scratch.0,
        &[("family/chosen.txt", &chosen), ("family/notes.txt", &notes)],
    );
    match observe_source_roundtrip(
        SUBJECT,
        &coords(&scratch.0, &rev, &["family/chosen.txt", "family/notes.txt"]),
    ) {
        Err(error) => assert_eq!(error.code, "ungrounded"),
        Ok(_) => panic!("consulted fact admitted a receipt"),
    }
}

#[test]
fn source_roundtrip_refuses_two_selected_members() {
    let scratch = Scratch::new("two");
    let chosen = member(
        "chosen",
        "2",
        "selected",
        "family/chosen.txt",
        Some("semantic-tests"),
        "select this source",
        None,
    );
    let other = member(
        "other",
        "3",
        "selected",
        "family/other.txt",
        Some("semantic-tests"),
        "select this source",
        None,
    );
    let rev = commit(
        &scratch.0,
        &[("family/chosen.txt", &chosen), ("family/other.txt", &other)],
    );
    match observe_source_roundtrip(
        SUBJECT,
        &coords(&scratch.0, &rev, &["family/chosen.txt", "family/other.txt"]),
    ) {
        Err(error) => assert_eq!(error.code, "selection-count"),
        Ok(_) => panic!("two selected members admitted a receipt"),
    }
}

#[test]
fn source_roundtrip_ignores_a_replacement_ref() {
    let scratch = Scratch::new("replace");
    let chosen = member(
        "chosen",
        "2",
        "selected",
        "family/chosen.txt",
        Some("semantic-tests"),
        "select this source",
        None,
    );
    let original = commit(&scratch.0, &[("family/chosen.txt", &chosen)]);
    std::fs::write(
        scratch.0.join("family/chosen.txt"),
        "# NO_V3_18: replaced\n",
    )
    .expect("replace");
    git(&scratch.0, &["add", "family/chosen.txt"]);
    git(&scratch.0, &["commit", "-m", "replaced"]);
    let replacement = head(&scratch.0);
    git(&scratch.0, &["replace", &original, &replacement]);
    let observed = observe_source_roundtrip(
        SUBJECT,
        &coords(&scratch.0, &original, &["family/chosen.txt"]),
    )
    .expect("original");
    assert_eq!(observed.result, "source-roundtrip");
    assert_eq!(observed.source_digest, digest(&chosen));
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("nv12-roundtrip-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn coords(dir: &Path, rev: &str, paths: &[&str]) -> SourceRoundtrip {
    SourceRoundtrip {
        repository: dir.to_path_buf(),
        commit: rev.to_owned(),
        paths: paths.iter().map(|path| (*path).to_owned()).collect(),
    }
}

fn member(
    name: &str,
    round: &str,
    stance: &str,
    path: &str,
    fact: Option<&str>,
    decision: &str,
    consult: Option<&str>,
) -> String {
    let mut text = format!(
        "family/v1\nname {name}\nround {round}\nstance {stance}\npath {path}\nargument the argument\nwork the work\ngate the gate\nratchet ratchet-{round}\nowner sim-agent-net\nchecker checker-{round}\nroadmap value-roadmap\n"
    );
    if let Some(fact) = fact {
        text.push_str(&format!("fact {fact}\n"));
    }
    text.push_str("advice keep the source\n");
    text.push_str(&format!("decision {decision}\n"));
    if let Some(consult) = consult {
        text.push_str(&format!("consult {consult}\n"));
    }
    text
}

fn digest(bytes: &str) -> String {
    Sha256::digest(bytes.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn commit(dir: &Path, files: &[(&str, &str)]) -> String {
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "nv12@example.com"]);
    git(dir, &["config", "user.name", "nv12"]);
    for (path, body) in files {
        let full = dir.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(full, body).expect("file");
        git(dir, &["add", path]);
    }
    git(dir, &["commit", "-m", "family"]);
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
