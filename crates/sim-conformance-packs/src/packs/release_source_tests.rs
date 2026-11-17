// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::{
    CheckScopeId, CheckedSubjectId, CheckerResultId, EvidenceGrade, RevocationStatus,
};
use sim_lib_roadmap_runner::EvidenceDecision;

use crate::{CheckObservation, MemorySubject, PackRequest};

use super::{admit_parsed, observations};

const SUBJECT: &str = "subject/source-example";
const REV: &str = "0123456789abcdef0123456789abcdef01234567";
const REV_B: &str = "0123456789abcdef0123456789abcdef01234568";
const REV_C: &str = "fedcba9876543210fedcba9876543210fedcba98";

fn inline_git(name: &str, rev: &str) -> String {
    format!(
        "{name} = {{ version = \"0.1.0\", git = \"https://example.invalid/{name}.git\", rev = \"{rev}\" }}"
    )
}

fn manifest() -> String {
    format!(
        "[package]\npublish = false\n[dependencies]\n{}\n",
        inline_git("sim-example", REV)
    )
}

fn identity() -> String {
    format!("git+{REV}:sim-example@0.1.0")
}

fn expected_result() -> String {
    format!("source-identity {}", identity())
}

fn refuses(text: &str, code: &str) {
    match super::observe_source_identity(SUBJECT, text) {
        Err(error) => assert_eq!(error.code, code, "{text}"),
        Ok(_) => panic!("{code} must not admit a receipt"),
    }
}

fn request<'a>(evidence: &'a MemorySubject) -> PackRequest<'a> {
    PackRequest {
        checker: "checker/c-release",
        binding: "core/sha256-datum-v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        subject: "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        scope: "release/source-identity",
        evidence,
    }
}

#[test]
fn a_parsed_manifest_admits_its_source_receipt() {
    let text = manifest();
    let observed = super::observe_source_identity(SUBJECT, &text).expect("admitted");
    let expected = expected_result();
    assert_eq!(observed.subject, SUBJECT);
    assert_eq!(observed.result, expected);
    assert!(!observed.result.contains("completion/claim-"));
    assert_eq!(observed.revocation, RevocationStatus::Current);
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/source-identity").expect("scope")
    );
    let receipt = observed
        .catalog
        .checker_receipt(&observed.checker, RevocationStatus::Current)
        .expect("receipt");
    assert_eq!(
        receipt.subject(),
        &CheckedSubjectId::from_text(SUBJECT).expect("subject")
    );
    assert_ne!(
        receipt.subject(),
        &CheckedSubjectId::from_text("subject/other").expect("other")
    );
    assert_eq!(
        receipt.result(),
        &CheckerResultId::from_text(&expected).expect("result")
    );
    match observed
        .catalog
        .decide_evidence(&observed.admitted.key, EvidenceGrade::Release, |_| {
            RevocationStatus::Current
        }) {
        EvidenceDecision::Reuse { receipt } => assert_eq!(receipt, observed.admitted.id),
        other => panic!("expected release reuse, got {other:?}"),
    }
}

#[test]
fn shade_1_produces_no_receipt() {
    let text = format!("publish = false\nshade-1\n{}", manifest());
    match super::observe_source_identity(SUBJECT, &text) {
        Err(error) => assert_eq!(error.code, "construction-specimen"),
        Ok(_) => panic!("shade-1 must not admit a receipt"),
    }
}

#[test]
fn publish_true_produces_no_receipt() {
    let text = manifest().replacen("publish = false", "publish = true", 1);
    match super::observe_source_identity(SUBJECT, &text) {
        Err(error) => assert_eq!(error.code, "local-publication-claim"),
        Ok(_) => panic!("publish = true must not admit a receipt"),
    }
}

#[test]
fn an_extra_fact_is_refused() {
    let evidence = MemorySubject::default()
        .with("release.manifest", manifest())
        .with("release.extra", "1");
    let error = observations(&request(&evidence)).expect_err("extra");
    assert_eq!(error.code, "unexpected-fact");
}

#[test]
fn a_forged_source_observation_is_refused() {
    let text = manifest();
    let evidence = MemorySubject::default().with("release.manifest", text.clone());
    let mut seen = observations(&request(&evidence)).expect("observations");
    let source = seen
        .iter_mut()
        .find(|item| item.key == "release.source")
        .expect("source");
    source.value = "git+not-an-identity".to_owned();
    match admit_parsed(SUBJECT, &text, &seen) {
        Err(error) => assert_eq!(error.code, "evidence-mismatch"),
        Ok(_) => panic!("a forged source observation must not admit a receipt"),
    }
}

#[test]
fn another_subject_is_another_receipt() {
    let text = manifest();
    let first = super::observe_source_identity(SUBJECT, &text).expect("first");
    let second = super::observe_source_identity("subject/other", &text).expect("second");
    assert_eq!(first.result, second.result);
    assert_ne!(first.checker, second.checker);
    assert_ne!(first.admitted.id, second.admitted.id);
}

#[test]
fn a_commented_git_key_produces_no_receipt() {
    let text = format!(
        "[package]\npublish = false\n[dependencies]\nsim-example = {{ version = \"0.1.0\" }} # git = \"https://example.invalid/sim-example.git\", rev = \"{REV}\"\n"
    );
    match super::observe_source_identity(SUBJECT, &text) {
        Err(error) => assert_eq!(error.code, "missing-source"),
        Ok(_) => panic!("a comment must not admit a git identity"),
    }
}

#[test]
fn a_quoted_package_publish_true_produces_no_receipt() {
    let text = format!(
        "[package]\n\"publish\" = true\n[package.metadata]\npublish = false\n[dependencies]\nsim-example = {{ version = \"0.1.0\", git = \"https://example.invalid/sim-example.git\", rev = \"{REV}\" }}\n"
    );
    match super::observe_source_identity(SUBJECT, &text) {
        Err(error) => assert_eq!(error.code, "local-publication-claim"),
        Ok(_) => panic!("package publish = true must not admit a receipt"),
    }
}

#[test]
fn metadata_publish_false_does_not_cover_a_package() {
    let text = format!(
        "[package]\nname = \"source-review-fixture\"\n[package.metadata]\npublish = false\n[dependencies]\nsim-example = {{ version = \"0.1.0\", git = \"https://example.invalid/sim-example.git\", rev = \"{REV}\" }}\n"
    );
    match super::observe_source_identity(SUBJECT, &text) {
        Err(error) => assert_eq!(error.code, "missing-observation"),
        Ok(_) => panic!("metadata publish = false must not cover the package"),
    }
}

#[test]
fn a_package_table_admits_its_real_git_dependency() {
    let text = format!(
        "[package]\npublish = false\n[package.metadata]\npublish = true\n[dependencies]\nsim-example = {{ version = \"0.1.0\", git = \"https://example.invalid/sim-example.git\", rev = \"{REV}\" }} # not a source\n"
    );
    let observed = super::observe_source_identity(SUBJECT, &text).expect("admitted");
    assert_eq!(observed.result, expected_result());
    assert!(!observed.result.contains("completion/claim-"));
}

#[test]
fn observations_name_only_parsed_identities() {
    let evidence = MemorySubject::default().with("release.manifest", manifest());
    let seen = observations(&request(&evidence)).expect("observations");
    assert_eq!(
        seen,
        vec![
            CheckObservation {
                key: "release.unpublished".to_owned(),
                value: "publish-false".to_owned(),
            },
            CheckObservation {
                key: "release.source".to_owned(),
                value: identity(),
            },
        ]
    );
}

#[test]
fn a_metadata_git_table_produces_no_dependency_evidence() {
    let text = format!(
        "[package]\npublish = false\n[package.metadata]\n{}\n",
        inline_git("sim-example", REV)
    );
    refuses(&text, "missing-source");
    let evidence = MemorySubject::default().with("release.manifest", text);
    let error = observations(&request(&evidence)).expect_err("metadata table");
    assert_eq!(error.code, "missing-source");
}

#[test]
fn nested_metadata_and_workspace_tables_produce_no_dependency_evidence() {
    let git = inline_git("sim-example", REV);
    for text in [
        format!("[package]\npublish = false\n[package.metadata.tools]\n{git}\n"),
        format!("[package]\npublish = false\n[package.metadata.dependencies]\n{git}\n"),
        format!("[package]\npublish = false\n[workspace.dependencies]\n{git}\n"),
    ] {
        refuses(&text, "missing-source");
    }
}

#[test]
fn a_metadata_git_table_beside_a_real_dependency_admits_only_the_real_one() {
    let text = format!(
        "[package]\npublish = false\n[package.metadata]\n{}\n[dependencies]\nsim-alias = {{ package = \"sim-example\", version = \"0.1.0\", git = \"https://example.invalid/sim-example.git\", rev = \"{REV}\" }}\nsim-crates = \"0.1.0\"\n",
        inline_git("sim-hidden", REV)
    );
    let observed = super::observe_source_identity(SUBJECT, &text).expect("admitted");
    assert_eq!(observed.result, expected_result());
    assert!(!observed.result.contains("completion/claim-"));
    assert!(!observed.result.contains("sim-hidden"));
    assert!(!observed.result.contains("sim-alias"));
    assert!(!observed.result.contains("sim-crates"));
    let evidence = MemorySubject::default().with("release.manifest", text);
    let seen = observations(&request(&evidence)).expect("observations");
    let sources = seen
        .iter()
        .filter(|item| item.key == "release.source")
        .map(|item| item.value.as_str())
        .collect::<Vec<_>>();
    assert_eq!(sources, vec![identity().as_str()]);
}

#[test]
fn dev_build_and_target_dependency_tables_admit() {
    let text = format!(
        "[package]\npublish = false\n[dev-dependencies]\n{}\n[build-dependencies]\n{}\n[target.'cfg(unix)'.dependencies]\n{}\n",
        inline_git("sim-example", REV),
        inline_git("sim-build", REV_B),
        inline_git("sim-target", REV_C)
    );
    let mut identities = [
        format!("git+{REV}:sim-example@0.1.0"),
        format!("git+{REV_B}:sim-build@0.1.0"),
        format!("git+{REV_C}:sim-target@0.1.0"),
    ];
    identities.sort();
    let expected = format!("source-identity {}", identities.join(" "));
    let observed = super::observe_source_identity(SUBJECT, &text).expect("admitted");
    assert_eq!(observed.result, expected);
    assert!(!observed.result.contains("completion/claim-"));
}

#[test]
fn a_target_named_metadata_is_still_a_dependency_table() {
    let text = format!(
        "[package]\npublish = false\n[target.metadata.dependencies]\n{}\n",
        inline_git("sim-example", REV)
    );
    let observed = super::observe_source_identity(SUBJECT, &text).expect("admitted");
    assert_eq!(observed.result, expected_result());
}

#[test]
fn invalid_toml_produces_no_receipt() {
    let text = format!(
        "[package]\npublish = false\nthis is not toml\n[dependencies]\n{}\n",
        inline_git("sim-example", REV)
    );
    refuses(&text, "bad-record");
}

#[test]
fn the_same_crate_in_two_dependency_tables_is_duplicate() {
    let git = inline_git("sim-example", REV);
    let text =
        format!("[package]\npublish = false\n[dependencies]\n{git}\n[dev-dependencies]\n{git}\n");
    refuses(&text, "duplicate-crate");
}
