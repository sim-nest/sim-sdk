// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-12` runs the named program. Closure booleans are not a catalog.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn request<'a>(evidence: &'a MemorySubject) -> PackRequest<'a> {
    let binding = find_pack("checker/c-release")
        .unwrap()
        .checker_binding()
        .unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    PackRequest {
        checker: "checker/c-release",
        binding,
        subject: SUBJECT,
        scope: "release/nv12-12",
        evidence,
    }
}

fn execute(evidence: &MemorySubject) -> PackVerdict {
    packs::release::check(&request(evidence))
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
    let dir = std::env::temp_dir().join(format!(
        "sim-release-evidence-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sim(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("sim");
    std::fs::write(&path, format!("#!/bin/sh\nset -e\n{body}")).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn init_checkout(dir: &Path) {
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git init")
            .success()
    );
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

fn admitted() -> String {
    "#(world/evidence-v1 (case \"admit\") (domain \"wrong-domain\") (effects 0) (key \"same\") (missing \"missing-object\") (receipt \"moved\") (resident true) (revoked \"revoked-support\") (storage \"storage-id-crossing\") (subprocesses 0) (telemetry \"same\"))".to_owned()
}

fn gap() -> String {
    "#(world/evidence-gap-v1 (case \"other\") (effects 0) (reason \"unknown-case\") (resident true) (subprocesses 0))".to_owned()
}

fn prepare(name: &str, body: &str) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    init_checkout(&dir);
    let program = write_sim(&dir, body);
    (dir, program)
}

fn admit_body(record: &str, receipt: bool) -> String {
    assert!(!record.contains('\''));
    let mut body = String::from(
        "if [ \"$1\" != world ] || [ \"$2\" != why ] || [ \"$3\" != evidence ] || [ \"$4\" != admit ] || [ -n \"${5:-}\" ]; then\n  printf '%s\\n' \"unexpected $1 $2 $3 $4\" >&2\n  exit 1\nfi\nprintf '%s\\n' admit >> \"$LOG\"\nprintf '%s\\n' '",
    );
    body.push_str(record);
    body.push_str("'\n");
    if receipt {
        body.push_str(
            "printf '%s\\n' '#(world/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/world-command\"))'\n",
        );
    }
    body
}

fn pass_value<'a>(verdict: &'a PackVerdict, key: &str) -> &'a str {
    match verdict {
        PackVerdict::Pass { observations, .. } => observations
            .iter()
            .find(|item| item.key == key)
            .unwrap_or_else(|| panic!("missing {key}"))
            .value
            .as_str(),
        other => panic!("expected a pass, got {other:?}"),
    }
}

fn with_log(dir: &Path, inner: &str) -> String {
    let log = shell_quote(&dir.join("ran.log"));
    format!("LOG={log}\n{inner}")
}

#[test]
fn closure_facts_do_not_execute_the_evidence_catalog() {
    let closure = packs::release::CLOSURE_FACTS
        .into_iter()
        .fold(MemorySubject::default(), |subject, key| {
            subject.with(key, "true")
        });
    let (code, detail) = refusal(&execute(&closure));
    assert_eq!(code, "unexpected-fact");
    assert_eq!(detail, "release.audit-passed");
    let (code, detail) = refusal(&execute(&MemorySubject::default()));
    assert_eq!(code, "missing-source");
    assert_eq!(detail, packs::product::COMMAND_FACT);
    let claimed = MemorySubject::default().with(packs::product::COMMAND_FACT, "true");
    let (code, _) = refusal(&execute(&claimed));
    assert_eq!(code, "bad-record");
    let relabelled =
        MemorySubject::default().with(packs::product::COMMAND_FACT, "completion/claim-10");
    let (code, _) = refusal(&execute(&relabelled));
    assert_eq!(code, "claim-relabelled");
}

#[test]
fn the_named_program_supplies_the_evidence_record() {
    let record = admitted();
    let dir = scratch("pass");
    init_checkout(&dir);
    let program = write_sim(&dir, &with_log(&dir, &admit_body(&record, false)));
    let verdict = execute(&command(&program));
    let stdout = pass_value(&verdict, "release.evidence.admit");
    assert_eq!(stdout, format!("{record}\n"));
    let log = std::fs::read_to_string(dir.join("ran.log")).unwrap();
    assert_eq!(log, "admit\nadmit\n");
}

#[test]
fn a_load_receipt_stays_outside_the_record() {
    let record = admitted();
    let dir = scratch("receipt");
    init_checkout(&dir);
    let program = write_sim(&dir, &with_log(&dir, &admit_body(&record, true)));
    let verdict = execute(&command(&program));
    let stdout = pass_value(&verdict, "release.evidence.admit");
    assert!(stdout.starts_with(&record), "{stdout}");
    assert!(stdout.contains("\n#(world/load-receipt-v1 "), "{stdout}");
}

#[test]
fn a_gap_is_not_the_specimen() {
    let gap = gap();
    let (_dir, program) = prepare("gap", &format!("printf '%s\\n' '{gap}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "world/evidence-v1");
}

#[test]
fn a_prefixed_key_is_refused() {
    let prefixed = admitted().replace("(key \"same\")", "(key \"same-other\")");
    assert!(prefixed.contains("(key \"same-other\")"), "{prefixed}");
    let (_dir, program) = prepare("key", &format!("printf '%s\\n' '{prefixed}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "key");
}

#[test]
fn a_prefixed_case_is_refused() {
    let prefixed = admitted().replace("(case \"admit\")", "(case \"admit-other\")");
    let (_dir, program) = prepare("case", &format!("printf '%s\\n' '{prefixed}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "case");
}

#[test]
fn moved_telemetry_is_refused() {
    let moved = admitted().replace("(telemetry \"same\")", "(telemetry \"moved\")");
    let (_dir, program) = prepare("telemetry", &format!("printf '%s\\n' '{moved}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "telemetry");
}

#[test]
fn an_unmoved_receipt_is_refused() {
    let stayed = admitted().replace("(receipt \"moved\")", "(receipt \"same\")");
    let (_dir, program) = prepare("receipt-same", &format!("printf '%s\\n' '{stayed}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "receipt");
}

#[test]
fn an_admitted_refusal_mark_is_refused() {
    for (field, expected) in [
        ("domain", "wrong-domain"),
        ("revoked", "revoked-support"),
        ("missing", "missing-object"),
        ("storage", "storage-id-crossing"),
    ] {
        let bad = admitted().replace(
            &format!("({field} \"{expected}\")"),
            &format!("({field} \"admitted\")"),
        );
        let (_dir, program) = prepare(field, &format!("printf '%s\\n' '{bad}'\n"));
        let (code, detail) = refusal(&execute(&command(&program)));
        assert_eq!(code, "record-mismatch", "{field} {detail}");
        assert_eq!(detail, field);
    }
}

#[test]
fn an_extra_policy_field_is_refused() {
    let extra = admitted().replace(
        "(telemetry \"same\"))",
        "(telemetry \"same\") (policy \"policy/other\"))",
    );
    let (_dir, program) = prepare("policy", &format!("printf '%s\\n' '{extra}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "fields");
}

#[test]
fn a_reordered_record_is_refused() {
    let swapped = admitted().replace(
        "(case \"admit\") (domain \"wrong-domain\")",
        "(domain \"wrong-domain\") (case \"admit\")",
    );
    let (_dir, program) = prepare("order", &format!("printf '%s\\n' '{swapped}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "fields");
}

#[test]
fn a_duplicate_key_is_refused() {
    let duplicated = admitted().replace("(key \"same\")", "(key \"same\") (key \"same\")");
    let (_dir, program) = prepare("duplicate", &format!("printf '%s\\n' '{duplicated}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "bad-record", "{detail}");
    assert_eq!(detail, "key");
}

#[test]
fn a_nonzero_count_is_refused() {
    for (field, from, to) in [
        ("effects", "(effects 0)", "(effects 1)"),
        ("subprocesses", "(subprocesses 0)", "(subprocesses 1)"),
        ("resident", "(resident true)", "(resident false)"),
        ("effects-quoted", "(effects 0)", "(effects \"0\")"),
    ] {
        let bad = admitted().replace(from, to);
        let (_dir, program) = prepare(field, &format!("printf '%s\\n' '{bad}'\n"));
        let (code, detail) = refusal(&execute(&command(&program)));
        let expected = if field == "effects-quoted" {
            "effects"
        } else {
            field
        };
        assert_eq!(code, "record-mismatch", "{field} {detail}");
        assert_eq!(detail, expected);
    }
}

#[test]
fn a_changed_second_admit_is_refused() {
    let record = admitted();
    let moved = record.replace("(key \"same\")", "(key \"moved\")");
    let dir = scratch("unstable");
    init_checkout(&dir);
    let log = shell_quote(&dir.join("ran.log"));
    let count = shell_quote(&dir.join("count"));
    let body = format!(
        "LOG={log}\nCOUNT={count}\nif [ \"$1\" != world ] || [ \"$2\" != why ] || [ \"$3\" != evidence ] || [ \"$4\" != admit ] || [ -n \"${{5:-}}\" ]; then\n  printf '%s\\n' \"unexpected $1 $2 $3 $4\" >&2\n  exit 1\nfi\nn=0\nif [ -f \"$COUNT\" ]; then n=$(cat \"$COUNT\"); fi\nn=$((n + 1))\nprintf '%s\\n' \"$n\" > \"$COUNT\"\nprintf '%s\\n' admit >> \"$LOG\"\nif [ \"$n\" = 1 ]; then\n  printf '%s\\n' '{record}'\nelse\n  printf '%s\\n' '{moved}'\nfi\n"
    );
    let program = write_sim(&dir, &body);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "observation-unstable", "{detail}");
    assert_eq!(detail, "admit");
}

#[test]
fn a_sealed_word_in_program_output_is_refused() {
    let sealed = format!("{} PortableClosed", admitted());
    let (_dir, program) = prepare("sealed", &format!("printf '%s\\n' '{sealed}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "forbidden-output", "{detail}");
}

#[test]
fn a_third_line_is_refused() {
    let record = admitted();
    let body = format!(
        "printf '%s\\n' '{record}'\nprintf '%s\\n' '#(world/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/world-command\"))'\nprintf '%s\\n' extra\n"
    );
    let (_dir, program) = prepare("third", &body);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "load-receipt");
}
