// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-09` runs the named program. Closure booleans are not a snapshot.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const ROWS: &[(&str, &str, &str)] = &[
    ("ci/run", "uncertain", "ci"),
    ("filesystem/readme", "observed", "filesystem"),
    ("filesystem/secret", "denied", "filesystem"),
    ("git/HEAD", "timed-out", "git"),
    ("process/probe", "unavailable", "process"),
    ("registry/index", "absent", "registry"),
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
        scope: "release/nv12-09",
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
        std::env::temp_dir().join(format!("sim-release-observe-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn quote(text: &str) -> String {
    format!("\"{text}\"")
}

fn list(items: &[&str]) -> String {
    format!(
        "[{}]",
        items
            .iter()
            .map(|item| quote(item))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

struct Shot<'a> {
    batch: &'a str,
    selector: &'a str,
    status: &'a str,
    port: &'a str,
    matched: bool,
    policy: &'a str,
    provenance: &'a str,
    value: Option<&'a str>,
    reads: &'a [&'a str],
    ports: &'a [&'a str],
}

fn snapshot(shot: &Shot<'_>) -> String {
    let matched = if shot.matched { "true" } else { "false" };
    let value_field = shot
        .value
        .map(|body| format!(" (value {})", quote(body)))
        .unwrap_or_default();
    let row = format!(
        "#(world/observation-v1 (envelope {}) (freshness {}) (matched {matched}) (policy {}) (port {}) (provenance {}) (selector {}) (semantic-id {}) (status {}){value_field})",
        quote("model-log"),
        quote("fresh"),
        quote(shot.policy),
        quote(shot.port),
        quote(shot.provenance),
        quote(shot.selector),
        quote("semantic"),
        quote(shot.status),
    );
    format!(
        "#(world/snapshot-v1 (batch {}) (effects 0) (observations [{row}]) (ports {}) (reads {}) (resident true) (selector {}) (subprocesses 0))",
        quote(shot.batch),
        list(shot.ports),
        list(shot.reads),
        quote(shot.selector),
    )
}

fn model_row(selector: &str, status: &str, port: &str) -> String {
    let provenance = format!("model/{selector}");
    snapshot(&Shot {
        batch: "admitted",
        selector,
        status,
        port,
        matched: true,
        policy: "policy/explicit-model-v1",
        provenance: &provenance,
        value: (status == "observed").then_some("verified-source"),
        reads: &[selector],
        ports: &[port],
    })
}

fn native_row() -> String {
    snapshot(&Shot {
        batch: "native",
        selector: "filesystem/readme",
        status: "unavailable",
        port: "native",
        matched: false,
        policy: "policy/no-native-fallback-v1",
        provenance: "native-capability-missing",
        value: None,
        reads: &[],
        ports: &[],
    })
}

struct Change<'a> {
    selector: &'a str,
    before: &'a str,
    after: &'a str,
    changed: bool,
    before_status: Option<&'a str>,
    after_status: Option<&'a str>,
    reads: &'a [&'a str],
    ports: &'a [&'a str],
}

fn diff_row(change: &Change<'_>) -> String {
    let changed = if change.changed { "true" } else { "false" };
    let before_status = change
        .before_status
        .map(|status| format!(" (before-status {})", quote(status)))
        .unwrap_or_default();
    let after_status = change
        .after_status
        .map(|status| format!(" (after-status {})", quote(status)))
        .unwrap_or_default();
    format!(
        "#(world/observation-diff-v1 (after {}){after_status} (before {}){before_status} (changed {changed}) (effects 0) (ports {}) (reads {}) (resident true) (selector {}) (subprocesses 0))",
        quote(change.after),
        quote(change.before),
        list(change.ports),
        list(change.reads),
        quote(change.selector),
    )
}

struct Arms {
    rows: Vec<(String, String)>,
    native: String,
    changed: String,
    quiet: String,
    second_ci: Option<String>,
}

fn admitted() -> Arms {
    Arms {
        rows: ROWS
            .iter()
            .map(|(selector, status, port)| {
                ((*selector).to_owned(), model_row(selector, status, port))
            })
            .collect(),
        native: native_row(),
        changed: diff_row(&Change {
            selector: "git/HEAD",
            before: "before",
            after: "after",
            changed: true,
            before_status: Some("timed-out"),
            after_status: Some("observed"),
            reads: &["git/HEAD", "git/HEAD"],
            ports: &["git", "git"],
        }),
        quiet: diff_row(&Change {
            selector: "filesystem/readme",
            before: "admitted",
            after: "admitted",
            changed: false,
            before_status: None,
            after_status: None,
            reads: &[],
            ports: &[],
        }),
        second_ci: None,
    }
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

fn print_arm(record: &str) -> String {
    assert!(!record.contains('\''), "{record}");
    format!("  printf '%s\\n' '{record}'\n  ;;\n")
}

fn stub(dir: &Path, arms: &Arms) -> PathBuf {
    let mut body = format!(
        "log={}\ncount={}\nsub=\"$2\"\nif [ \"$sub\" = snapshot-diff ]; then sel=\"$5\"; doc=\"$3\"; other=\"$4\"; else sel=\"$4\"; doc=\"$3\"; other=\"\"; fi\nprintf '%s\\n' \"BEGIN $sub $sel\" >> \"$log\"\nprintf '%s\\n' \"$doc\" >> \"$log\"\nif [ -n \"$other\" ]; then printf '%s\\n' \"$other\" >> \"$log\"; fi\nprintf '%s\\n' \"END $sub $sel\" >> \"$log\"\ncase \"$sub:$sel\" in\n",
        shell_quote(&dir.join("ran.log")),
        shell_quote(&dir.join("count")),
    );
    for (selector, record) in &arms.rows {
        body.push_str(&format!("snapshot:{selector})\n"));
        if selector == "filesystem/readme" {
            assert!(!arms.native.contains('\''));
            assert!(!record.contains('\''));
            body.push_str(
                "  if printf '%s\\n' \"$doc\" | grep -q 'realization native'; then\n    printf '%s\\n' '",
            );
            body.push_str(&arms.native);
            body.push_str("'\n  else\n    printf '%s\\n' '");
            body.push_str(record);
            body.push_str("'\n  fi\n  ;;\n");
            continue;
        }
        if selector == "ci/run"
            && let Some(second) = &arms.second_ci
        {
            assert!(!second.contains('\''));
            assert!(!record.contains('\''));
            body.push_str(
                "  n=0\n  if [ -f \"$count\" ]; then n=$(cat \"$count\"); fi\n  n=$((n + 1))\n  printf '%s\\n' \"$n\" > \"$count\"\n  if [ \"$n\" = 1 ]; then\n    printf '%s\\n' '",
            );
            body.push_str(record);
            body.push_str("'\n  else\n    printf '%s\\n' '");
            body.push_str(second);
            body.push_str("'\n  fi\n  ;;\n");
            continue;
        }
        body.push_str(&print_arm(record));
    }
    body.push_str("snapshot-diff:git/HEAD)\n");
    body.push_str(&print_arm(&arms.changed));
    body.push_str("snapshot-diff:filesystem/readme)\n");
    body.push_str(&print_arm(&arms.quiet));
    body.push_str("*)\n  printf '%s\\n' \"unexpected $sub $sel\" >&2\n  exit 1\n  ;;\nesac\n");
    write_sim(dir, &body)
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
fn closure_facts_do_not_execute_the_observation() {
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
        MemorySubject::default().with(packs::product::COMMAND_FACT, "completion/claim-09");
    let (code, _) = refusal(&execute(&relabelled));
    assert_eq!(code, "claim-relabelled");
}

fn prepare(name: &str, arms: &Arms) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    init_checkout(&dir);
    let program = stub(&dir, arms);
    (dir, program)
}

#[test]
fn the_named_program_supplies_distinct_statuses() {
    let arms = admitted();
    let (dir, program) = prepare("pass", &arms);
    let verdict = execute(&command(&program));
    for (selector, record) in &arms.rows {
        assert_eq!(
            pass_value(&verdict, &format!("release.observe.{selector}")).trim_end(),
            record,
            "{selector}"
        );
    }
    assert_eq!(
        pass_value(&verdict, "release.observe.native").trim_end(),
        arms.native
    );
    assert_eq!(
        pass_value(&verdict, "release.observe.changed").trim_end(),
        arms.changed
    );
    assert_eq!(
        pass_value(&verdict, "release.observe.quiet").trim_end(),
        arms.quiet
    );
    let log = std::fs::read_to_string(dir.join("ran.log")).unwrap();
    assert_eq!(log.matches("BEGIN snapshot ci/run\n").count(), 2, "{log}");
    assert_eq!(
        log.matches("BEGIN snapshot filesystem/secret\n").count(),
        2,
        "{log}"
    );
    assert_eq!(log.matches("realization native\n").count(), 2, "{log}");
    assert_eq!(
        log.matches("BEGIN snapshot-diff git/HEAD\n").count(),
        2,
        "{log}"
    );
    assert_eq!(
        log.matches("BEGIN snapshot-diff filesystem/readme\n")
            .count(),
        2,
        "{log}"
    );
    assert!(
        log.contains("filesystem filesystem/secret denied -"),
        "{log}"
    );
    assert!(log.contains("git git/HEAD observed abc"), "{log}");
    assert!(!log.contains("prove plan"), "{log}");
}

#[test]
fn a_longer_selector_is_not_the_requested_selector() {
    let mut arms = admitted();
    let row = arms
        .rows
        .iter_mut()
        .find(|(selector, _)| selector == "filesystem/secret")
        .unwrap();
    row.1 = model_row("filesystem/secret-file", "denied", "filesystem");
    let (_dir, program) = prepare("prefix", &arms);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "selector");
}

#[test]
fn a_read_for_a_different_selector_is_refused() {
    let mut arms = admitted();
    let row = arms
        .rows
        .iter_mut()
        .find(|(selector, _)| selector == "filesystem/secret")
        .unwrap();
    row.1 = snapshot(&Shot {
        batch: "admitted",
        selector: "filesystem/secret",
        status: "denied",
        port: "filesystem",
        matched: true,
        policy: "policy/explicit-model-v1",
        provenance: "model/filesystem/secret",
        value: None,
        reads: &["filesystem/readme"],
        ports: &["filesystem"],
    });
    let (_dir, program) = prepare("read", &arms);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "reads");
}

#[test]
fn an_unchanged_batch_that_reads_a_port_is_refused() {
    let mut arms = admitted();
    arms.quiet = diff_row(&Change {
        selector: "filesystem/readme",
        before: "admitted",
        after: "admitted",
        changed: false,
        before_status: None,
        after_status: None,
        reads: &["filesystem/readme"],
        ports: &["filesystem"],
    });
    let (_dir, program) = prepare("quiet", &arms);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "reads");
}

#[test]
fn a_native_value_is_refused() {
    let mut arms = admitted();
    arms.native = snapshot(&Shot {
        batch: "native",
        selector: "filesystem/readme",
        status: "unavailable",
        port: "native",
        matched: false,
        policy: "policy/no-native-fallback-v1",
        provenance: "native-capability-missing",
        value: Some("verified-source"),
        reads: &[],
        ports: &[],
    });
    let (_dir, program) = prepare("native", &arms);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "value");
}

#[test]
fn a_collapsed_status_is_refused() {
    let mut arms = admitted();
    let row = arms
        .rows
        .iter_mut()
        .find(|(selector, _)| selector == "filesystem/secret")
        .unwrap();
    row.1 = model_row("filesystem/secret", "uncertain", "filesystem");
    let (_dir, program) = prepare("status", &arms);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch", "{detail}");
    assert_eq!(detail, "status");
}

#[test]
fn a_changed_second_snapshot_is_refused() {
    let mut arms = admitted();
    arms.second_ci =
        Some(model_row("ci/run", "uncertain", "ci").replace("(effects 0)", "(effects 1)"));
    let (_dir, program) = prepare("unstable", &arms);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "observation-unstable", "{detail}");
    assert_eq!(detail, "ci/run");
}

#[test]
fn a_sealed_word_in_program_output_is_refused() {
    let mut arms = admitted();
    let row = arms
        .rows
        .iter_mut()
        .find(|(selector, _)| selector == "ci/run")
        .unwrap();
    row.1.push_str(" PortableClosed");
    let (_dir, program) = prepare("sealed", &arms);
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "forbidden-output", "{detail}");
}
