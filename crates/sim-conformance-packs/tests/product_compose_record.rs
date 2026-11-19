// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Composition comes from the printed qualify records, not caller facts.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const COMPOSITION: &str = "#(world/qualify-v1 (case \"composition\") (detail \"pages 1 revision 8eed83fd708e5c7a0944c8627663af1f8bc105a9c5a6f7d5659122e4661be28b transitions 1 stale-effects 0 effects 1 hot-delta 0 samples 20 providers model/direction,model/landing,model/snapshot stale-edges 1 bound 1 proof 0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e linux 0\") (effects 0) (invokes 1) (resident true) (subprocesses 0))";
const LINUX: &str = "#(world/qualify-v1 (case \"linux\") (detail \"linux-provider\") (effects 0) (invokes 0) (resident true) (subprocesses 0))";
const FREEZE: &str = "#(world/qualify-v1 (case \"freeze\") (detail \"freeze-mismatch empty freeze-mismatch\") (effects 0) (invokes 0) (resident true) (subprocesses 0))";
const ADVICE: &str = "#(world/qualify-v1 (case \"advice\") (detail \"advice-rejected\") (effects 0) (invokes 0) (resident true) (subprocesses 0))";
const BEGIN: &str = "#(world/qualify-v1 (case \"begin\") (detail \"distinct begin-required landed 1 stale 1 proof 0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e\") (effects 0) (invokes 1) (resident true) (subprocesses 0))";
const PRIVATE: &str = "#(world/qualify-gap-v1 (case \"\") (effects 0) (evidence \"\") (reason \"private-leakage\") (resident true) (subprocesses 0))";

struct Lines {
    composition: String,
    linux: String,
    freeze: String,
    advice: String,
    begin: String,
    private: String,
}

fn lawful() -> Lines {
    Lines {
        composition: COMPOSITION.to_owned(),
        linux: LINUX.to_owned(),
        freeze: FREEZE.to_owned(),
        advice: ADVICE.to_owned(),
        begin: BEGIN.to_owned(),
        private: PRIVATE.to_owned(),
    }
}

fn rows(lines: &Lines) -> Vec<(&str, &str)> {
    vec![
        ("composition", lines.composition.as_str()),
        ("linux", lines.linux.as_str()),
        ("freeze", lines.freeze.as_str()),
        ("advice", lines.advice.as_str()),
        ("begin", lines.begin.as_str()),
        ("private-material:s3cret-token", lines.private.as_str()),
    ]
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn execute(evidence: &MemorySubject) -> PackVerdict {
    let binding = find_pack("checker/c-product")
        .unwrap()
        .checker_binding()
        .unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    packs::product::check(&PackRequest {
        checker: "checker/c-product",
        binding,
        subject: SUBJECT,
        scope: "product/composed",
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
    MemorySubject::default().with(packs::product::COMMAND_FACT, path.display().to_string())
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("sim-compose-record-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sim(dir: &Path, lines: &Lines) -> PathBuf {
    let mut body = String::from("#!/bin/sh\nset -e\ncase \"$1:$2:$3\" in\n");
    for (case, line) in rows(lines) {
        body.push_str(&format!(
            "world:qualify:{case})\n  printf '%s\\n' '{line}'\n  ;;\n"
        ));
    }
    body.push_str("*)\n  exit 1\n  ;;\nesac\n");
    let path = dir.join("sim");
    std::fs::write(&path, body).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn refuse_lines(name: &str, lines: &Lines) -> (String, String) {
    let dir = scratch(name);
    let program = write_sim(&dir, lines);
    refusal(&execute(&command(&program)))
}

fn value<'a>(verdict: &'a PackVerdict, key: &str) -> &'a str {
    match verdict {
        PackVerdict::Pass { observations, .. } => observations
            .iter()
            .find(|observation| observation.key == key)
            .map(|observation| observation.value.as_str())
            .unwrap_or_else(|| panic!("missing {key}")),
        other => panic!("expected a pass, got {other:?}"),
    }
}

#[test]
fn caller_facts_are_not_a_composition() {
    let (code, detail) = refusal(&execute(&MemorySubject::default()));
    assert_eq!(code, "missing-source");
    assert_eq!(detail, "product.command");
    let truths = MemorySubject::default().with("product.brochure-present", "true");
    let (code, detail) = refusal(&execute(&truths));
    assert_eq!(code, "unexpected-fact");
    assert_eq!(detail, "product.brochure-present");
}

#[test]
fn a_printed_record_is_the_composition_observation() {
    let dir = scratch("printed");
    let program = write_sim(&dir, &lawful());
    let verdict = execute(&command(&program));
    assert!(value(&verdict, "product.qualify-composition").contains("samples 20"));
    assert!(value(&verdict, "product.qualify-linux").contains("linux-provider"));
    assert!(value(&verdict, "product.qualify-freeze").contains("freeze-mismatch"));
    assert!(value(&verdict, "product.qualify-advice").contains("advice-rejected"));
    assert!(value(&verdict, "product.qualify-begin").contains("begin-required"));
    assert!(value(&verdict, "product.qualify-private").contains("private-leakage"));
}

#[test]
fn a_changed_revision_is_not_the_composition() {
    let mut lines = lawful();
    lines.composition = lines.composition.replace("8eed83fd", "aaaaaaaa");
    let (code, detail) = refuse_lines("revision", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "product.qualify-composition");
}

#[test]
fn a_missing_effect_count_is_not_zero() {
    let mut lines = lawful();
    lines.composition = lines.composition.replace(" (effects 0)", "");
    let (code, detail) = refuse_lines("missing-effects", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "effects");
}

#[test]
fn a_negative_effect_count_is_not_zero() {
    let mut lines = lawful();
    lines.composition = lines.composition.replace("(effects 0)", "(effects -1)");
    let (code, detail) = refuse_lines("effects", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "effects");
}

#[test]
fn a_negative_subprocess_count_is_not_zero() {
    let mut lines = lawful();
    lines.linux = lines.linux.replace("(subprocesses 0)", "(subprocesses -1)");
    let (code, detail) = refuse_lines("subprocesses", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "subprocesses");
}

#[test]
fn a_resident_phrase_is_not_resident() {
    let mut lines = lawful();
    lines.freeze = lines
        .freeze
        .replace("(resident true)", "(resident \"not resident\")");
    let (code, detail) = refuse_lines("resident", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "resident");
}

#[test]
fn a_copied_private_value_is_not_refused() {
    let mut lines = lawful();
    lines.private = lines.private.replace("private-leakage", "copied");
    let (code, detail) = refuse_lines("private", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "product.qualify-private");
}

#[test]
fn a_second_record_on_the_same_line_is_not_the_composition() {
    let mut lines = lawful();
    lines.composition = format!("#(world/qualify-v1){COMPOSITION}");
    let (code, detail) = refuse_lines("glued", &lines);
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_empty_root_with_a_space_glued_to_a_second_record_is_extra() {
    let mut lines = lawful();
    lines.begin = format!("#(world/qualify-v1 ){BEGIN}");
    let (code, detail) = refuse_lines("glued-space", &lines);
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_unclosed_root_is_not_a_record() {
    let mut lines = lawful();
    lines.advice = ADVICE.strip_suffix(')').expect("root closes").to_owned();
    let (code, detail) = refuse_lines("unclosed", &lines);
    assert_eq!(code, "bad-record");
    assert_eq!(detail, "unclosed");
}

#[test]
fn installed_sim_records_the_composition_laws() {
    let Some(path) = std::env::var_os("SIM_PRODUCT_BIN") else {
        let (code, detail) = refusal(&execute(&MemorySubject::default()));
        assert_eq!(code, "missing-source");
        assert_eq!(detail, "product.command");
        return;
    };
    let program = PathBuf::from(path);
    assert!(program.is_file(), "installed sim is missing");
    let verdict = execute(&command(&program));
    let composition = value(&verdict, "product.qualify-composition");
    assert!(composition.contains("#(world/qualify-v1 "));
    assert!(
        composition
            .contains("revision 8eed83fd708e5c7a0944c8627663af1f8bc105a9c5a6f7d5659122e4661be28b")
    );
    assert!(composition.contains("samples 20"));
    assert!(composition.contains("(invokes 1)"));
    assert!(value(&verdict, "product.qualify-linux").contains("linux-provider"));
    assert!(
        value(&verdict, "product.qualify-freeze").contains("freeze-mismatch empty freeze-mismatch")
    );
    assert!(value(&verdict, "product.qualify-advice").contains("advice-rejected"));
    assert!(value(&verdict, "product.qualify-begin").contains("begin-required"));
    let private = value(&verdict, "product.qualify-private");
    assert!(private.contains("(reason \"private-leakage\")"));
    assert!(!private.contains("private-material:"));
    assert!(!composition.contains("completion/claim-"));
    assert!(!composition.contains("PortableClosed"));
    assert!(!composition.contains("derive_completion"));
}
