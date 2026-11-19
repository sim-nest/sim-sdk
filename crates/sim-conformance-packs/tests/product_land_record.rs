// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Landing queries come from the printed land records, not caller facts.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack, packs};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const FOREIGN: &str = "#(world/land-v1 (case \"foreign\") (detail \"would-commit applied kept same 1\") (effects 0) (invokes 3) (resident true) (subprocesses 0))";
const REPLAY: &str = "#(world/land-v1 (case \"replay\") (detail \"already-true same 1\") (effects 0) (invokes 2) (resident true) (subprocesses 0))";
const RECOVER: &str = "#(world/land-v1 (case \"recover\") (detail \"restored exact kept 1\") (effects 0) (invokes 3) (resident true) (subprocesses 0))";
const GENERATED: &str = "#(world/land-v1 (case \"generated\") (detail \"generated-owner-bypass same 1\") (effects 0) (invokes 2) (resident true) (subprocesses 0))";
const CACHE: &str = "#(world/land-v1 (case \"cache\") (detail \"unknown current stale same 1\") (effects 0) (invokes 6) (resident true) (subprocesses 0))";
const STATUS: &str = "#(world/land-v1 (case \"status\") (detail \"present missing same 1\") (effects 0) (invokes 2) (resident true) (subprocesses 0))";
const PRIVATE: &str = "#(world/land-gap-v1 (case \"\") (effects 0) (evidence \"\") (reason \"private-leakage\") (resident true) (subprocesses 0))";

struct Lines {
    foreign: String,
    replay: String,
    recover: String,
    generated: String,
    cache: String,
    status: String,
    private: String,
}

fn lawful() -> Lines {
    Lines {
        foreign: FOREIGN.to_owned(),
        replay: REPLAY.to_owned(),
        recover: RECOVER.to_owned(),
        generated: GENERATED.to_owned(),
        cache: CACHE.to_owned(),
        status: STATUS.to_owned(),
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
        scope: "product/landing-queries",
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
    let dir = std::env::temp_dir().join(format!("sim-land-record-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_sim(dir: &Path, lines: &Lines) -> PathBuf {
    let path = dir.join("sim");
    let body = format!(
        "#!/bin/sh\nset -e\ncase \"$1:$2:$3\" in\nworld:land:foreign)\n  printf '%s\\n' '{foreign}'\n  ;;\nworld:land:replay)\n  printf '%s\\n' '{replay}'\n  ;;\nworld:land:recover)\n  printf '%s\\n' '{recover}'\n  ;;\nworld:land:generated)\n  printf '%s\\n' '{generated}'\n  ;;\nworld:land:cache)\n  printf '%s\\n' '{cache}'\n  ;;\nworld:land:status)\n  printf '%s\\n' '{status}'\n  ;;\nworld:land:private-material:s3cret-token)\n  printf '%s\\n' '{private}'\n  ;;\n*)\n  exit 1\n  ;;\nesac\n",
        foreign = lines.foreign,
        replay = lines.replay,
        recover = lines.recover,
        generated = lines.generated,
        cache = lines.cache,
        status = lines.status,
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
fn caller_facts_are_not_a_landing_query() {
    let (code, detail) = refusal(&execute(&MemorySubject::default()));
    assert_eq!(code, "missing-source");
    assert_eq!(detail, "product.command");
    let truths = MemorySubject::default().with("product.brochure-present", "true");
    let (code, detail) = refusal(&execute(&truths));
    assert_eq!(code, "unexpected-fact");
    assert_eq!(detail, "product.brochure-present");
}

#[test]
fn a_printed_record_is_the_landing_observation() {
    let dir = scratch("printed");
    let program = write_sim(&dir, &lawful());
    let verdict = execute(&command(&program));
    assert!(value(&verdict, "product.land-foreign").contains("(case \"foreign\")"));
    assert!(value(&verdict, "product.land-replay").contains("already-true"));
    assert!(value(&verdict, "product.land-recover").contains("(case \"recover\")"));
    assert!(value(&verdict, "product.land-generated").contains("generated-owner-bypass"));
    assert!(value(&verdict, "product.land-cache").contains("stale"));
    assert!(value(&verdict, "product.land-status").contains("(case \"status\")"));
    assert!(value(&verdict, "product.land-private").contains("private-leakage"));
}

#[test]
fn a_changed_foreign_detail_is_not_the_accepted_revision() {
    let mut lines = lawful();
    lines.foreign = lines
        .foreign
        .replace("would-commit applied", "would-drop applied");
    let (code, detail) = refuse_lines("foreign", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "product.land-foreign");
}

#[test]
fn a_negative_effect_count_is_not_zero() {
    let mut lines = lawful();
    lines.foreign = lines.foreign.replace("(effects 0)", "(effects -1)");
    let (code, detail) = refuse_lines("effects", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "effects");
}

#[test]
fn a_negative_subprocess_count_is_not_zero() {
    let mut lines = lawful();
    lines.replay = lines
        .replay
        .replace("(subprocesses 0)", "(subprocesses -1)");
    let (code, detail) = refuse_lines("subprocesses", &lines);
    assert_eq!(code, "record-mismatch");
    assert_eq!(detail, "subprocesses");
}

#[test]
fn a_resident_phrase_is_not_resident() {
    let mut lines = lawful();
    lines.recover = lines
        .recover
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
    assert_eq!(detail, "product.land-private");
}

#[test]
fn a_second_record_on_the_same_line_is_not_the_query() {
    let mut lines = lawful();
    lines.foreign = format!("#(world/land-v1){FOREIGN}");
    let (code, detail) = refuse_lines("glued", &lines);
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_empty_root_with_a_space_glued_to_a_second_record_is_extra() {
    let mut lines = lawful();
    lines.cache = format!("#(world/land-v1 ){CACHE}");
    let (code, detail) = refuse_lines("glued-space", &lines);
    assert_eq!(code, "marker-absent");
    assert_eq!(detail, "extra");
}

#[test]
fn an_unclosed_root_is_not_a_record() {
    let mut lines = lawful();
    lines.status = STATUS.strip_suffix(')').expect("root closes").to_owned();
    let (code, detail) = refuse_lines("unclosed", &lines);
    assert_eq!(code, "bad-record");
    assert_eq!(detail, "unclosed");
}

#[test]
fn installed_sim_records_the_landing_laws() {
    let Some(path) = std::env::var_os("SIM_PRODUCT_BIN") else {
        let (code, detail) = refusal(&execute(&MemorySubject::default()));
        assert_eq!(code, "missing-source");
        assert_eq!(detail, "product.command");
        return;
    };
    let program = PathBuf::from(path);
    assert!(program.is_file(), "installed sim is missing");
    let verdict = execute(&command(&program));
    let foreign = value(&verdict, "product.land-foreign");
    let private = value(&verdict, "product.land-private");
    assert!(foreign.contains("#(world/land-v1 "));
    assert!(foreign.contains("would-commit applied"));
    assert!(foreign.contains("(invokes 3)"));
    assert!(value(&verdict, "product.land-replay").contains("already-true"));
    assert!(value(&verdict, "product.land-recover").contains("restored exact"));
    assert!(value(&verdict, "product.land-generated").contains("generated-owner-bypass"));
    assert!(value(&verdict, "product.land-cache").contains("stale"));
    assert!(value(&verdict, "product.land-status").contains("present missing"));
    assert!(private.contains("(reason \"private-leakage\")"));
    assert!(!private.contains("private-material:"));
    assert!(!foreign.contains("completion/claim-"));
    assert!(!foreign.contains("PortableClosed"));
    assert!(!foreign.contains("derive_completion"));
}
