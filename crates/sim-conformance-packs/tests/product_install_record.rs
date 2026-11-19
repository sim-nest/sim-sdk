// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Installation comes from the printed promote records, not caller facts.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const REGISTRY: &str = "#(world/promote-v1 (case \"registry\") (effects 0) (needs 2) (packages 3) (resident true) (subprocesses 0))";
const PRIVATE: &str = "#(world/promote-gap-v1 (case \"\") (effects 0) (evidence \"\") (reason \"private-leakage\") (resident true) (subprocesses 0))";

struct Gap {
    case: &'static str,
    reason: &'static str,
}

const GAPS: &[Gap] = &[
    Gap {
        case: "path",
        reason: "path-source",
    },
    Gap {
        case: "receipt",
        reason: "local-receipt",
    },
    Gap {
        case: "model",
        reason: "model-plan",
    },
    Gap {
        case: "version",
        reason: "missing-version",
    },
    Gap {
        case: "checksum",
        reason: "missing-checksum",
    },
    Gap {
        case: "source",
        reason: "missing-source",
    },
    Gap {
        case: "order",
        reason: "dependency-order",
    },
    Gap {
        case: "revision",
        reason: "dependency-order",
    },
    Gap {
        case: "linux",
        reason: "linux-requirement",
    },
    Gap {
        case: "install",
        reason: "missing-install",
    },
    Gap {
        case: "unreconciled",
        reason: "unreconciled",
    },
    Gap {
        case: "overlap",
        reason: "overlap",
    },
    Gap {
        case: "empty",
        reason: "missing-package",
    },
    Gap {
        case: "ci",
        reason: "missing-ci",
    },
    Gap {
        case: "mirror",
        reason: "missing-mirror",
    },
];

struct Lines {
    registry: String,
    gaps: Vec<String>,
    private: String,
}

fn gap_line(case: &str, reason: &str) -> String {
    format!(
        "#(world/promote-gap-v1 (case \"{case}\") (effects 0) (evidence \"\") (reason \"{reason}\") (resident true) (subprocesses 0))"
    )
}

fn lawful() -> Lines {
    Lines {
        registry: REGISTRY.to_owned(),
        gaps: GAPS
            .iter()
            .map(|gap| gap_line(gap.case, gap.reason))
            .collect(),
        private: PRIVATE.to_owned(),
    }
}

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
        scope: "product/installed",
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
    let dir =
        std::env::temp_dir().join(format!("sim-install-record-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sim(dir: &Path, lines: &Lines) -> PathBuf {
    let mut body = String::from("#!/bin/sh\nset -e\ncase \"$1:$2:$3\" in\n");
    body.push_str(&format!(
        "world:promote:registry)\n  printf '%s\\n' '{}'\n  ;;\n",
        lines.registry
    ));
    for (gap, line) in GAPS.iter().zip(lines.gaps.iter()) {
        body.push_str(&format!(
            "world:promote:{})\n  printf '%s\\n' '{line}'\n  ;;\n",
            gap.case
        ));
    }
    body.push_str(&format!(
        "world:promote:private-material:s3cret-token)\n  printf '%s\\n' '{}'\n  ;;\n",
        lines.private
    ));
    body.push_str("*)\n  exit 1\n  ;;\nesac\n");
    let path = dir.join("sim");
    std::fs::write(&path, body).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn refuse_lines(name: &str, lines: &Lines) -> (String, String) {
    let dir = scratch(name);
    let program = write_sim(&dir, lines);
    refusal(&execute(&command(&program)))
}

fn value<'a>(verdict: &'a PackVerdict, key: &str) -> &'a str {
    match verdict {
        PackVerdict::Pass { observations, .. } => observations
            .iter()
            .find(|observation| observation.key == key)
            .map(|observation| observation.value.as_str())
            .unwrap_or_else(|| panic!("missing {key}")),
        other => panic!("expected a pass, got {other:?}"),
    }
}

#[test]
fn caller_facts_are_not_an_installation() {
    let (code, detail) = refusal(&execute(&MemorySubject::default()));
    assert_eq!(code, "missing-source");
    assert_eq!(detail, "product.command");
    let truths = MemorySubject::default().with("product.brochure-present", "true");
    let (code, detail) = refusal(&execute(&truths));
    assert_eq!(code, "unexpected-fact");
    assert_eq!(detail, "product.brochure-present");
}

#[test]
fn a_printed_record_is_the_installation_observation() {
    let dir = scratch("printed");
    let program = write_sim(&dir, &lawful());
    let verdict = execute(&command(&program));
    assert!(value(&verdict, "product.promote-registry").contains("(packages 3)"));
    assert!(value(&verdict, "product.promote-registry").contains("(needs 2)"));
    assert!(value(&verdict, "product.promote-path").contains("path-source"));
    assert!(value(&verdict, "product.promote-receipt").contains("local-receipt"));
    assert!(value(&verdict, "product.promote-order").contains("dependency-order"));
    assert!(value(&verdict, "product.promote-linux").contains("linux-requirement"));
    assert!(value(&verdict, "product.promote-install").contains("missing-install"));
    assert!(value(&verdict, "product.promote-private").contains("private-leakage"));
}

#[test]
fn a_changed_package_count_is_not_the_registry_set() {
    let mut lines = lawful();
    lines.registry = lines.registry.replace("(packages 3)", "(packages 4)");
    let (code, detail) = refuse_lines("packages", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "product.promote-registry");
}

#[test]
fn a_missing_package_count_is_not_the_registry_set() {
    let mut lines = lawful();
    lines.registry = lines.registry.replace(" (packages 3)", "");
    let (code, detail) = refuse_lines("missing-packages", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "product.promote-registry");
}

#[test]
fn a_negative_effect_count_is_not_zero() {
    let mut lines = lawful();
    lines.registry = lines.registry.replace("(effects 0)", "(effects -1)");
    let (code, detail) = refuse_lines("effects", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "effects");
}

#[test]
fn a_negative_subprocess_count_is_not_zero() {
    let mut lines = lawful();
    lines.gaps[0] = lines.gaps[0].replace("(subprocesses 0)", "(subprocesses -1)");
    let (code, detail) = refuse_lines("subprocesses", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "subprocesses");
}

#[test]
fn a_resident_phrase_is_not_resident() {
    let mut lines = lawful();
    lines.gaps[1] = lines.gaps[1].replace("(resident true)", "(resident \"not resident\")");
    let (code, detail) = refuse_lines("resident", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "resident");
}

#[test]
fn a_local_receipt_reason_is_not_a_path_source() {
    let mut lines = lawful();
    lines.gaps[0] = lines.gaps[0].replace("path-source", "local-receipt");
    let (code, detail) = refuse_lines("path", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "product.promote-path");
}

#[test]
fn a_copied_private_value_is_not_refused() {
    let mut lines = lawful();
    lines.private = lines.private.replace("private-leakage", "copied");
    let (code, detail) = refuse_lines("private", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "product.promote-private");
}

#[test]
fn a_second_record_on_the_same_line_is_not_the_installation() {
    let mut lines = lawful();
    lines.registry = format!("#(world/promote-v1){REGISTRY}");
    let (code, detail) = refuse_lines("glued", &lines);
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_empty_root_with_a_space_glued_to_a_second_record_is_extra() {
    let mut lines = lawful();
    lines.gaps[2] = format!("#(world/promote-gap-v1 ){}", lines.gaps[2]);
    let (code, detail) = refuse_lines("glued-space", &lines);
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_unclosed_root_is_not_a_record() {
    let mut lines = lawful();
    lines.registry = REGISTRY.strip_suffix(')').expect("root closes").to_owned();
    let (code, detail) = refuse_lines("unclosed", &lines);
    assert_eq!(code, "bad-record");
    assert_eq!(detail, "unclosed");
}

#[test]
fn installed_sim_records_the_publication_laws() {
    let Some(path) = std::env::var_os("SIM_PRODUCT_BIN") else {
        let (code, detail) = refusal(&execute(&MemorySubject::default()));
        assert_eq!(code, "missing-source");
        assert_eq!(detail, "product.command");
        return;
    };
    let program = PathBuf::from(path);
    assert!(program.is_file(), "installed sim is missing");
    let verdict = execute(&command(&program));
    let registry = value(&verdict, "product.promote-registry");
    assert!(registry.contains("#(world/promote-v1 "));
    assert!(registry.contains("(packages 3)"));
    assert!(registry.contains("(needs 2)"));
    assert!(value(&verdict, "product.promote-path").contains("path-source"));
    assert!(value(&verdict, "product.promote-receipt").contains("local-receipt"));
    assert!(value(&verdict, "product.promote-model").contains("model-plan"));
    assert!(value(&verdict, "product.promote-version").contains("missing-version"));
    assert!(value(&verdict, "product.promote-checksum").contains("missing-checksum"));
    assert!(value(&verdict, "product.promote-source").contains("missing-source"));
    assert!(value(&verdict, "product.promote-order").contains("dependency-order"));
    assert!(value(&verdict, "product.promote-revision").contains("dependency-order"));
    assert!(value(&verdict, "product.promote-linux").contains("linux-requirement"));
    assert!(value(&verdict, "product.promote-install").contains("missing-install"));
    assert!(value(&verdict, "product.promote-unreconciled").contains("(reason \"unreconciled\")"));
    assert!(value(&verdict, "product.promote-overlap").contains("(reason \"overlap\")"));
    assert!(value(&verdict, "product.promote-empty").contains("missing-package"));
    assert!(value(&verdict, "product.promote-ci").contains("missing-ci"));
    assert!(value(&verdict, "product.promote-mirror").contains("missing-mirror"));
    let private = value(&verdict, "product.promote-private");
    assert!(private.contains("(reason \"private-leakage\")"));
    assert!(!private.contains("private-material:"));
    assert!(!registry.contains("completion/claim-"));
    assert!(!registry.contains("PortableClosed"));
    assert!(!registry.contains("derive_completion"));
}
