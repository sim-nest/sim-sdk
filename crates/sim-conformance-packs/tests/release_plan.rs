// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-08` runs the named program. Closure booleans are not a plan.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const RECEIPT: &str = "#(prove/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/prove-command\"))";
const PLAN: &str = "#(prove/plan-v1 (denominator 29) (page 0) (pages 1) (leaves [\"leaf/portable\"]) (excluded [\"leaf/native-worker-adapter\" \"leaf/native-assay\"]) (effects 0) (subprocesses 0) (owner-commands 0) (resident true))";
const WHY: &str = "#(world/explanation-v1 (path [\"owner/sim-private/disclosure\"]))";

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
        scope: "release/nv12-08",
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
    let dir = std::env::temp_dir().join(format!("sim-release-plan-{name}-{}", std::process::id()));
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

fn quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
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

fn stub(dir: &Path, first_plan: &str, second_plan: &str) -> PathBuf {
    let log = quote(&dir.join("ran.log"));
    let count = quote(&dir.join("count"));
    let body = format!(
        r#"printf '%s\n' "$*" >> {log}
cmd="$1"
sub="$2"
case "$cmd:$sub" in
prove:plan)
  n=0
  if [ -f {count} ]; then n=$(cat {count}); fi
  n=$((n + 1))
  printf '%s\n' "$n" > {count}
  if [ "$n" = 1 ]; then
    printf '%s\n' '{first_plan}'
  else
    printf '%s\n' '{second_plan}'
  fi
  printf '%s\n' '{RECEIPT}'
  ;;
world:why)
  printf '%s\n' '{WHY}'
  ;;
*)
  printf '%s\n' "unexpected $*" >&2
  exit 1
  ;;
esac
"#
    );
    write_sim(dir, &body)
}

#[test]
fn closure_facts_do_not_execute_the_plan() {
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
        MemorySubject::default().with(packs::product::COMMAND_FACT, "completion/claim-08");
    let (code, _) = refusal(&execute(&relabelled));
    assert_eq!(code, "claim-relabelled");
}

#[test]
fn the_named_program_supplies_the_plan_and_the_explanation() {
    let dir = scratch("pass");
    init_checkout(&dir);
    let program = stub(&dir, PLAN, PLAN);
    let verdict = execute(&command(&program));
    let plan = pass_value(&verdict, "release.plan");
    let why = pass_value(&verdict, "release.why");
    assert!(plan.contains(PLAN), "{plan}");
    assert!(plan.contains(RECEIPT), "{plan}");
    assert_eq!(why.trim_end(), WHY);
    assert_ne!(plan, "true");
    assert_ne!(why, "true");
    let log = std::fs::read_to_string(dir.join("ran.log")).unwrap();
    assert_eq!(log.matches("prove plan").count(), 2, "{log}");
    assert_eq!(log.matches("world why").count(), 1, "{log}");
    assert!(
        log.contains("world why conclusion/public-release policy/no-v3-disclosure"),
        "{log}"
    );
}

#[test]
fn a_changed_second_plan_is_refused() {
    let dir = scratch("unstable");
    init_checkout(&dir);
    let other = PLAN.replace("(denominator 29)", "(denominator 28)");
    let program = stub(&dir, PLAN, &other);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "plan-unstable", "{detail}");
}

#[test]
fn a_plan_without_a_zero_effect_count_is_refused() {
    let dir = scratch("effects");
    init_checkout(&dir);
    let other = PLAN.replace("(effects 0)", "(effects 1)");
    let program = stub(&dir, &other, &other);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "effects");
}

#[test]
fn a_plan_missing_a_native_leaf_is_refused() {
    let dir = scratch("leaf");
    init_checkout(&dir);
    let other = PLAN.replace("leaf/native-assay", "leaf/other");
    let program = stub(&dir, &other, &other);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "leaf/native-assay");
}

#[test]
fn a_longer_leaf_name_is_not_the_required_leaf() {
    let dir = scratch("prefix");
    init_checkout(&dir);
    let other = PLAN.replace("\"leaf/native-assay\"", "\"leaf/native-assay-disabled\"");
    let program = stub(&dir, &other, &other);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "leaf/native-assay");
}

#[test]
fn a_sealed_word_in_program_output_is_refused() {
    let dir = scratch("sealed");
    init_checkout(&dir);
    let sealed = format!("{PLAN} PortableClosed");
    let program = stub(&dir, &sealed, &sealed);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "forbidden-output", "{detail}");
}
