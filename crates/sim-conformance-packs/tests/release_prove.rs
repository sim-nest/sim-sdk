// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-14` runs the named prove program. Closure booleans are not a comparison.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const VIEW: &str = "sim-world-proof-view-v1;revision aabbccddeeff00112233445566778899";
const RECEIPT: &str = "#(prove/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/prove-command\"))";
const PLAN_TOKEN: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const NEXT_TOKEN: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const FRONTIER_TOKEN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    UnionBasis,
    ColdExport,
    MissingReceipt,
    OpenFrontier,
    NativeReason,
}

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
        scope: "release/nv12-14",
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
    let dir = std::env::temp_dir().join(format!("sim-release-prove-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
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

fn write_sim(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("sim");
    std::fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}")).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn prepare(name: &str, fault: Fault) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    init_checkout(&dir);
    let log = dir.join("ran.log");
    let program = write_sim(&dir, &script(&log, fault));
    (dir, program)
}

fn leaf(index: u8) -> String {
    format!("conclusion/{index:02}")
}

fn listed(names: &[String]) -> String {
    format!(
        "[{}]",
        names
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn complement(rerun: &[&str]) -> Vec<String> {
    (1..=40)
        .map(leaf)
        .filter(|name| !rerun.contains(&name.as_str()))
        .collect()
}

fn page(start: u8) -> Vec<String> {
    (start..start + 8).map(leaf).collect()
}

fn proof_record() -> String {
    format!(
        "#(prove/proof-v1 (denominator 40) (executed 40) (requests 11) (edges 3) (effects 0) (subprocesses 0) (owner-commands 0) (resident true) (subject \"subject-seal\") (failures []) (view \"{VIEW}\"))"
    )
}

fn cold_record() -> String {
    "#(prove/cold-v1 (denominator 29) (executed 29) (requests 1) (edges 1) (effects 0) (subprocesses 0) (owner-commands 0) (reuse-disabled true) (resident true) (subject \"union\") (excluded []) (gaps []) (view \"sim-world-portable-union-v1\"))".to_owned()
}

fn reuse_record(delta: &str, basis: u16, requests: u64, revision: u64, rerun: &[&str]) -> String {
    let rerun_names: Vec<String> = rerun.iter().map(|name| (*name).to_owned()).collect();
    let reused = complement(rerun);
    format!(
        "#(prove/reuse-v1 (delta \"{delta}\") (denominator 40) (executed 40) (reused {}) (reused-leaves {}) (rerun {}) (reuse-basis-points {basis}) (false-reuse 0) (cold-requests 11) (reuse-requests {requests}) (revision-delta {revision}) (effects 0) (subprocesses 0) (owner-commands 0) (reuse-disabled false) (gaps []))",
        reused.len(),
        listed(&reused),
        listed(&rerun_names),
    )
}

fn plan_record(page_no: u64, leaves: &[String], token: &str) -> String {
    format!(
        "#(prove/plan-v1 (denominator 40) (page {page_no}) (pages 5) (leaves {}) (excluded [\"leaf/native-worker-adapter\" \"leaf/native-assay\"]) (effects 0) (subprocesses 0) (owner-commands 0) (resident true) (continuation \"{token}\"))",
        listed(leaves),
    )
}

fn status_record() -> String {
    "#(prove/status-v1 (denominator 40) (executed 40) (excluded [\"leaf/native-worker-adapter\" \"leaf/native-assay\"]) (requests 2) (effects 0) (subprocesses 0) (owner-commands 0) (resident true) (gaps []))".to_owned()
}

fn why_record(leaf_name: &str, reused: bool, exercised: &str) -> String {
    let reused = if reused { "true" } else { "false" };
    format!(
        "#(prove/why-v1 (delta \"D2\") (leaf \"{leaf_name}\") (reused {reused}) (false-reuse false) (exercised {exercised}) (digest \"abc\") (requests 4) (effects 0) (subprocesses 0))"
    )
}

fn frontier_record() -> String {
    format!(
        "#(prove/frontier-v1 (complete true) (failures []) (gaps []) (denominator 40) (executed 40) (requests 4) (effects 0) (subprocesses 0) (continuation \"{FRONTIER_TOKEN}\"))"
    )
}

fn gap_record(call: &str, reason: &str) -> String {
    format!("#(prove/gap-v1 (call \"{call}\") (reason \"{reason}\") (effects 0) (subprocesses 0))")
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

fn print_record(record: &str, receipt: bool) -> String {
    assert!(!record.contains('\''), "{record}");
    let mut body = format!("    printf '%s\\n' {}\n", shell_quote(record));
    if receipt {
        body.push_str(&format!("    printf '%s\\n' {}\n", shell_quote(RECEIPT)));
    }
    body
}

fn law_record(delta: &str, fault: Fault) -> String {
    let basis = if fault == Fault::UnionBasis && delta == "D2" {
        10_000
    } else {
        match delta {
            "D1" | "D6" => 10_000,
            "D2" => 9_500,
            "D3" => 8_000,
            "D4" => 7_000,
            "D5" => 9_000,
            _ => panic!("delta"),
        }
    };
    let (requests, revision, rerun): (u64, u64, &[&str]) = match delta {
        "D1" => (0, 0, &[]),
        "D2" => (4, 0, &["conclusion/01", "conclusion/02"]),
        "D3" => (
            16,
            0,
            &[
                "conclusion/03",
                "conclusion/04",
                "conclusion/05",
                "conclusion/06",
                "conclusion/07",
                "conclusion/08",
                "conclusion/09",
                "conclusion/10",
            ],
        ),
        "D4" => (
            24,
            0,
            &[
                "conclusion/11",
                "conclusion/12",
                "conclusion/13",
                "conclusion/14",
                "conclusion/15",
                "conclusion/16",
                "conclusion/17",
                "conclusion/18",
                "conclusion/19",
                "conclusion/20",
                "conclusion/21",
                "conclusion/22",
            ],
        ),
        "D5" => (
            8,
            0,
            &[
                "conclusion/23",
                "conclusion/24",
                "conclusion/25",
                "conclusion/26",
            ],
        ),
        "D6" => (0, 1, &[]),
        _ => panic!("delta"),
    };
    reuse_record(delta, basis, requests, revision, rerun)
}

fn script(log: &Path, fault: Fault) -> String {
    let log = shell_quote(&log.display().to_string());
    let view = shell_quote(VIEW);
    let receipt = fault != Fault::MissingReceipt;
    let proof = if fault == Fault::ColdExport {
        cold_record()
    } else {
        proof_record()
    };
    let unavailable = if fault == Fault::OpenFrontier {
        frontier_record()
    } else {
        gap_record("conclusion/41", "unavailable-proof")
    };
    let native = if fault == Fault::NativeReason {
        gap_record("leaf/native-worker-adapter", "os 0")
    } else {
        gap_record("leaf/native-worker-adapter", "excluded-scope")
    };
    let assay = gap_record("leaf/native-assay", "excluded-scope");
    let mut runs = String::new();
    for delta in ["D1", "D2", "D3", "D4", "D5", "D6"] {
        let record = law_record(delta, fault);
        runs.push_str(&format!(
            "      {delta})\n        [ \"$#\" -eq 4 ] || bad\n        same \"$4\"\n        note \"prove run {delta} $4\"\n{body}        ;;\n",
            delta = delta,
            body = print_record(&record, receipt),
        ));
    }
    format!(
        r#"LOG={log}
VIEW={view}
bad() {{ printf '%s\n' unexpected >&2; exit 1; }}
note() {{ printf '%s\n' "$1" >> "$LOG"; }}
same() {{
  if [ "$1" != "$VIEW" ]; then
    note "mismatch"
    {gap}
    exit 0
  fi
}}
if [ "$1" != prove ]; then bad; fi
case "$2" in
  run)
    case "$3" in
      proof)
        [ "$#" -eq 3 ] || bad
        note "prove run proof"
{proof}
        ;;
{runs}      *)
        bad
        ;;
    esac
    ;;
  plan)
    [ "$#" -eq 3 ] || bad
    same "$3"
    note "prove plan $3"
{plan}
    ;;
  status)
    [ "$#" -eq 3 ] || bad
    same "$3"
    note "prove status $3"
{status}
    ;;
  why)
    [ "$#" -eq 5 ] || bad
    same "$5"
    note "prove why $3 $4 $5"
    case "$4" in
      conclusion/01)
{why_rerun}        ;;
      conclusion/40)
{why_kept}        ;;
      *)
        bad
        ;;
    esac
    ;;
  frontier)
    if [ "$#" -eq 3 ]; then
      same "$3"
      note "prove frontier $3"
{frontier}    elif [ "$#" -eq 4 ]; then
      same "$4"
      note "prove frontier $3 $4"
      case "$3" in
        conclusion/41)
{unavailable}          ;;
        leaf/native-worker-adapter)
{native}          ;;
        leaf/native-assay)
{assay}          ;;
        *)
          bad
          ;;
      esac
    else
      bad
    fi
    ;;
  resume)
    [ "$#" -eq 4 ] || bad
    same "$4"
    note "prove resume $3 $4"
    case "$3" in
      {plan_token})
{plan_next}        ;;
      {frontier_token})
{frontier_resume}        ;;
      *)
        bad
        ;;
    esac
    ;;
  *)
    bad
    ;;
esac
"#,
        log = log,
        view = view,
        gap = print_record(&gap_record("view", "refresh-needed"), receipt),
        proof = print_record(&proof, receipt),
        runs = runs,
        plan = print_record(&plan_record(0, &page(1), PLAN_TOKEN), receipt),
        status = print_record(&status_record(), receipt),
        why_rerun = print_record(
            &why_record("conclusion/01", false, "[\"stable/01\"]"),
            receipt
        ),
        why_kept = print_record(&why_record("conclusion/40", true, "[]"), receipt),
        frontier = print_record(&frontier_record(), receipt),
        unavailable = print_record(&unavailable, receipt),
        native = print_record(&native, receipt),
        assay = print_record(&assay, receipt),
        plan_token = PLAN_TOKEN,
        plan_next = print_record(&plan_record(1, &page(9), NEXT_TOKEN), receipt),
        frontier_token = FRONTIER_TOKEN,
        frontier_resume = print_record(&frontier_record(), receipt),
    )
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
fn closure_facts_do_not_execute_the_comparison() {
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
        MemorySubject::default().with(packs::product::COMMAND_FACT, "completion/claim-16");
    let (code, _) = refusal(&execute(&relabelled));
    assert_eq!(code, "claim-relabelled");
}

#[test]
fn the_named_program_supplies_the_proof_comparison() {
    let (dir, program) = prepare("pass", Fault::None);
    let verdict = execute(&command(&program));
    let proof = pass_value(&verdict, "release.prove.proof");
    assert!(proof.contains("(denominator 40)"), "{proof}");
    assert!(proof.contains(VIEW), "{proof}");
    assert!(proof.contains("(executed 40)"), "{proof}");
    let d1 = pass_value(&verdict, "release.prove.D1");
    let d2 = pass_value(&verdict, "release.prove.D2");
    let d4 = pass_value(&verdict, "release.prove.D4");
    assert!(d1.contains("(reuse-basis-points 10000)"), "{d1}");
    assert!(d1.contains("(false-reuse 0)"), "{d1}");
    assert!(d2.contains("(reuse-basis-points 9500)"), "{d2}");
    assert!(d2.contains("(false-reuse 0)"), "{d2}");
    assert!(d4.contains("(reuse-basis-points 7000)"), "{d4}");
    assert!(d4.contains("(reuse-requests 24)"), "{d4}");
    let d6 = pass_value(&verdict, "release.prove.D6");
    assert!(d6.contains("(revision-delta 1)"), "{d6}");
    let unavailable = pass_value(&verdict, "release.prove.unavailable");
    assert!(unavailable.contains("unavailable-proof"), "{unavailable}");
    let worker = pass_value(
        &verdict,
        "release.prove.excluded.leaf/native-worker-adapter",
    );
    assert!(worker.contains("excluded-scope"), "{worker}");
    let log = std::fs::read_to_string(dir.join("ran.log")).unwrap();
    for line in [
        "prove run proof",
        &format!("prove run D2 {VIEW}"),
        &format!("prove plan {VIEW}"),
        &format!("prove status {VIEW}"),
        &format!("prove why D2 conclusion/01 {VIEW}"),
        &format!("prove why D2 conclusion/40 {VIEW}"),
        &format!("prove frontier {VIEW}"),
        &format!("prove frontier conclusion/41 {VIEW}"),
        &format!("prove frontier leaf/native-worker-adapter {VIEW}"),
        &format!("prove frontier leaf/native-assay {VIEW}"),
        &format!("prove resume {PLAN_TOKEN} {VIEW}"),
        &format!("prove resume {FRONTIER_TOKEN} {VIEW}"),
    ] {
        assert_eq!(
            log.matches(&format!("{line}\n")).count(),
            2,
            "{line}\n{log}"
        );
    }
}

#[test]
fn a_phase_union_basis_is_not_the_proof_comparison() {
    let (_dir, program) = prepare("union", Fault::UnionBasis);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "reuse-basis-points");
}

#[test]
fn the_phase_union_cold_export_is_not_the_proof() {
    let (_dir, program) = prepare("cold", Fault::ColdExport);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "prove/proof-v1");
}

#[test]
fn a_missing_load_receipt_is_not_a_proof() {
    let (_dir, program) = prepare("receipt", Fault::MissingReceipt);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "load-receipt");
}

#[test]
fn an_open_leaf_is_not_a_complete_frontier() {
    let (_dir, program) = prepare("open", Fault::OpenFrontier);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "marker-absent", "{detail}");
    assert_eq!(detail, "prove/gap-v1");
}

#[test]
fn a_native_leaf_stays_outside_the_denominator() {
    let (_dir, program) = prepare("native", Fault::NativeReason);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "reason");
}
