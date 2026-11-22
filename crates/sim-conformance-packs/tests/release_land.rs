// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-18` runs the named program. Closure booleans are not a landing.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PRIVATE: &str = "private-material:s3cret-token";
const LAWS: &[(&str, &str, &str)] = &[
    ("foreign", "would-commit applied kept same 1", "3"),
    ("replay", "already-true same 1", "2"),
    ("recover", "restored exact kept 1", "3"),
    ("generated", "generated-owner-bypass same 1", "2"),
    ("cache", "unknown current stale same 1", "6"),
    ("status", "present missing same 1", "2"),
];

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
        scope: "release/nv12-18",
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
    let dir = std::env::temp_dir().join(format!("sim-release-land-{name}-{}", std::process::id()));
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

fn record(case: &str, detail: &str, invokes: &str) -> String {
    format!(
        "#(world/land-v1 (case \"{case}\") (detail \"{detail}\") (effects 0) (invokes {invokes}) (resident true) (subprocesses 0))"
    )
}

fn private_gap() -> String {
    "#(world/land-gap-v1 (case \"\") (effects 0) (evidence \"\") (reason \"private-leakage\") (resident true) (subprocesses 0))".to_owned()
}

fn prepare(name: &str, body: &str) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    init_checkout(&dir);
    let program = write_sim(&dir, body);
    (dir, program)
}

fn law_record(case: &str) -> String {
    let (_, detail, invokes) = LAWS.iter().find(|(name, _, _)| *name == case).unwrap();
    record(case, detail, invokes)
}

fn dispatch(changed: Option<(&str, &str)>, private_record: Option<&str>, receipt: bool) -> String {
    let mut body = String::from(
        "if [ \"$1\" != world ] || [ \"$2\" != land ] || [ -n \"${4:-}\" ]; then\n  printf '%s\\n' unexpected >&2\n  exit 1\nfi\ncase \"$3\" in\n",
    );
    for (case, detail, invokes) in LAWS {
        let mut record = record(case, detail, invokes);
        if let Some((name, replacement)) = changed
            && name == *case
        {
            record = replacement.to_owned();
        }
        assert!(!record.contains('\''), "{record}");
        body.push_str(&format!(
            "  {case})\n    printf '%s\\n' {case} >> \"$LOG\"\n    printf '%s\\n' '{record}'\n"
        ));
        if receipt && *case == "foreign" {
            body.push_str(
                "    printf '%s\\n' '#(world/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/world-command\"))'\n",
            );
        }
        body.push_str("    ;;\n");
    }
    let gap = private_record.unwrap_or(&private_gap()).to_owned();
    assert!(!gap.contains('\''), "{gap}");
    body.push_str(&format!(
        "  {PRIVATE})\n    printf '%s\\n' private >> \"$LOG\"\n    printf '%s\\n' '{gap}'\n    ;;\n  *)\n    printf '%s\\n' unexpected >&2\n    exit 1\n    ;;\nesac\n"
    ));
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

fn run_dispatch(
    name: &str,
    changed: Option<(&str, &str)>,
    private_record: Option<&str>,
    receipt: bool,
) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    init_checkout(&dir);
    let program = write_sim(
        &dir,
        &with_log(&dir, &dispatch(changed, private_record, receipt)),
    );
    (dir, program)
}

#[test]
fn closure_facts_do_not_execute_the_landing() {
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
fn the_named_program_supplies_the_landing_records() {
    let (dir, program) = run_dispatch("pass", None, None, false);
    let verdict = execute(&command(&program));
    for (case, _, _) in LAWS {
        let stdout = pass_value(&verdict, &format!("release.land.{case}"));
        assert_eq!(stdout, format!("{}\n", law_record(case)));
    }
    let private_out = pass_value(&verdict, "release.land.private");
    assert_eq!(private_out, format!("{}\n", private_gap()));
    assert!(!private_out.contains(PRIVATE), "{private_out}");
    let log = std::fs::read_to_string(dir.join("ran.log")).unwrap();
    for (case, _, _) in LAWS {
        assert_eq!(log.matches(&format!("{case}\n")).count(), 2, "{log}");
    }
    assert_eq!(log.matches("private\n").count(), 2, "{log}");
}

#[test]
fn a_load_receipt_stays_outside_the_record() {
    let (_dir, program) = run_dispatch("receipt", None, None, true);
    let verdict = execute(&command(&program));
    let stdout = pass_value(&verdict, "release.land.foreign");
    assert!(stdout.starts_with(&law_record("foreign")), "{stdout}");
    assert!(stdout.contains("\n#(world/load-receipt-v1 "), "{stdout}");
}

#[test]
fn a_gap_is_not_the_foreign_specimen() {
    let gap = private_gap();
    let (_dir, program) = run_dispatch("gap-tag", Some(("foreign", gap.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "world/land-v1");
}

#[test]
fn a_foreign_edit_must_survive() {
    let dropped = law_record("foreign").replace("kept", "dropped");
    assert!(dropped.contains("dropped"), "{dropped}");
    let (_dir, program) = run_dispatch("foreign", Some(("foreign", dropped.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "detail");
}

#[test]
fn a_replay_does_not_apply() {
    let applied = law_record("replay").replace("already-true", "applied");
    assert!(applied.contains("applied"), "{applied}");
    let (_dir, program) = run_dispatch("replay", Some(("replay", applied.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "detail");
}

#[test]
fn a_recovery_restores_the_exact_commit() {
    let drifted = law_record("recover").replace("exact", "approx");
    assert!(drifted.contains("approx"), "{drifted}");
    let (_dir, program) = run_dispatch("recover", Some(("recover", drifted.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "detail");
}

#[test]
fn a_generated_region_stays_refused() {
    let allowed = law_record("generated").replace("generated-owner-bypass", "owner-allowed");
    assert!(allowed.contains("owner-allowed"), "{allowed}");
    let (_dir, program) = run_dispatch(
        "generated",
        Some(("generated", allowed.as_str())),
        None,
        false,
    );
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "detail");
}

#[test]
fn a_cached_answer_names_stale() {
    let fresh = law_record("cache").replace("stale", "fresh");
    assert!(fresh.contains("fresh"), "{fresh}");
    let (_dir, program) = run_dispatch("cache", Some(("cache", fresh.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "detail");
}

#[test]
fn a_status_names_a_missing_repository() {
    let absent = law_record("status").replace("missing", "absent");
    assert!(absent.contains("absent"), "{absent}");
    let (_dir, program) = run_dispatch("status", Some(("status", absent.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "detail");
}

#[test]
fn a_prefixed_case_is_refused() {
    let prefixed = law_record("foreign").replace("(case \"foreign\")", "(case \"foreign-other\")");
    let (_dir, program) = run_dispatch("case", Some(("foreign", prefixed.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "case");
}

#[test]
fn a_blank_word_is_not_an_empty_case() {
    let blank = private_gap().replace("(case \"\")", "(case \"blank\")");
    let (_dir, program) = run_dispatch("blank", None, Some(blank.as_str()), false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "case");
}

#[test]
fn a_swapped_field_order_is_refused() {
    let swapped = law_record("status").replace(
        "(case \"status\") (detail \"present missing same 1\")",
        "(detail \"present missing same 1\") (case \"status\")",
    );
    let (_dir, program) = run_dispatch("swap", Some(("status", swapped.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "fields");
}

#[test]
fn an_extra_field_is_refused() {
    let extra = law_record("status").replace(
        "(subprocesses 0))",
        "(subprocesses 0) (policy \"policy/other\"))",
    );
    let (_dir, program) = run_dispatch("extra", Some(("status", extra.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "fields");
}

#[test]
fn a_quoted_invoke_count_is_refused() {
    let quoted = law_record("replay").replace("(invokes 2)", "(invokes \"2\")");
    let (_dir, program) = run_dispatch("invokes", Some(("replay", quoted.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "invokes");
}

#[test]
fn a_private_echo_is_refused() {
    let body = format!("printf '%s\\n' '{PRIVATE}'\n");
    let (_dir, program) = prepare("echo", &body);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "forbidden-output", "{detail}");
    assert_eq!(detail, "private");
}

#[test]
fn a_changed_second_foreign_is_refused() {
    let first = law_record("foreign");
    let moved = first.replace("(invokes 3)", "(invokes 4)");
    let dir = scratch("unstable");
    init_checkout(&dir);
    let log = shell_quote(&dir.join("ran.log"));
    let count = shell_quote(&dir.join("count"));
    let mut body = format!(
        "LOG={log}\nCOUNT={count}\nif [ \"$1\" != world ] || [ \"$2\" != land ] || [ -n \"${{4:-}}\" ]; then\n  printf '%s\\n' unexpected >&2\n  exit 1\nfi\ncase \"$3\" in\n  foreign)\n    n=0\n    if [ -f \"$COUNT\" ]; then n=$(cat \"$COUNT\"); fi\n    n=$((n + 1))\n    printf '%s\\n' \"$n\" > \"$COUNT\"\n    printf '%s\\n' foreign >> \"$LOG\"\n    if [ \"$n\" = 1 ]; then\n      printf '%s\\n' '{first}'\n    else\n      printf '%s\\n' '{moved}'\n    fi\n    ;;\n"
    );
    for (case, detail, invokes) in LAWS.iter().skip(1) {
        let record = record(case, detail, invokes);
        body.push_str(&format!(
            "  {case})\n    printf '%s\\n' {case} >> \"$LOG\"\n    printf '%s\\n' '{record}'\n    ;;\n"
        ));
    }
    let gap = private_gap();
    body.push_str(&format!(
        "  {PRIVATE})\n    printf '%s\\n' private >> \"$LOG\"\n    printf '%s\\n' '{gap}'\n    ;;\n  *)\n    exit 1\n    ;;\nesac\n"
    ));
    let program = write_sim(&dir, &body);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "observation-unstable", "{detail}");
    assert_eq!(detail, "foreign");
}

#[test]
fn a_sealed_word_in_program_output_is_refused() {
    let sealed = format!("{} PortableClosed", law_record("foreign"));
    let (_dir, program) = prepare("sealed", &format!("printf '%s\\n' '{sealed}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "forbidden-output", "{detail}");
}
