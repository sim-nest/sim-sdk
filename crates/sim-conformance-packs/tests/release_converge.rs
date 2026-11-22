// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-15` runs the named program. Closure booleans are not a convergence.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PRIVATE: &str = "private-material:s3cret-token";
const LAWS: &[(&str, &str, &str)] = &[
    (
        "preserve",
        "self-approval held 1 uncertain 1 same 1 later uncertain",
        "5",
    ),
    ("invalidate", "affected a closed b changed 1", "3"),
    ("oscillate", "empty 1 seen note same 1", "5"),
    (
        "parallel",
        "same 1 failures a:proof-failed,b:proof-failed",
        "2",
    ),
    ("honest", "incomplete need malformed bad same 1", "4"),
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
        scope: "release/nv12-15",
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
        "sim-release-converge-{name}-{}",
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

fn converge(case: &str, detail: &str, invokes: &str) -> String {
    format!(
        "#(world/converge-v1 (case \"{case}\") (detail \"{detail}\") (effects 0) (invokes {invokes}) (resident true) (subprocesses 0))"
    )
}

fn private_gap() -> String {
    "#(world/converge-gap-v1 (case \"\") (effects 0) (evidence \"\") (reason \"private-leakage\") (resident true) (subprocesses 0))".to_owned()
}

fn prepare(name: &str, body: &str) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    init_checkout(&dir);
    let program = write_sim(&dir, body);
    (dir, program)
}

fn law_record(case: &str) -> String {
    let (_, detail, invokes) = LAWS.iter().find(|(name, _, _)| *name == case).unwrap();
    converge(case, detail, invokes)
}

fn dispatch(changed: Option<(&str, &str)>, private_record: Option<&str>, receipt: bool) -> String {
    let mut body = String::from(
        "if [ \"$1\" != world ] || [ \"$2\" != converge ] || [ -n \"${4:-}\" ]; then\n  printf '%s\\n' unexpected >&2\n  exit 1\nfi\ncase \"$3\" in\n",
    );
    for (case, detail, invokes) in LAWS {
        let mut record = converge(case, detail, invokes);
        if let Some((name, replacement)) = changed
            && name == *case
        {
            record = replacement.to_owned();
        }
        assert!(!record.contains('\''), "{record}");
        body.push_str(&format!(
            "  {case})\n    printf '%s\\n' {case} >> \"$LOG\"\n    printf '%s\\n' '{record}'\n"
        ));
        if receipt && *case == "preserve" {
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
fn closure_facts_do_not_execute_the_converge() {
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
fn the_named_program_supplies_the_converge_records() {
    let (dir, program) = run_dispatch("pass", None, None, false);
    let verdict = execute(&command(&program));
    for (case, _, _) in LAWS {
        let stdout = pass_value(&verdict, &format!("release.converge.{case}"));
        assert_eq!(stdout, format!("{}\n", law_record(case)));
    }
    let private_out = pass_value(&verdict, "release.converge.private");
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
    let stdout = pass_value(&verdict, "release.converge.preserve");
    assert!(stdout.starts_with(&law_record("preserve")), "{stdout}");
    assert!(stdout.contains("\n#(world/load-receipt-v1 "), "{stdout}");
}

#[test]
fn a_gap_is_not_the_preserve_specimen() {
    let gap = private_gap();
    let (_dir, program) = run_dispatch("gap", Some(("preserve", gap.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "world/converge-v1");
}

#[test]
fn a_self_approval_is_not_a_commit() {
    let applied = law_record("preserve").replace("self-approval held", "self-approval applied");
    assert!(applied.contains("self-approval applied"), "{applied}");
    let (_dir, program) = run_dispatch(
        "approval",
        Some(("preserve", applied.as_str())),
        None,
        false,
    );
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "detail");
}

#[test]
fn an_unrelated_claim_stays_closed() {
    let opened = law_record("invalidate").replace("closed b", "open b");
    let (_dir, program) =
        run_dispatch("closed", Some(("invalidate", opened.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "detail");
}

#[test]
fn an_oscillation_without_its_witness_is_refused() {
    let hidden = law_record("oscillate").replace("seen note", "seen other");
    assert!(hidden.contains("seen other"), "{hidden}");
    let (_dir, program) =
        run_dispatch("witness", Some(("oscillate", hidden.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "detail");
}

#[test]
fn a_prefixed_case_is_refused() {
    let prefixed =
        law_record("preserve").replace("(case \"preserve\")", "(case \"preserve-other\")");
    let (_dir, program) = run_dispatch("case", Some(("preserve", prefixed.as_str())), None, false);
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
    let swapped = law_record("honest").replace(
        "(case \"honest\") (detail \"incomplete need malformed bad same 1\")",
        "(detail \"incomplete need malformed bad same 1\") (case \"honest\")",
    );
    let (_dir, program) = run_dispatch("swap", Some(("honest", swapped.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "fields");
}

#[test]
fn an_extra_field_is_refused() {
    let extra = law_record("honest").replace(
        "(subprocesses 0))",
        "(subprocesses 0) (policy \"policy/other\"))",
    );
    let (_dir, program) = run_dispatch("extra", Some(("honest", extra.as_str())), None, false);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "fields");
}

#[test]
fn a_quoted_invoke_count_is_refused() {
    let quoted = law_record("honest").replace("(invokes 4)", "(invokes \"4\")");
    let (_dir, program) = run_dispatch("invokes", Some(("honest", quoted.as_str())), None, false);
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
fn a_changed_second_preserve_is_refused() {
    let first = law_record("preserve");
    let moved = first.replace("(invokes 5)", "(invokes 6)");
    let dir = scratch("unstable");
    init_checkout(&dir);
    let log = shell_quote(&dir.join("ran.log"));
    let count = shell_quote(&dir.join("count"));
    let mut body = format!(
        "LOG={log}\nCOUNT={count}\nif [ \"$1\" != world ] || [ \"$2\" != converge ] || [ -n \"${{4:-}}\" ]; then\n  printf '%s\\n' unexpected >&2\n  exit 1\nfi\ncase \"$3\" in\n  preserve)\n    n=0\n    if [ -f \"$COUNT\" ]; then n=$(cat \"$COUNT\"); fi\n    n=$((n + 1))\n    printf '%s\\n' \"$n\" > \"$COUNT\"\n    printf '%s\\n' preserve >> \"$LOG\"\n    if [ \"$n\" = 1 ]; then\n      printf '%s\\n' '{first}'\n    else\n      printf '%s\\n' '{moved}'\n    fi\n    ;;\n"
    );
    for (case, detail, invokes) in LAWS.iter().skip(1) {
        let record = converge(case, detail, invokes);
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
    assert_eq!(detail, "preserve");
}

#[test]
fn a_sealed_word_in_program_output_is_refused() {
    let sealed = format!("{} PortableClosed", law_record("preserve"));
    let (_dir, program) = prepare("sealed", &format!("printf '%s\\n' '{sealed}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "forbidden-output", "{detail}");
}
