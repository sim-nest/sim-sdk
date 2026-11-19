// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The product checker runs the named program. Boolean facts are not a result.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const RECEIPT: &str = "#(prove/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/prove-command\"))";
const ROADMAP: &str = "#(roadmap/local-result-v1 (outcome \"ok\") (execution \"local/no-model\") (conduct \"conduct/local\") (model-pick \"model/local\") (proof-catalog \"proof/local\") (runner-generation \"generation/local\") (journal-acknowledged true) (detail \"execution not started\"))";

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn request<'a>(scope: &'a str, evidence: &'a MemorySubject) -> PackRequest<'a> {
    let binding = find_pack("checker/c-product")
        .unwrap()
        .checker_binding()
        .unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    PackRequest {
        checker: "checker/c-product",
        binding,
        subject: SUBJECT,
        scope,
        evidence,
    }
}

fn execute(scope: &str, evidence: &MemorySubject) -> PackVerdict {
    packs::product::check(&request(scope, evidence))
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
        std::env::temp_dir().join(format!("sim-product-command-{name}-{}", std::process::id()));
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

fn cold_line(view: &str, denominator: &str, executed: &str) -> String {
    let encoded = view.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "#(prove/cold-v1 (denominator {denominator}) (executed {executed}) (view \"{encoded}\"))"
    )
}

fn stub(dir: &Path, view: &str) -> PathBuf {
    let log = quote(&dir.join("ran.log"));
    let chunks = quote(&dir.join("chunks"));
    let cold = cold_line(view, "29", "29");
    let body = format!(
        r#"cmd="$1"
sub="$2"
pwd >> {log}
printf '%s\n' "$*" >> {log}
receipt='{RECEIPT}'
case "$cmd:$sub" in
world:why)
  printf '%s\n' '#(world/explanation-v1 (path ["owner/sim-private/disclosure"]))'
  ;;
world:project)
  printf '%s\n' '#(world/projection-v1 conclusion/source-api)'
  ;;
world:diff)
  printf '%s\n' '#(world/diff-v1 (changed true) conclusion/public-release)'
  ;;
world:deliver)
  printf '%s\n' '#(world/deliver-v1 (realization "model") (effects 0) (subprocesses 0) (resident true) (recovered true))'
  ;;
roadmap:status)
  printf '%s\n' '{ROADMAP}'
  ;;
prove:plan)
  printf '%s\n' '#(prove/plan-v1 (denominator 29) (excluded ["leaf/native-worker-adapter"]))'
  printf '%s\n' "$receipt"
  ;;
prove:run)
  printf '%s\n' '{cold}'
  printf '%s\n' "$receipt"
  ;;
prove:status|prove:frontier)
  shift 2
  printf '%s\n' "$@" > {chunks}
  ;;
esac
if [ "$cmd" = prove ] && [ "$sub" = status ]; then
  printf '%s\n' '#(prove/status-v1 (denominator 29) (executed 29))'
  printf '%s\n' "$receipt"
elif [ "$cmd" = prove ] && [ "$sub" = frontier ]; then
  printf '%s\n' '#(prove/frontier-v1 (complete true) (denominator 29) (executed 29))'
  printf '%s\n' "$receipt"
fi
"#
    );
    write_sim(dir, &body)
}

#[test]
fn boolean_facts_are_not_product_execution() {
    let truths = [
        "product.world-loads-through-sim-run",
        "product.loads-through-sim-run",
        "product.stable-value",
        "product.verb-not-baked",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    });
    let (code, _) = refusal(&execute("product/world", &truths));
    assert_eq!(code, "unexpected-fact");
    for scope in [
        "product/prove-plan",
        "product/prove",
        "product/deliver",
        "product/roadmap",
        "product/convergence-queries",
    ] {
        let (code, _) = refusal(&execute(scope, &truths));
        assert_eq!(code, "unexpected-fact", "{scope}");
    }
    for scope in [
        "product/landing-queries",
        "product/installed",
        "product/composed",
    ] {
        let (code, detail) = refusal(&execute(scope, &truths));
        assert_eq!(code, "unexecuted", "{scope} {detail}");
    }
    let pasted = MemorySubject::default().with(
        packs::product::COMMAND_FACT,
        "#(world/explanation-v1 owner/sim-private/disclosure)",
    );
    let (code, _) = refusal(&execute("product/world", &pasted));
    assert_eq!(code, "missing-source");
    let claimed = MemorySubject::default().with(packs::product::COMMAND_FACT, "true");
    let (code, _) = refusal(&execute("product/deliver", &claimed));
    assert_eq!(code, "bad-record");
}

#[test]
fn the_checker_runs_the_named_program_and_records_its_output() {
    let dir = scratch("pass");
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git init")
            .success()
    );
    let view = "view-token";
    let program = stub(&dir, view);
    let evidence = command(&program);
    let world = execute("product/world", &evidence);
    assert!(pass_value(&world, "product.world-why").contains("world/explanation-v1"));
    assert!(pass_value(&world, "product.world-why").contains("owner/sim-private/disclosure"));
    assert!(pass_value(&world, "product.world-project").contains("world/projection-v1"));
    assert!(pass_value(&world, "product.world-diff").contains("world/diff-v1"));
    assert!(!pass_value(&world, "product.world-diff").contains("conclusion/source-api"));
    let plan = execute("product/prove-plan", &evidence);
    assert!(pass_value(&plan, "product.prove-plan").contains("#(prove/plan-v1 "));
    assert!(pass_value(&plan, "product.prove-plan").contains(RECEIPT));
    let prove = execute("product/prove", &evidence);
    assert!(pass_value(&prove, "product.prove-status").contains("#(prove/status-v1 "));
    assert!(pass_value(&prove, "product.prove-frontier").contains("#(prove/frontier-v1 "));
    assert!(pass_value(&prove, "product.prove-frontier").contains("(complete true)"));
    let deliver = execute("product/deliver", &evidence);
    assert!(pass_value(&deliver, "product.deliver").contains("#(world/deliver-v1 "));
    assert!(!pass_value(&deliver, "product.deliver").contains("PortableClosed"));
    let roadmap = execute("product/roadmap", &evidence);
    assert_eq!(pass_value(&roadmap, "product.roadmap").trim_end(), ROADMAP);
    let log = std::fs::read_to_string(dir.join("ran.log")).unwrap();
    let checkout = dir.canonicalize().unwrap();
    assert_eq!(
        log.lines().next().map(Path::new),
        Some(checkout.as_path()),
        "{log}"
    );
    assert!(
        log.contains("world why conclusion/public-release policy/no-v3-disclosure"),
        "{log}"
    );
    assert!(log.contains("prove run cold"), "{log}");
    assert!(log.contains("prove status view-token"), "{log}");
    assert!(
        log.contains("roadmap status --conduct conduct/local"),
        "{log}"
    );
    assert!(log.contains("world deliver model"), "{log}");
    let saved = std::fs::read_to_string(dir.join("chunks")).unwrap();
    assert_eq!(saved.lines().collect::<String>(), view);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_long_prove_view_is_passed_in_chunks() {
    let dir = scratch("chunks");
    let view = "a".repeat(100_001);
    let program = stub(&dir, &view);
    let prove = execute("product/prove", &command(&program));
    assert!(pass_value(&prove, "product.prove-cold").contains("prove/cold-v1"));
    let saved = std::fs::read_to_string(dir.join("chunks")).unwrap();
    assert_eq!(saved.lines().collect::<Vec<_>>().len(), 2);
    assert_eq!(saved.lines().collect::<String>(), view);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn an_escaped_prove_view_is_the_status_argument() {
    let dir = scratch("escape");
    let view = "say \"hi\"";
    let program = stub(&dir, view);
    let _prove = execute("product/prove", &command(&program));
    let saved = std::fs::read_to_string(dir.join("chunks")).unwrap();
    assert_eq!(saved.lines().next().unwrap(), view);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn wrong_output_claim_text_and_a_failed_process_are_refused() {
    let dir = scratch("refuse");
    let mark = dir.join("ran");
    let program = write_sim(
        &dir,
        &format!("touch {}\nprintf '%s\\n' true\n", quote(&mark)),
    );
    let (code, _) = refusal(&execute("product/world", &command(&program)));
    assert_eq!(code, "marker-absent");
    assert!(mark.is_file(), "the program did not run");

    let program = write_sim(
        &dir,
        "printf '%s\\n' 'world/explanation-v1 completion/claim-18 owner/sim-private/disclosure'\n",
    );
    let (code, _) = refusal(&execute("product/world", &command(&program)));
    assert_eq!(code, "claim-relabelled");

    let program = write_sim(&dir, "printf '%s\\n' '#(world/deliver-v1 '\nexit 1\n");
    let (code, _) = refusal(&execute("product/deliver", &command(&program)));
    assert_eq!(code, "process-failed");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_quoted_complete_marker_is_not_the_frontier_field() {
    let dir = scratch("quoted-complete");
    let program = write_sim(
        &dir,
        &format!(
            r#"cmd="$1"
sub="$2"
receipt='{RECEIPT}'
case "$cmd:$sub" in
prove:plan)
  printf '%s\n' '#(prove/plan-v1 (denominator 29) (excluded ["leaf/native-worker-adapter"]))'
  printf '%s\n' "$receipt"
  ;;
prove:run)
  printf '%s\n' '#(prove/cold-v1 (denominator 29) (executed 29) (view "v"))'
  printf '%s\n' "$receipt"
  ;;
prove:status)
  printf '%s\n' '#(prove/status-v1 (denominator 29) (executed 29))'
  printf '%s\n' "$receipt"
  ;;
prove:frontier)
  printf '%s\n' '#(prove/frontier-v1 (diagnostic "(complete true)") (complete false) (denominator 29) (executed 29))'
  printf '%s\n' "$receipt"
  ;;
esac
"#
        ),
    );
    let (code, detail) = refusal(&execute("product/prove", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "complete");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_deep_program_runs_in_its_checkout() {
    let dir = scratch("deep");
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git init")
            .success()
    );
    let mut nested = dir.clone();
    for index in 0..40 {
        nested.push(format!("d{index}"));
    }
    std::fs::create_dir_all(&nested).unwrap();
    let program = write_sim(
        &nested,
        r#"pwd
cmd="$1"
sub="$2"
case "$cmd:$sub" in
world:why)
  printf '%s\n' '#(world/explanation-v1 (path ["owner/sim-private/disclosure"]))'
  ;;
world:project)
  printf '%s\n' '#(world/projection-v1 conclusion/source-api)'
  ;;
world:diff)
  printf '%s\n' '#(world/diff-v1 (changed true) conclusion/public-release)'
  ;;
esac
"#,
    );
    let world = execute("product/world", &command(&program));
    let checkout = dir.canonicalize().unwrap();
    assert_eq!(
        pass_value(&world, "product.world-why")
            .lines()
            .next()
            .map(Path::new),
        Some(checkout.as_path())
    );
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn installed_sim_records_product_commands() {
    let named = std::env::var_os("SIM_PRODUCT_BIN").map(PathBuf::from);
    let Some(program) = named.filter(|path| path.is_file()) else {
        let (code, _) = refusal(&execute(
            "product/world",
            &command(Path::new("missing-sim-product-bin")),
        ));
        assert_eq!(code, "missing-source");
        return;
    };
    let evidence = command(&program);
    let world = execute("product/world", &evidence);
    assert!(pass_value(&world, "product.world-why").contains("world/explanation-v1"));
    assert!(pass_value(&world, "product.world-project").contains("world/projection-v1"));
    assert!(pass_value(&world, "product.world-diff").contains("world/diff-v1"));
    let plan = execute("product/prove-plan", &evidence);
    assert!(pass_value(&plan, "product.prove-plan").contains("#(prove/plan-v1 "));
    assert!(pass_value(&plan, "product.prove-plan").contains("leaf/native-worker-adapter"));
    let deliver = execute("product/deliver", &evidence);
    let delivered = pass_value(&deliver, "product.deliver");
    assert!(delivered.contains("#(world/deliver-v1 "));
    assert!(delivered.contains("(realization \"model\")"));
    assert!(!delivered.contains("PortableClosed"));
    assert!(!delivered.contains("derive_completion"));
    let roadmap = execute("product/roadmap", &evidence);
    assert_eq!(pass_value(&roadmap, "product.roadmap").trim_end(), ROADMAP);
    let prove = execute("product/prove", &evidence);
    assert!(pass_value(&prove, "product.prove-status").contains("#(prove/status-v1 "));
    assert!(pass_value(&prove, "product.prove-frontier").contains("(complete true)"));
}
