// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Authoring laws come from the printed author records, not caller facts.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const SELECTED: &str = "#(world/author-v1 (bytes 278) (case \"selected\") (changed \"name,round,stance,path,ratchet,checker,consult\") (claims 1) (class \"selected\") (closed \"true\") (compiled \"true\") (consultation 1) (effects 0) (entries 1) (grounded \"true\") (lint \"ok\") (members 2) (phase \"authoring\") (postimage 278) (proof \"source-deck\") (resident true) (round-trip \"true\") (selection \"chosen\") (shapes \"distinct\") (shown \"chosen\") (subprocesses 0) (uncertain \"false\"))";

const CONSULTED: &str = "#(world/author-gap-v1 (case \"consulted\") (effects 0) (evidence \"semantic-tests\") (reason \"unsatisfied\") (resident true) (subprocesses 0))";

const SUPERSEDED: &str = "#(world/author-gap-v1 (case \"superseded\") (effects 0) (evidence \"semantic-tests\") (reason \"unsatisfied\") (resident true) (subprocesses 0))";

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn execute(scope: &str, evidence: &MemorySubject) -> PackVerdict {
    let binding = find_pack("checker/c-author")
        .unwrap()
        .checker_binding()
        .unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    packs::authoring::check(&PackRequest {
        checker: "checker/c-author",
        binding,
        subject: SUBJECT,
        scope,
        evidence,
    })
}

fn refusal(verdict: &PackVerdict) -> (String, String) {
    match verdict {
        PackVerdict::Refused(failure) => (failure.code.to_owned(), failure.detail.clone()),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn command(path: &Path) -> MemorySubject {
    MemorySubject::default().with(packs::authoring::COMMAND_FACT, path.display().to_string())
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sim-authoring-record-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sim(dir: &Path, selected: &str, consulted: &str, superseded: &str) -> PathBuf {
    let path = dir.join("sim");
    let body = format!(
        "#!/bin/sh\nset -e\ncase \"$1:$2:$3\" in\nworld:author:selected)\n  cat <<'EOF'\n{selected}\nEOF\n  ;;\nworld:author:consulted)\n  cat <<'EOF'\n{consulted}\nEOF\n  ;;\nworld:author:superseded)\n  cat <<'EOF'\n{superseded}\nEOF\n  ;;\n*)\n  exit 1\n  ;;\nesac\n"
    );
    std::fs::write(&path, body).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn result_text(verdict: &PackVerdict) -> String {
    match verdict {
        PackVerdict::Pass { result, .. } => format!("{result:?}"),
        other => panic!("expected a pass, got {other:?}"),
    }
}

fn value<'a>(values: &[(&'a str, &'a str)], key: &str) -> &'a str {
    values
        .iter()
        .find(|(found, _)| *found == key)
        .map(|(_, body)| *body)
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn pass_values(verdict: &PackVerdict) -> Vec<(&str, &str)> {
    match verdict {
        PackVerdict::Pass { observations, .. } => observations
            .iter()
            .map(|observation| (observation.key.as_str(), observation.value.as_str()))
            .collect(),
        other => panic!("expected a pass, got {other:?}"),
    }
}

#[test]
fn caller_authoring_facts_are_not_an_authoring() {
    let (code, detail) = refusal(&execute("authoring/family", &MemorySubject::default()));
    assert_eq!(code, "missing-source");
    assert_eq!(detail, "authoring.command");
    let mut truths = MemorySubject::default();
    for key in packs::authoring::SCENARIOS {
        truths = truths.with(key, "true");
    }
    let (code, detail) = refusal(&execute("authoring/composed", &truths));
    assert_eq!(code, "unexpected-fact");
    assert_eq!(detail, "authoring.consultation-not-a-requirement");
}

#[test]
fn a_printed_record_is_the_authoring_observation() {
    let dir = scratch("printed");
    let program = write_sim(&dir, SELECTED, CONSULTED, SUPERSEDED);
    let mut results = Vec::new();
    for scope in ["authoring/family", "authoring/composed"] {
        let verdict = execute(scope, &command(&program));
        let values = pass_values(&verdict);
        let selected = value(&values, "authoring.selected");
        let consulted = value(&values, "authoring.consulted");
        let superseded = value(&values, "authoring.superseded");
        assert_eq!(values.len(), 3);
        assert!(selected.contains("#(world/author-v1 "));
        assert!(consulted.contains("consulted"));
        assert!(superseded.contains("superseded"));
        results.push(result_text(&verdict));
    }
    assert_ne!(
        results[0], results[1],
        "authoring scopes must not share one result"
    );
}

#[test]
fn a_short_family_is_not_self_contained() {
    let dir = scratch("members");
    let selected = SELECTED.replace("(members 2)", "(members 1)");
    let program = write_sim(&dir, &selected, CONSULTED, SUPERSEDED);
    let (code, detail) = refusal(&execute("authoring/family", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "authoring.family-self-contained");
}

#[test]
fn a_compiled_consultation_is_not_provenance() {
    let dir = scratch("consult");
    let consulted = SELECTED.replace("(case \"selected\")", "(case \"consulted\")");
    let program = write_sim(&dir, SELECTED, &consulted, SUPERSEDED);
    let (code, detail) = refusal(&execute("authoring/family", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "authoring.consultation-not-a-requirement");
}

#[test]
fn a_sibling_requirement_is_not_the_selection() {
    let dir = scratch("sibling");
    let consulted = CONSULTED.replace("semantic-tests", "sibling-rule");
    let program = write_sim(&dir, SELECTED, &consulted, SUPERSEDED);
    let (code, detail) = refusal(&execute("authoring/composed", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "authoring.forbidden-sibling-requirement-refused");
}

#[test]
fn a_superseded_sibling_is_not_selected() {
    let dir = scratch("super");
    let superseded = SUPERSEDED.replace("unsatisfied", "selected");
    let program = write_sim(&dir, SELECTED, CONSULTED, &superseded);
    let (code, detail) = refusal(&execute("authoring/family", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "authoring.superseded-not-selected");
}

#[test]
fn a_different_member_is_not_the_human_selection() {
    let dir = scratch("human");
    let selected = SELECTED.replace("(selection \"chosen\")", "(selection \"notes\")");
    let program = write_sim(&dir, &selected, CONSULTED, SUPERSEDED);
    let (code, detail) = refusal(&execute("authoring/composed", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "authoring.human-selection-exact");
}

#[test]
fn a_changed_source_is_not_an_exact_round_trip() {
    let dir = scratch("round");
    let selected = SELECTED.replace("(round-trip \"true\")", "(round-trip \"false\")");
    let program = write_sim(&dir, &selected, CONSULTED, SUPERSEDED);
    let (code, detail) = refusal(&execute("authoring/family", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "authoring.exact-source-roundtrip");
}

#[test]
fn a_missing_round_trip_is_not_exact() {
    let dir = scratch("round-missing");
    let selected = SELECTED.replace(" (round-trip \"true\")", "");
    let program = write_sim(&dir, &selected, CONSULTED, SUPERSEDED);
    let (code, detail) = refusal(&execute("authoring/family", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "authoring.exact-source-roundtrip");
}

#[test]
fn a_second_record_on_the_same_line_is_not_the_authoring() {
    let dir = scratch("glued");
    let selected = format!("#(world/author-v1){SELECTED}");
    let program = write_sim(&dir, &selected, CONSULTED, SUPERSEDED);
    let (code, detail) = refusal(&execute("authoring/family", &command(&program)));
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_empty_root_with_a_space_glued_to_a_second_record_is_extra() {
    let dir = scratch("glued-space");
    let selected = format!("#(world/author-v1 ){SELECTED}");
    let program = write_sim(&dir, &selected, CONSULTED, SUPERSEDED);
    let (code, detail) = refusal(&execute("authoring/family", &command(&program)));
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_unclosed_root_is_not_a_record() {
    let dir = scratch("unclosed");
    let selected = SELECTED.strip_suffix(')').expect("root closes");
    let program = write_sim(&dir, selected, CONSULTED, SUPERSEDED);
    let (code, detail) = refusal(&execute("authoring/composed", &command(&program)));
    assert_eq!(code, "bad-record");
    assert_eq!(detail, "unclosed");
}

#[test]
fn installed_sim_records_the_authoring_laws() {
    let Some(path) = std::env::var_os("SIM_PRODUCT_BIN") else {
        let (code, detail) = refusal(&execute("authoring/composed", &MemorySubject::default()));
        assert_eq!(code, "missing-source");
        assert_eq!(detail, "authoring.command");
        return;
    };
    let program = PathBuf::from(path);
    assert!(program.is_file(), "installed sim is missing");
    let verdict = execute("authoring/family", &command(&program));
    let values = pass_values(&verdict);
    let selected = value(&values, "authoring.selected");
    let consulted = value(&values, "authoring.consulted");
    let superseded = value(&values, "authoring.superseded");
    assert!(selected.contains("#(world/author-v1 "));
    assert!(selected.contains("(selection \"chosen\")"));
    assert!(selected.contains("(round-trip \"true\")"));
    assert!(consulted.contains("(case \"consulted\")"));
    assert!(consulted.contains("unsatisfied"));
    assert!(superseded.contains("(case \"superseded\")"));
    assert!(!selected.contains("completion/claim-"));
    assert!(!selected.contains("PortableClosed"));
    assert!(!selected.contains("derive_completion"));
}
