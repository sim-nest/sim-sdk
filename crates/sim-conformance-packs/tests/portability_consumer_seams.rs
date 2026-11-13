// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fs;
use std::path::Path;

use sim_conformance_packs::{
    EXECUTOR_CLOSURE_PATHS, MemorySubject, PackVerdict, WORKSPACE_RESOLVER_LOCK,
    compiled_executor_closure, find_pack,
    packs::portability::consumer_seams::{
        SeamFault, advice_rejected, advice_slot, capture_executor_subject_closure, changed_answer,
        check_subject, direction_once, discover_affordance, hot_needs_begin, hot_noop,
        interrupt_once, open_with_linux, over_bound, qualify, qualify_subject, renote_stales,
        reopen_keeps_direction, reopen_same, revoked_subject, scope_confusion,
        selection_is_distinct, selection_without_begin, snapshot_pages, wrong_execution,
    },
};

fn projects_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn closure() -> (Vec<(String, Vec<u8>)>, String) {
    let root = projects_root();
    let mut entries = Vec::new();
    for relative in EXECUTOR_CLOSURE_PATHS {
        let path = root.join(relative);
        let bytes = fs::read(&path).unwrap_or_else(|_| panic!("missing {}", path.display()));
        entries.push(((*relative).to_owned(), bytes));
    }
    let lock = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let bytes = fs::read(&lock).unwrap_or_else(|_| panic!("missing {}", lock.display()));
    entries.push((WORKSPACE_RESOLVER_LOCK.to_owned(), bytes));
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let borrowed = entries
        .iter()
        .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
        .collect::<Vec<_>>();
    let digest = capture_executor_subject_closure(&borrowed).expect("closure");
    (entries, digest)
}

fn request<'a>(
    evidence: &'a MemorySubject,
    binding: &'a str,
) -> sim_conformance_packs::PackRequest<'a> {
    sim_conformance_packs::PackRequest {
        checker: "checker/c-port",
        binding,
        subject: "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        scope: "portability/consumer-seams",
        evidence,
    }
}

#[test]
#[ignore = "requires sibling repository checkouts"]
fn frozen_executor_sources_qualify_the_outside_specimen() {
    let (_files, before) = closure();
    assert_eq!(before, compiled_executor_closure());
    let lock =
        fs::read_to_string(projects_root().join("sim-agent-net/Cargo.lock")).expect("runner lock");
    for name in ["sim-lib-journal", "sim-kernel", "sim-artifact-facet"] {
        assert!(
            !registry_checksums(&lock, name).is_empty(),
            "{name} has no registry checksum"
        );
    }
    for path in [
        "sim-agent-net/crates/sim-lib-roadmap-runner/src/journal.rs",
        "sim-agent-net/crates/sim-lib-roadmap-runner/src/causal.rs",
        "sim-agent-net/crates/sim-lib-roadmap-runner/src/codec.rs",
        "sim-agent-net/crates/sim-lib-roadmap-runner/src/local_emit.rs",
        "sim-agent-net/crates/sim-source-deck/src/build.rs",
        "sim-agent-net/crates/sim-source-deck/src/canonical.rs",
        "sim-agent-net/crates/sim-source-deck/src/types.rs",
        "sim-agent-net/crates/sim-source-deck/src/lib.rs",
    ] {
        assert!(
            EXECUTOR_CLOSURE_PATHS.contains(&path),
            "closure omitted {path}"
        );
    }
    let report = qualify(&before, &before).expect("specimen");
    let (_again, after) = closure();
    assert_eq!(before, after);
    assert_eq!(before.len(), 64);
    assert_eq!(report.first_page, 1);
    assert_eq!(report.retained_revision.len(), 64);
    assert_eq!(report.transitions, 1);
    assert_eq!(report.stale_effects, 0);
    assert_eq!(report.effects, 1);
    assert_eq!(report.hot_delta, 0);
    assert_eq!(
        report.providers,
        vec![
            "model/direction".to_owned(),
            "model/landing".to_owned(),
            "model/snapshot".to_owned(),
        ]
    );
    assert_eq!(report.stale_edges, 1);
    assert_eq!(report.source_bound, 1);
    assert!(!report.unrelated_proof.is_empty());
    assert!(qualify(&before, &"ab".repeat(32)).is_err());
    assert!(qualify("", "").is_err());
}

#[test]
fn the_specimen_attributes_the_requested_subject() {
    let digest = compiled_executor_closure();
    let report = qualify_subject(&digest, &digest, "subject/other").expect("specimen");
    assert_eq!(report.first_page, 1);
    let binding = find_pack("checker/c-port")
        .unwrap()
        .checker_binding()
        .unwrap();
    let rendered = render(binding.id().content_id());
    let evidence = MemorySubject::default()
        .with("executor-freeze-before", digest.clone())
        .with("executor-freeze-after", digest);
    let observations = check_subject(&sim_conformance_packs::PackRequest {
        checker: "checker/c-port",
        binding: &rendered,
        subject: "subject/other",
        scope: "portability/consumer-seams",
        evidence: &evidence,
    })
    .expect("scope");
    assert!(
        observations
            .iter()
            .any(|item| { item.key == "consumer-seams.subject" && item.value == "subject/other" })
    );
}

#[test]
fn each_seam_refusal_is_its_own_case() {
    let (page, revision) = snapshot_pages().expect("pages");
    assert_eq!(page, 1);
    assert_eq!(revision.len(), 64);
    assert_eq!(direction_once().expect("direction"), (1, 0));
    wrong_execution().expect("execution");
    changed_answer().expect("answer");
    advice_slot().expect("advice slot");
    let (effects, stale, proof) = selection_without_begin().expect("begin");
    assert_eq!(effects, 1);
    assert_eq!(stale, 1);
    assert!(!proof.is_empty());
    advice_rejected().expect("advice");
    reopen_same().expect("reopen");
    interrupt_once().expect("interrupt");
    revoked_subject().expect("revoked");
    let (hot_delta, hot_samples) = hot_noop().expect("hot");
    assert_eq!(hot_delta, 0);
    assert_eq!(hot_samples.len(), 20);
    assert_eq!(over_bound().expect("bound"), 1);
    scope_confusion().expect("scope");
    renote_stales().expect("renote");
    reopen_keeps_direction().expect("reopen direction");
    hot_needs_begin().expect("hot begin");
    selection_is_distinct().expect("selection");
    discover_affordance().expect("affordance");
    assert_eq!(open_with_linux(), Err(SeamFault::LinuxProvider));
}

#[test]
fn altered_closure_bytes_change_the_digest() {
    let left = capture_executor_subject_closure(&[("sim-kernel/src/lib.rs", b"one")]).unwrap();
    let right = capture_executor_subject_closure(&[("sim-kernel/src/lib.rs", b"two")]).unwrap();
    assert_ne!(left, right);
    assert_eq!(
        capture_executor_subject_closure(&[]).unwrap_err(),
        SeamFault::EmptyClosure
    );
    assert_eq!(
        capture_executor_subject_closure(&[("../escape.rs", b"x")]).unwrap_err(),
        SeamFault::BadPath
    );
    assert_eq!(
        capture_executor_subject_closure(&[("a\0\0\0\0\0\0\0\0\x09", b"")]).unwrap_err(),
        SeamFault::BadPath
    );
    let framed = capture_executor_subject_closure(&[("a", b"bc")]).unwrap();
    let other = capture_executor_subject_closure(&[("ab", b"c")]).unwrap();
    assert_ne!(framed, other);
}

#[test]
#[ignore = "requires sibling repository checkouts"]
fn the_pack_passes_only_the_frozen_closure() {
    let binding = find_pack("checker/c-port")
        .unwrap()
        .checker_binding()
        .unwrap();
    let rendered = render(binding.id().content_id());
    let (_files, digest) = closure();
    let evidence = MemorySubject::default()
        .with("executor-freeze-before", digest.clone())
        .with("executor-freeze-after", digest.clone());
    match sim_conformance_packs::packs::portability::check(&request(&evidence, &rendered)) {
        PackVerdict::Pass { observations, .. } => {
            assert!(
                observations
                    .iter()
                    .any(|item| { item.key == "consumer-seams.freeze" && item.value == digest })
            );
        }
        other => panic!("expected a pass, got {other:?}"),
    }
    let moved = MemorySubject::default()
        .with("executor-freeze-before", "a".repeat(64))
        .with("executor-freeze-after", "b".repeat(64));
    assert!(matches!(
        sim_conformance_packs::packs::portability::check(&request(&moved, &rendered)),
        PackVerdict::Refused(ref failure) if failure.code == "freeze-mismatch"
    ));
}

fn registry_checksums(lock: &str, name: &str) -> Vec<String> {
    let mut found = Vec::new();
    for block in lock.split("[[package]]").skip(1) {
        let Some(parsed) = block.lines().find_map(|line| {
            line.trim()
                .strip_prefix("name = ")
                .map(|value| value.trim().trim_matches('"').to_owned())
        }) else {
            continue;
        };
        if parsed != name {
            continue;
        }
        let checksum = block.lines().find_map(|line| {
            line.trim()
                .strip_prefix("checksum = ")
                .map(|value| value.trim().trim_matches('"').to_owned())
        });
        let Some(checksum) = checksum.filter(|value| {
            !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) else {
            panic!("{name} package block has no registry checksum");
        };
        found.push(checksum);
    }
    found
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}
