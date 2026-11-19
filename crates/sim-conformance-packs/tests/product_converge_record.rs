// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Convergence queries come from the printed converge records, not caller facts.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PRESERVE: &str = "#(world/converge-v1 (case \"preserve\") (detail \"self-approval held 1 uncertain 1 same 1 later uncertain\") (effects 0) (invokes 5) (resident true) (subprocesses 0))";
const INVALIDATE: &str = "#(world/converge-v1 (case \"invalidate\") (detail \"affected a closed b changed 1\") (effects 0) (invokes 3) (resident true) (subprocesses 0))";
const OSCILLATE: &str = "#(world/converge-v1 (case \"oscillate\") (detail \"empty 1 seen note same 1\") (effects 0) (invokes 5) (resident true) (subprocesses 0))";
const PARALLEL: &str = "#(world/converge-v1 (case \"parallel\") (detail \"same 1 failures a:proof-failed,b:proof-failed\") (effects 0) (invokes 2) (resident true) (subprocesses 0))";
const HONEST: &str = "#(world/converge-v1 (case \"honest\") (detail \"incomplete need malformed bad same 1\") (effects 0) (invokes 4) (resident true) (subprocesses 0))";
const PRIVATE: &str = "#(world/converge-gap-v1 (case \"\") (effects 0) (evidence \"\") (reason \"private-leakage\") (resident true) (subprocesses 0))";

struct Lines {
    preserve: String,
    invalidate: String,
    oscillate: String,
    parallel: String,
    honest: String,
    private: String,
}

fn lawful() -> Lines {
    Lines {
        preserve: PRESERVE.to_owned(),
        invalidate: INVALIDATE.to_owned(),
        oscillate: OSCILLATE.to_owned(),
        parallel: PARALLEL.to_owned(),
        honest: HONEST.to_owned(),
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
        scope: "product/convergence-queries",
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
        std::env::temp_dir().join(format!("sim-converge-record-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sim(dir: &Path, lines: &Lines) -> PathBuf {
    let path = dir.join("sim");
    let body = format!(
        "#!/bin/sh\nset -e\ncase \"$1:$2:$3\" in\nworld:converge:preserve)\n  printf '%s\\n' '{preserve}'\n  ;;\nworld:converge:invalidate)\n  printf '%s\\n' '{invalidate}'\n  ;;\nworld:converge:oscillate)\n  printf '%s\\n' '{oscillate}'\n  ;;\nworld:converge:parallel)\n  printf '%s\\n' '{parallel}'\n  ;;\nworld:converge:honest)\n  printf '%s\\n' '{honest}'\n  ;;\nworld:converge:private-material:s3cret-token)\n  printf '%s\\n' '{private}'\n  ;;\n*)\n  exit 1\n  ;;\nesac\n",
        preserve = lines.preserve,
        invalidate = lines.invalidate,
        oscillate = lines.oscillate,
        parallel = lines.parallel,
        honest = lines.honest,
        private = lines.private,
    );
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
fn caller_facts_are_not_a_convergence_query() {
    let (code, detail) = refusal(&execute(&MemorySubject::default()));
    assert_eq!(code, "missing-source");
    assert_eq!(detail, "product.command");
    let truths = MemorySubject::default().with("product.brochure-present", "true");
    let (code, detail) = refusal(&execute(&truths));
    assert_eq!(code, "unexpected-fact");
    assert_eq!(detail, "product.brochure-present");
}

#[test]
fn a_printed_record_is_the_convergence_observation() {
    let dir = scratch("printed");
    let program = write_sim(&dir, &lawful());
    let verdict = execute(&command(&program));
    assert!(value(&verdict, "product.converge-preserve").contains("(case \"preserve\")"));
    assert!(value(&verdict, "product.converge-invalidate").contains("affected a closed b"));
    assert!(value(&verdict, "product.converge-oscillate").contains("(case \"oscillate\")"));
    assert!(value(&verdict, "product.converge-parallel").contains("proof-failed"));
    assert!(value(&verdict, "product.converge-honest").contains("(case \"honest\")"));
    assert!(value(&verdict, "product.converge-private").contains("private-leakage"));
}

#[test]
fn a_changed_preserve_detail_is_not_the_accepted_revision() {
    let mut lines = lawful();
    lines.preserve = lines
        .preserve
        .replace("self-approval held 1", "self-approval held 0");
    let (code, detail) = refuse_lines("preserve", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "product.converge-preserve");
}

#[test]
fn a_negative_effect_count_is_not_zero() {
    let mut lines = lawful();
    lines.preserve = lines.preserve.replace("(effects 0)", "(effects -1)");
    let (code, detail) = refuse_lines("effects", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "effects");
}

#[test]
fn a_negative_subprocess_count_is_not_zero() {
    let mut lines = lawful();
    lines.invalidate = lines
        .invalidate
        .replace("(subprocesses 0)", "(subprocesses -1)");
    let (code, detail) = refuse_lines("subprocesses", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "subprocesses");
}

#[test]
fn a_resident_phrase_is_not_resident() {
    let mut lines = lawful();
    lines.oscillate = lines
        .oscillate
        .replace("(resident true)", "(resident \"not resident\")");
    let (code, detail) = refuse_lines("resident", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "resident");
}

#[test]
fn a_copied_private_value_is_not_refused() {
    let mut lines = lawful();
    lines.private = lines.private.replace("private-leakage", "copied");
    let (code, detail) = refuse_lines("private", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "product.converge-private");
}

#[test]
fn a_second_record_on_the_same_line_is_not_the_query() {
    let mut lines = lawful();
    lines.preserve = format!("#(world/converge-v1){PRESERVE}");
    let (code, detail) = refuse_lines("glued", &lines);
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_empty_root_with_a_space_glued_to_a_second_record_is_extra() {
    let mut lines = lawful();
    lines.parallel = format!("#(world/converge-v1 ){PARALLEL}");
    let (code, detail) = refuse_lines("glued-space", &lines);
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_unclosed_root_is_not_a_record() {
    let mut lines = lawful();
    lines.honest = HONEST.strip_suffix(')').expect("root closes").to_owned();
    let (code, detail) = refuse_lines("unclosed", &lines);
    assert_eq!(code, "bad-record");
    assert_eq!(detail, "unclosed");
}

#[test]
fn installed_sim_records_the_convergence_laws() {
    let Some(path) = std::env::var_os("SIM_PRODUCT_BIN") else {
        let (code, detail) = refusal(&execute(&MemorySubject::default()));
        assert_eq!(code, "missing-source");
        assert_eq!(detail, "product.command");
        return;
    };
    let program = PathBuf::from(path);
    assert!(program.is_file(), "installed sim is missing");
    let verdict = execute(&command(&program));
    let preserve = value(&verdict, "product.converge-preserve");
    let private = value(&verdict, "product.converge-private");
    assert!(preserve.contains("#(world/converge-v1 "));
    assert!(preserve.contains("self-approval held 1"));
    assert!(preserve.contains("(invokes 5)"));
    assert!(value(&verdict, "product.converge-invalidate").contains("closed b"));
    assert!(value(&verdict, "product.converge-oscillate").contains("seen note"));
    assert!(value(&verdict, "product.converge-parallel").contains("proof-failed"));
    assert!(value(&verdict, "product.converge-honest").contains("malformed"));
    assert!(private.contains("(reason \"private-leakage\")"));
    assert!(!private.contains("private-material:"));
    assert!(!preserve.contains("completion/claim-"));
    assert!(!preserve.contains("PortableClosed"));
    assert!(!preserve.contains("derive_completion"));
}
