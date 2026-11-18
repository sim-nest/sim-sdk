// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Delivery laws come from the printed deliver record, not caller facts.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const SCOPES: [&str; 5] = [
    "delivery/observation",
    "delivery/plan",
    "delivery/adapters",
    "delivery/real-canary",
    "delivery/composed",
];

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn execute(scope: &str, evidence: &MemorySubject) -> PackVerdict {
    let binding = find_pack("checker/c-deliver")
        .unwrap()
        .checker_binding()
        .unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    packs::delivery::check(&PackRequest {
        checker: "checker/c-deliver",
        binding,
        subject: SUBJECT,
        scope,
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
    MemorySubject::default().with(packs::delivery::COMMAND_FACT, path.display().to_string())
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("sim-delivery-record-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sim(dir: &Path, line: &str) -> PathBuf {
    let path = dir.join("sim");
    let mut body = String::from(
        "#!/bin/sh\nset -e\ncase \"$1:$2:$3\" in\nworld:deliver:model)\n  cat <<'EOF'\n",
    );
    body.push_str(line);
    body.push_str("\nEOF\n  ;;\n*)\n  exit 1\n  ;;\nesac\n");
    std::fs::write(&path, body).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn action(
    cause: &str,
    class: &str,
    acknowledgement: &str,
    dispatches: u64,
    effects: u64,
) -> String {
    format!(
        "#(world/deliver-action-v1 (acknowledgement \"{acknowledgement}\") (cause \"{cause}\") (class \"{class}\") (dispatches {dispatches}) (effects {effects}) (selector \"{cause}\"))"
    )
}

fn omission() -> String {
    "#(world/deliver-omission-v1 (acknowledgement \"omitted\") (cause \"already-true\") (class \"already-true\") (dispatches 0) (effects 0) (selector \"git/branch/main\"))".to_owned()
}

fn record(outer_effects: u64, actions: &[String], omissions: &[String]) -> String {
    let note = "#(world/deliver-omission-v1 (cause \\\"already-true\\\") (effects 9))";
    format!(
        "#(world/deliver-v1 (actions [{}]) (effects {outer_effects}) (note \"{note}\") (omissions [{}]) (realization \"model\") (recovered true) (resident true) (subprocesses 0))",
        actions.join(" "),
        omissions.join(" ")
    )
}

fn lawful(outer_effects: u64) -> String {
    record(
        outer_effects,
        &[
            action("stale-observation", "uncertain", "unsent", 0, 0),
            action("partial-success", "uncertain", "missing", 1, 0),
            action("divergent", "diverged", "missing", 1, 0),
            action("registry-lag", "uncertain", "unsent", 0, 0),
            action("uncertain-reading", "uncertain", "unsent", 0, 0),
        ],
        &[omission()],
    )
}

fn pass_value(verdict: &PackVerdict) -> &str {
    match verdict {
        PackVerdict::Pass { observations, .. } => {
            assert_eq!(observations.len(), 1);
            assert_eq!(observations[0].key, "delivery.model");
            observations[0].value.as_str()
        }
        other => panic!("expected a pass, got {other:?}"),
    }
}

fn result_text(verdict: &PackVerdict) -> String {
    match verdict {
        PackVerdict::Pass { result, .. } => format!("{result:?}"),
        other => panic!("expected a pass, got {other:?}"),
    }
}

#[test]
fn caller_delivery_facts_are_not_a_delivery() {
    let (code, detail) = refusal(&execute("delivery/observation", &MemorySubject::default()));
    assert_eq!(code, "missing-source");
    assert_eq!(detail, "delivery.command");
    let truths = packs::delivery::SCENARIOS
        .iter()
        .fold(MemorySubject::default(), |subject, key| {
            subject.with(*key, "true")
        });
    let (code, detail) = refusal(&execute("delivery/plan", &truths));
    assert_eq!(code, "unexpected-fact");
    assert_eq!(detail, "delivery.already-true");
}

#[test]
fn a_printed_record_is_the_delivery_observation() {
    let dir = scratch("pass");
    let program = write_sim(&dir, &lawful(0));
    let mut results = Vec::new();
    for scope in SCOPES {
        let verdict = execute(scope, &command(&program));
        let value = pass_value(&verdict);
        assert_eq!(value.trim_end(), lawful(0));
        results.push(result_text(&verdict));
    }
    assert!(
        results.windows(2).all(|pair| pair[0] != pair[1]),
        "delivery scopes must not share one result"
    );
}

#[test]
fn an_inner_effect_is_not_the_plan_effect() {
    let dir = scratch("outer-effect");
    let program = write_sim(&dir, &lawful(1));
    let (code, detail) = refusal(&execute("delivery/observation", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "effects");
}

#[test]
fn a_quoted_omission_is_not_the_checkpoint() {
    let dir = scratch("quoted");
    let line = record(
        0,
        &[
            action("stale-observation", "uncertain", "unsent", 0, 0),
            action("partial-success", "uncertain", "missing", 1, 0),
            action("divergent", "diverged", "missing", 1, 0),
            action("registry-lag", "uncertain", "unsent", 0, 0),
            action("uncertain-reading", "uncertain", "unsent", 0, 0),
        ],
        &[],
    );
    let program = write_sim(&dir, &line);
    let (code, detail) = refusal(&execute("delivery/adapters", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "delivery.already-true");
}

#[test]
fn an_already_true_action_is_not_an_omission() {
    let dir = scratch("action");
    let actions = vec![
        action("stale-observation", "uncertain", "unsent", 0, 0),
        action("partial-success", "uncertain", "missing", 1, 0),
        action("divergent", "diverged", "missing", 1, 0),
        action("registry-lag", "uncertain", "unsent", 0, 0),
        action("uncertain-reading", "uncertain", "unsent", 0, 0),
        action("already-true", "already-true", "omitted", 0, 0),
    ];
    let program = write_sim(&dir, &record(0, &actions, &[omission()]));
    let (code, detail) = refusal(&execute("delivery/real-canary", &command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "delivery.completed-checkpoint-omission-refused");
}

#[test]
fn a_second_record_on_the_same_line_is_not_the_delivery() {
    let dir = scratch("trailing");
    let line = format!("#(world/deliver-v1 ){}", lawful(0));
    let program = write_sim(&dir, &line);
    let (code, detail) = refusal(&execute("delivery/observation", &command(&program)));
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_empty_root_glued_to_a_second_record_is_extra() {
    let dir = scratch("glued");
    let line = format!("#(world/deliver-v1){}", lawful(0));
    let program = write_sim(&dir, &line);
    let (code, detail) = refusal(&execute("delivery/adapters", &command(&program)));
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_unclosed_root_is_not_a_record() {
    let dir = scratch("unclosed");
    let line = lawful(0);
    let line = line.strip_suffix(')').expect("root closes");
    let program = write_sim(&dir, line);
    let (code, detail) = refusal(&execute("delivery/plan", &command(&program)));
    assert_eq!(code, "bad-record");
    assert_eq!(detail, "unclosed");
}

#[test]
fn installed_sim_records_the_delivery_laws() {
    let Some(path) = std::env::var_os("SIM_PRODUCT_BIN") else {
        let (code, detail) = refusal(&execute("delivery/composed", &MemorySubject::default()));
        assert_eq!(code, "missing-source");
        assert_eq!(detail, "delivery.command");
        return;
    };
    let program = PathBuf::from(path);
    assert!(program.is_file(), "installed sim is missing");
    let verdict = execute("delivery/composed", &command(&program));
    let value = pass_value(&verdict);
    assert!(value.contains("#(world/deliver-v1 "));
    assert!(value.contains("already-true"));
    assert!(value.contains("stale-observation"));
    assert!(value.contains("registry-lag"));
    assert!(value.contains("uncertain-reading"));
    assert!(value.contains("divergent"));
    assert!(!value.contains("completion/claim-"));
    assert!(!value.contains("PortableClosed"));
    assert!(!value.contains("derive_completion"));
}
