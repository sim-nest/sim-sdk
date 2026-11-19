// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Cached query latency is the program's measurement, not a caller fact.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

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

fn execute(evidence: &MemorySubject) -> PackVerdict {
    let binding = find_pack("checker/c-product")
        .unwrap()
        .checker_binding()
        .unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    packs::product::check(&PackRequest {
        checker: "checker/c-product",
        binding,
        subject: SUBJECT,
        scope: "product/query-latency",
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
    MemorySubject::default().with(packs::product::COMMAND_FACT, path.display().to_string())
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sim-product-query-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sim(dir: &Path, line: &str) -> PathBuf {
    let path = dir.join("sim");
    let body = format!(
        "#!/bin/sh\nset -e\ncase \"$1:$2:$3\" in\nworld:measure:why)\n  printf '%s\\n' '{line}'\n  ;;\n*)\n  exit 1\n  ;;\nesac\n"
    );
    std::fs::write(&path, body).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn latency_line(p95_ms: u64, sample: u64, count: usize) -> String {
    let raw = std::iter::repeat_n(sample.to_string(), count)
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "#(world/query-latency-v1 (note \"(raw-us [1]) (command \\\"status\\\")\") (command \"why\") (records 100000) (units 10000) (warmups 20) (samples {count}) (p95-ms {p95_ms}) (raw-us [{raw}]) (effects 0) (subprocesses 0) (resident true))"
    )
}

fn pass_value(verdict: &PackVerdict) -> &str {
    match verdict {
        PackVerdict::Pass { observations, .. } => observations
            .iter()
            .find(|item| item.key == "product.query-latency")
            .unwrap_or_else(|| panic!("missing latency observation"))
            .value
            .as_str(),
        other => panic!("expected a pass, got {other:?}"),
    }
}

#[test]
fn caller_latency_facts_are_not_a_measurement() {
    let evidence = MemorySubject::default()
        .with("product.query-command", "world/why")
        .with("product.query-warmups-20", "true")
        .with("product.query-samples-200", "true")
        .with("product.query-p95-at-most-200ms", "true")
        .with("product.query-effect-calls-zero", "true");
    let (code, detail) = refusal(&execute(&evidence));
    assert_eq!(code, "unexpected-fact");
    assert_eq!(detail, "product.query-command");
}

#[test]
fn a_resident_measurement_is_the_latency_observation() {
    let dir = scratch("measured");
    let line = latency_line(0, 9, 200);
    let program = write_sim(&dir, &line);
    let verdict = execute(&command(&program));
    assert_eq!(pass_value(&verdict).trim_end(), line);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_p95_field_must_match_the_samples() {
    let dir = scratch("mismatch");
    let program = write_sim(&dir, &latency_line(1, 9, 200));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "p95-ms");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_p95_over_200ms_is_not_inside_the_ceiling() {
    let dir = scratch("ceiling");
    let program = write_sim(&dir, &latency_line(200, 200_001, 200));
    let (code, detail) = refusal(&execute(&command(&program)));
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "ceiling");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn installed_sim_records_query_latency() {
    let named = std::env::var_os("SIM_PRODUCT_BIN").map(PathBuf::from);
    let Some(program) = named.filter(|path| path.is_file()) else {
        let (code, _) = refusal(&execute(&command(Path::new("missing-sim-product-bin"))));
        assert_eq!(code, "missing-source");
        return;
    };
    let verdict = execute(&command(&program));
    let value = pass_value(&verdict);
    assert!(value.contains("#(world/query-latency-v1 "));
    assert!(value.contains("(command \"why\")"));
    assert!(value.contains("(records 100000)"));
    assert!(value.contains("(units 10000)"));
    assert!(value.contains("(warmups 20)"));
    assert!(value.contains("(samples 200)"));
    assert!(value.contains("(effects 0)"));
    assert!(value.contains("(subprocesses 0)"));
    assert!(!value.contains("completion/claim-"));
    assert!(!value.contains("PortableClosed"));
    assert!(!value.contains("derive_completion"));
}
