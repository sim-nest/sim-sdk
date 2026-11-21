// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-10` runs the named program. Closure booleans are not a seal.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const MEASURE: &str = "#(world/seal-measure-v1 (compile \"checked\") (corrupt \"corrupt-input\") (effects 0) (lock \"moved\") (missing \"missing-input\") (resident true) (seal \"same\") (stub \"stub-only\") (subprocesses 1) (target \"wrong-target\"))";

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
        scope: "release/nv12-10",
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
    let dir = std::env::temp_dir().join(format!("sim-release-seal-{name}-{}", std::process::id()));
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

fn stub(dir: &Path, record: &str, second: Option<&str>, receipt: bool) -> PathBuf {
    assert!(!record.contains('\''), "{record}");
    if let Some(second) = second {
        assert!(!second.contains('\''), "{second}");
    }
    let log = shell_quote(&dir.join("ran.log"));
    let count = shell_quote(&dir.join("count"));
    let mut body = format!(
        "log={log}\ncount={count}\nprintf '%s\\n' \"BEGIN $1 $2 $3\" >> \"$log\"\nif [ \"$1\" != world ] || [ \"$2\" != measure ] || [ \"$3\" != seal ]; then\n  printf '%s\\n' \"unexpected $1 $2 $3\" >&2\n  exit 1\nfi\n"
    );
    if let Some(second) = second {
        body.push_str("n=0\nif [ -f \"$count\" ]; then n=$(cat \"$count\"); fi\nn=$((n + 1))\nprintf '%s\\n' \"$n\" > \"$count\"\nif [ \"$n\" = 1 ]; then\n  printf '%s\\n' '");
        body.push_str(record);
        body.push_str("'\nelse\n  printf '%s\\n' '");
        body.push_str(second);
        body.push_str("'\nfi\n");
    } else {
        body.push_str("printf '%s\\n' '");
        body.push_str(record);
        body.push_str("'\n");
    }
    if receipt {
        body.push_str(
            "printf '%s\\n' '#(world/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/world-command\"))'\n",
        );
    }
    write_sim(dir, &body)
}

fn prepare(name: &str, record: &str, second: Option<&str>, receipt: bool) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    init_checkout(&dir);
    let program = stub(&dir, record, second, receipt);
    (dir, program)
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

#[test]
fn closure_facts_do_not_execute_the_measure() {
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
fn the_named_program_supplies_the_seal_record() {
    let (dir, program) = prepare("pass", MEASURE, None, false);
    let verdict = execute(&command(&program));
    assert_eq!(
        pass_value(&verdict, "release.observe.seal").trim_end(),
        MEASURE
    );
    let log = std::fs::read_to_string(dir.join("ran.log")).unwrap();
    assert_eq!(
        log.matches("BEGIN world measure seal\n").count(),
        2,
        "{log}"
    );
    assert!(!log.contains("snapshot"), "{log}");
}

#[test]
fn a_load_receipt_stays_outside_the_measure() {
    let (_dir, program) = prepare("receipt", MEASURE, None, true);
    let verdict = execute(&command(&program));
    let value = pass_value(&verdict, "release.observe.seal");
    assert!(value.contains(MEASURE), "{value}");
    assert!(value.contains("#(world/load-receipt-v1 "), "{value}");
}

#[test]
fn a_gap_record_is_not_the_measure() {
    let gap = "#(world/seal-measure-gap-v1 (effects 0) (reason \"missing-input\") (resident true) (subprocesses 0))";
    let (_dir, program) = prepare("gap", gap, None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "world/seal-measure-v1");
}

#[test]
fn a_quiet_compiler_is_refused() {
    let quiet = MEASURE.replace("(subprocesses 1)", "(subprocesses 0)");
    let (_dir, program) = prepare("quiet", &quiet, None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "subprocesses");
}

#[test]
fn a_moved_seal_is_refused() {
    let moved = MEASURE.replace("(seal \"same\")", "(seal \"moved\")");
    let (_dir, program) = prepare("moved", &moved, None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "seal");
}

#[test]
fn an_unchanged_lock_is_refused() {
    let same = MEASURE.replace("(lock \"moved\")", "(lock \"same\")");
    let (_dir, program) = prepare("lock", &same, None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "lock");
}

#[test]
fn an_extra_field_is_refused() {
    let extra = MEASURE.replace(
        "(target \"wrong-target\"))",
        "(target \"wrong-target\") (alias \"resolver/other\"))",
    );
    let (_dir, program) = prepare("extra", &extra, None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "fields");
}

#[test]
fn a_changed_second_measure_is_refused() {
    let second = MEASURE.replace("(compile \"checked\")", "(compile \"refused\")");
    let (_dir, program) = prepare("unstable", MEASURE, Some(&second), false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "observation-unstable", "{detail}");
    assert_eq!(detail, "seal");
}

#[test]
fn a_sealed_word_in_program_output_is_refused() {
    let sealed = format!("{MEASURE} PortableClosed");
    let (_dir, program) = prepare("sealed", &sealed, None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "forbidden-output", "{detail}");
}
