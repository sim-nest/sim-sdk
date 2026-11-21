// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-11` runs the named program. Closure booleans are not a close.

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
        scope: "release/nv12-11",
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
    let dir =
        std::env::temp_dir().join(format!("sim-release-foreign-{name}-{}", std::process::id()));
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

fn closed(project: &str) -> String {
    let preimage = "a".repeat(64);
    let postimage = "b".repeat(64);
    let change_id = "c".repeat(64);
    format!(
        "#(world/succession-v1 (change \"shade-1\") (change-id \"{change_id}\") (closed \"portable\") (converge-first 1) (converge-second 0) (deliver-first 1) (deliver-second 0) (effects 0) (holds 0) (input-ids [\"{change_id}\" \"{postimage}\"]) (land 1) (linux \"absent\") (observed 3) (origin \"registry\") (postimage \"{postimage}\") (preimage \"{preimage}\") (project \"{project}\") (proved 3) (record \"change shade-1; project {project}; origin registry; closed portable\") (recovery \"match\") (recovery-reads 2) (resident true) (seal 2) (subprocesses 0))"
    )
}

fn gap() -> String {
    "#(world/succession-gap-v1 (closed \"open\") (effects 0) (reason \"private-branch\") (resident true) (subprocesses 0))".to_owned()
}

fn prepare(name: &str, body: &str) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    init_checkout(&dir);
    let program = write_sim(&dir, body);
    (dir, program)
}

fn dispatch(lantern: &str, harbor: &str, branch: &str, receipt: bool) -> String {
    assert!(!lantern.contains('\''));
    assert!(!harbor.contains('\''));
    assert!(!branch.contains('\''));
    let mut body = String::from(
        "if [ \"$1\" != world ] || [ \"$2\" != succeed ]; then\n  printf '%s\\n' \"unexpected $1 $2\" >&2\n  exit 1\nfi\ncase \"$4\" in\n  shade-1\\ *) ;;\n  *) printf '%s\\n' 'unexpected change' >&2; exit 1 ;;\nesac\n",
    );
    body.push_str(
        "if printf '%s' \"$3\" | grep -q 'branch private'; then\n  kind=branch\n  printf '%s\\n' '",
    );
    body.push_str(branch);
    body.push_str("'\nelif printf '%s' \"$3\" | grep -q 'project harbor'; then\n  kind=harbor\n  printf '%s\\n' '");
    body.push_str(harbor);
    body.push_str("'\nelif printf '%s' \"$3\" | grep -q 'project lantern'; then\n  kind=lantern\n  printf '%s\\n' '");
    body.push_str(lantern);
    body.push_str("'\nelse\n  printf '%s\\n' 'unexpected export' >&2\n  exit 1\nfi\n");
    if receipt {
        body.push_str(
            "printf '%s\\n' '#(world/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/world-command\"))'\n",
        );
    }
    body.push_str("printf '%s\\n' \"$kind\" >> \"$LOG\"\n");
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
fn closure_facts_do_not_execute_the_foreign_project() {
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
fn the_named_program_supplies_the_foreign_records() {
    let lantern = closed("lantern");
    let harbor = closed("harbor");
    let branch = gap();
    let dir = scratch("pass");
    init_checkout(&dir);
    let body = with_log(&dir, &dispatch(&lantern, &harbor, &branch, false));
    let program = write_sim(&dir, &body);
    let verdict = execute(&command(&program));
    let lantern_out = pass_value(&verdict, "release.foreign.lantern");
    let harbor_out = pass_value(&verdict, "release.foreign.harbor");
    let branch_out = pass_value(&verdict, "release.foreign.branch");
    assert!(lantern_out.contains("project \"lantern\""), "{lantern_out}");
    assert!(harbor_out.contains("project \"harbor\""), "{harbor_out}");
    assert!(branch_out.contains("private-branch"), "{branch_out}");
    let log = std::fs::read_to_string(dir.join("ran.log")).unwrap();
    assert_eq!(log.matches("lantern\n").count(), 2, "{log}");
    assert_eq!(log.matches("harbor\n").count(), 2, "{log}");
    assert_eq!(log.matches("branch\n").count(), 2, "{log}");
}

#[test]
fn a_load_receipt_stays_outside_the_record() {
    let lantern = closed("lantern");
    let harbor = closed("harbor");
    let branch = gap();
    let dir = scratch("receipt");
    init_checkout(&dir);
    let body = with_log(&dir, &dispatch(&lantern, &harbor, &branch, true));
    let program = write_sim(&dir, &body);
    let verdict = execute(&command(&program));
    let lantern_out = pass_value(&verdict, "release.foreign.lantern");
    assert!(
        lantern_out.contains("#(world/load-receipt-v1 "),
        "{lantern_out}"
    );
}

#[test]
fn a_gap_is_not_the_specimen() {
    let gap = gap();
    let (_dir, program) = prepare("gap", &format!("printf '%s\\n' '{gap}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "world/succession-v1");
}

#[test]
fn a_private_branch_that_closes_is_refused() {
    let lantern = closed("lantern");
    let harbor = closed("harbor");
    let dir = scratch("branch");
    init_checkout(&dir);
    let body = with_log(&dir, &dispatch(&lantern, &harbor, &lantern, false));
    let program = write_sim(&dir, &body);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "world/succession-gap-v1");
}

#[test]
fn a_harbor_named_lantern_is_refused() {
    let lantern = closed("lantern");
    let branch = gap();
    let dir = scratch("harbor");
    init_checkout(&dir);
    let body = with_log(&dir, &dispatch(&lantern, &lantern, &branch, false));
    let program = write_sim(&dir, &body);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "project");
}

#[test]
fn an_extra_field_is_refused() {
    let extra = closed("lantern").replace(
        "(subprocesses 0))",
        "(subprocesses 0) (alias \"resolver/other\"))",
    );
    let (_dir, program) = prepare("extra", &format!("printf '%s\\n' '{extra}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "fields");
}

#[test]
fn empty_input_ids_are_refused() {
    let populated = format!(
        "(input-ids [\"{}\" \"{}\"])",
        "c".repeat(64),
        "b".repeat(64)
    );
    let empty = closed("lantern").replace(&populated, "(input-ids [])");
    assert!(empty.contains("(input-ids [])"), "{empty}");
    let (_dir, program) = prepare("ids", &format!("printf '%s\\n' '{empty}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "input-ids");
}

#[test]
fn a_present_linux_claim_is_refused() {
    let present = closed("lantern").replace("(linux \"absent\")", "(linux \"present\")");
    let (_dir, program) = prepare("linux", &format!("printf '%s\\n' '{present}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "linux");
}

#[test]
fn a_changed_second_succeed_is_refused() {
    let lantern = closed("lantern");
    let moved = lantern.replace("(closed \"portable\")", "(closed \"open\")");
    let harbor = closed("harbor");
    let branch = gap();
    let dir = scratch("unstable");
    init_checkout(&dir);
    let log = shell_quote(&dir.join("ran.log"));
    let count = shell_quote(&dir.join("count"));
    let body = format!(
        "LOG={log}\nCOUNT={count}\nif [ \"$1\" != world ] || [ \"$2\" != succeed ]; then\n  printf '%s\\n' \"unexpected $1 $2\" >&2\n  exit 1\nfi\nif printf '%s' \"$3\" | grep -q 'branch private'; then\n  printf '%s\\n' branch >> \"$LOG\"\n  printf '%s\\n' '{branch}'\nelif printf '%s' \"$3\" | grep -q 'project harbor'; then\n  printf '%s\\n' harbor >> \"$LOG\"\n  printf '%s\\n' '{harbor}'\nelif printf '%s' \"$3\" | grep -q 'project lantern'; then\n  n=0\n  if [ -f \"$COUNT\" ]; then n=$(cat \"$COUNT\"); fi\n  n=$((n + 1))\n  printf '%s\\n' \"$n\" > \"$COUNT\"\n  printf '%s\\n' lantern >> \"$LOG\"\n  if [ \"$n\" = 1 ]; then\n    printf '%s\\n' '{lantern}'\n  else\n    printf '%s\\n' '{moved}'\n  fi\nelse\n  printf '%s\\n' 'unexpected export' >&2\n  exit 1\nfi\n"
    );
    let program = write_sim(&dir, &body);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "observation-unstable", "{detail}");
    assert_eq!(detail, "lantern");
}

#[test]
fn a_sealed_word_in_program_output_is_refused() {
    let sealed = format!("{} PortableClosed", closed("lantern"));
    let (_dir, program) = prepare("sealed", &format!("printf '%s\\n' '{sealed}'\n"));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "forbidden-output", "{detail}");
}
