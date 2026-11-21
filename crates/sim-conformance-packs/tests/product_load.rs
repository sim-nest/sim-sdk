// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The joined product load runs the named program twice and reads specimens.
//! A caller label is not `completion/claim-18`.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, all_packs, find_pack, packs};
use sim_conformance_v02::CheckScopeId;

const SUBJECT: &str = "subject/product-load";
const ACTIVATION: &str =
    "core/sha256-datum-v1:b9baa94a3ae79f1b3fdd30f73c10684c7f7ebe1802b19dfea8ee8c91b1d3f3a5";
const RECEIPT: &str = "#(prove/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/prove-command\"))";
const ROADMAP: &str = "#(roadmap/local-result-v1 (outcome \"ok\") (execution \"local/no-model\") (conduct \"conduct/local\") (model-pick \"model/local\") (proof-catalog \"proof/local\") (runner-generation \"generation/local\") (journal-acknowledged true) (detail \"execution not started\"))";
const FRAME: &str = "without placing its behavior in the frame.";
const WORLD_TASK: &str = "Project supplied semantic facts and explain their exact affected closure";
const PROVE_TASK: &str = "Run a cold proof and read its frontier";
const DELIVER_TASK: &str = "Read a delivery record through the installed world command";
const ROADMAP_TASK: &str = "Read the local roadmap status record";
const WORLD_ID: &str = "route/run-world-command";
const PROVE_ID: &str = "route/run-prove-command";
const DELIVER_ID: &str = "route/run-deliver-command";
const ROADMAP_ID: &str = "route/run-roadmap-command";
const WORLD_SPECIMEN: &str = "recipe/sim-runtime/crates/sim-lib-world/01-projection/world-explain";
const PROVE_SPECIMEN: &str = "spec-test/sim-run/crates/sim-run/tests/prove_gate";
const DELIVER_SPECIMEN: &str = "spec-test/sim-run/crates/sim-run/tests/deliver_gate";
const ROADMAP_SPECIMEN: &str = "spec-test/sim-run/crates/sim-run/tests/roadmap";

#[test]
fn the_joined_load_admits_claim_18() {
    let made = fixture("pass", "pass", true, true);
    let spec = find_pack("checker/c-product").expect("product");
    assert_eq!(all_packs().len(), 21);
    assert_eq!(spec.activation_binding, ACTIVATION);
    assert!(spec.allowed_scopes.contains(&"completion/product-load"));
    let evidence = facts(&made.sim, &made.root);
    let verdict = packs::product::check(&request("completion/product-load", &evidence));
    let PackVerdict::Pass { observations, .. } = &verdict else {
        panic!("expected a pass, got {verdict:?}");
    };
    assert!(
        observations
            .iter()
            .all(|item| !item.key.contains("completion/claim-")
                && !item.value.contains("completion/claim-"))
    );
    assert!(observations.iter().any(
        |item| item.key == "product.load.world" && item.value.contains("world/explanation-v1")
    ));
    let observed =
        packs::product::observe_product_load(SUBJECT, &made.sim, &made.root).expect("admit");
    assert_eq!(observed.result, "completion/claim-18");
    assert_eq!(observed.subject, SUBJECT);
    assert!(!observed.execution.contains("completion/claim-"));
    assert!(observed.execution.contains("world/explanation-v1"));
    assert!(observed.execution.contains(WORLD_ID));
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("completion/product-load").expect("scope")
    );
    packs::product::bind_product_load(&observed).expect("bind");
    let mut forged = observed;
    forged.result = "completion/claim-15".to_owned();
    assert_eq!(
        packs::product::bind_product_load(&forged)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
    forged.result = "completion/claim-18".to_owned();
    forged.execution = "completion/claim-18".to_owned();
    assert_eq!(
        packs::product::bind_product_load(&forged)
            .expect_err("labelled execution")
            .code,
        "claim-relabelled"
    );
}

#[test]
fn a_second_run_that_differs_is_unstable() {
    let made = fixture("unstable", "unstable", true, true);
    let error =
        packs::product::observe_product_load(SUBJECT, &made.sim, &made.root).expect_err("unstable");
    assert_eq!(error.code, "unstable");
}

#[test]
fn a_missing_brochure_is_refused() {
    let made = fixture("brochure", "pass", false, true);
    let error =
        packs::product::observe_product_load(SUBJECT, &made.sim, &made.root).expect_err("brochure");
    assert_eq!(error.code, "brochure-absent");
}

#[test]
fn a_missing_specimen_is_refused() {
    let made = fixture("specimen", "pass", true, false);
    let error =
        packs::product::observe_product_load(SUBJECT, &made.sim, &made.root).expect_err("specimen");
    assert_eq!(error.code, "specimen-absent");
}

#[test]
fn an_unreadable_specimen_is_refused() {
    let made = fixture("unreadable", "pass", true, true);
    let specimen = world_recipe(&made.root);
    let mut permissions = std::fs::metadata(&specimen).unwrap().permissions();
    permissions.set_mode(0o0);
    std::fs::set_permissions(&specimen, permissions).unwrap();
    let error = packs::product::observe_product_load(SUBJECT, &made.sim, &made.root)
        .expect_err("unreadable");
    assert_eq!(error.code, "specimen-absent");
    let mut restored = std::fs::metadata(&specimen).unwrap().permissions();
    restored.set_mode(0o644);
    std::fs::set_permissions(&specimen, restored).unwrap();
}

#[test]
fn an_empty_specimen_is_refused() {
    let made = fixture("empty", "pass", true, true);
    std::fs::write(world_recipe(&made.root), "").unwrap();
    let error =
        packs::product::observe_product_load(SUBJECT, &made.sim, &made.root).expect_err("empty");
    assert_eq!(error.code, "specimen-absent");
}

#[test]
fn a_route_without_the_frame_phrase_is_refused() {
    let made = fixture("frame", "frame", true, true);
    let error =
        packs::product::observe_product_load(SUBJECT, &made.sim, &made.root).expect_err("frame");
    assert_eq!(error.code, "marker-absent");
    assert_eq!(error.detail, "frame");
}

#[test]
fn a_specimen_that_leaves_the_root_is_refused() {
    let made = fixture("traversal", "traversal", true, true);
    let error = packs::product::observe_product_load(SUBJECT, &made.sim, &made.root)
        .expect_err("traversal");
    assert_eq!(error.code, "specimen-absent");
}

#[test]
fn a_claim_label_supplies_nothing() {
    let made = fixture("label", "pass", true, true);
    let error = packs::product::observe_product_load("completion/claim-18", &made.sim, &made.root)
        .expect_err("subject");
    assert_eq!(error.code, "bad-record");
    let mut evidence = facts(&made.sim, &made.root);
    evidence = evidence.with(packs::product::COMMAND_FACT, "completion/claim-18");
    let (code, _) = refusal(&packs::product::check(&request(
        "completion/product-load",
        &evidence,
    )));
    assert_eq!(code, "claim-relabelled");
    let extra = facts(&made.sim, &made.root).with("product.boot-smoke-passed", "true");
    let (code, _) = refusal(&packs::product::check(&request(
        "completion/product-load",
        &extra,
    )));
    assert_eq!(code, "unexpected-fact");
    let labelled_facts = facts(&made.sim, &made.root);
    let labelled = request_subject(
        "completion/claim-18",
        "completion/product-load",
        &labelled_facts,
    );
    let (code, detail) = refusal(&packs::product::check(&labelled));
    assert_eq!(code, "malformed-reference");
    assert_eq!(detail, "subject");
}

struct Made {
    sim: PathBuf,
    root: PathBuf,
}

fn fixture(name: &str, kind: &str, brochure: bool, specimen: bool) -> Made {
    let dir = scratch(name);
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
    let root = dir.join("tree");
    write_tree(&root, brochure, specimen);
    let sim = write_sim(&dir, &script(kind, &dir.join("count")));
    Made { sim, root }
}

fn write_tree(root: &Path, brochure: bool, specimen: bool) {
    let world = root.join("sim-runtime/crates/sim-lib-world");
    let run = root.join("sim-run/crates/sim-run");
    std::fs::create_dir_all(world.join("recipes/01-projection/world-explain")).unwrap();
    std::fs::create_dir_all(run.join("tests")).unwrap();
    if specimen {
        std::fs::write(
            world.join("recipes/01-projection/world-explain/recipe.toml"),
            "name = \"world-explain\"\n",
        )
        .unwrap();
        std::fs::write(run.join("tests/prove_gate.rs"), "// prove\n").unwrap();
        std::fs::write(run.join("tests/deliver_gate.rs"), "// deliver\n").unwrap();
        std::fs::write(run.join("tests/roadmap.rs"), "// roadmap\n").unwrap();
    }
    if brochure {
        let text = "In one line: Load the product through the bootloader.\n";
        std::fs::write(world.join("BROCHURE.md"), text).unwrap();
        std::fs::write(run.join("BROCHURE.md"), text).unwrap();
    }
}

fn script(kind: &str, count: &Path) -> String {
    let world_route = match kind {
        "frame" => format!(
            "{}\n{}",
            route(
                WORLD_ID,
                WORLD_SPECIMEN,
                "The row loads the product in the kernel."
            ),
            route(
                "route/decoy",
                "recipe/sim-runtime/crates/sim-lib-world/01-other/other",
                &format!("decoy {FRAME}"),
            )
        ),
        "traversal" => route(
            WORLD_ID,
            "recipe/sim-runtime/crates/../sim-lib-world/01-projection/world-explain",
            &format!("The world row loads the product {FRAME}"),
        ),
        _ => route(
            WORLD_ID,
            WORLD_SPECIMEN,
            &format!("The world row loads the product {FRAME}"),
        ),
    };
    let count = quote(count);
    let why = if kind == "unstable" {
        r##"n=0
  if [ -f COUNT ]; then n=$(cat COUNT); fi
  n=$((n + 1))
  printf '%s\n' "$n" > COUNT
  printf '%s\n' "#(world/explanation-v1 owner/sim-private/disclosure $n)"
"##
        .replace("COUNT", &count)
    } else {
        "printf '%s\\n' '#(world/explanation-v1 (path [\"owner/sim-private/disclosure\"]))'"
            .to_owned()
    };
    format!(
        r#"cmd="$1"
sub="$2"
receipt='{RECEIPT}'
roadmap='{ROADMAP}'
case "$cmd:$sub" in
world:why)
  {why}
  ;;
world:deliver)
  printf '%s\n' '#(world/deliver-v1 (realization "model") (effects 0) (subprocesses 0) (resident true) (recovered true))'
  ;;
prove:plan)
  printf '%s\n' '#(prove/plan-v1 (denominator 29) (excluded ["leaf/native-worker-adapter"]))'
  printf '%s\n' "$receipt"
  ;;
roadmap:status)
  printf '%s\n' "$roadmap"
  ;;
index:route)
  case "$3" in
  '{WORLD_TASK}')
    printf '%s\n' '{world_route}'
    ;;
  '{PROVE_TASK}')
    printf '%s\n' '{prove_route}'
    ;;
  '{DELIVER_TASK}')
    printf '%s\n' '{deliver_route}'
    ;;
  '{ROADMAP_TASK}')
    printf '%s\n' '{roadmap_route}'
    ;;
  esac
  ;;
esac
"#,
        prove_route = shell_quote(&route(
            PROVE_ID,
            PROVE_SPECIMEN,
            &format!("The prove row loads the product {FRAME}"),
        )),
        deliver_route = shell_quote(&route(
            DELIVER_ID,
            DELIVER_SPECIMEN,
            &format!("The deliver row loads the product {FRAME}"),
        )),
        roadmap_route = shell_quote(&route(
            ROADMAP_ID,
            ROADMAP_SPECIMEN,
            &format!("The roadmap row loads the product {FRAME}"),
        )),
        world_route = shell_quote(&world_route),
    )
}

fn route(id: &str, specimen: &str, why: &str) -> String {
    format!(
        "{id}\tTitle\tuser,code,framework\n  feature\tfeature/sim-run/bootloader\tBootloader\n    The standard bootloader selects command libraries by verb.\n  feature\tfeature/sim-run/product\tProduct\n    {why}\n  specimen\t{specimen}\tSpecimen\n    The specimen is the checked file."
    )
}

fn shell_quote(text: &str) -> String {
    text.replace('\'', "'\\''")
}

fn facts(sim: &Path, root: &Path) -> MemorySubject {
    MemorySubject::default()
        .with(packs::product::COMMAND_FACT, sim.display().to_string())
        .with("product.root", root.display().to_string())
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn request<'a>(scope: &'a str, evidence: &'a MemorySubject) -> PackRequest<'a> {
    request_subject(
        "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        scope,
        evidence,
    )
}

fn request_subject<'a>(
    subject: &'a str,
    scope: &'a str,
    evidence: &'a MemorySubject,
) -> PackRequest<'a> {
    let binding = find_pack("checker/c-product")
        .unwrap()
        .checker_binding()
        .unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    PackRequest {
        checker: "checker/c-product",
        binding,
        subject,
        scope,
        evidence,
    }
}

fn refusal(verdict: &PackVerdict) -> (String, String) {
    match verdict {
        PackVerdict::Refused(failure) => (failure.code.to_owned(), failure.detail.clone()),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn world_recipe(root: &Path) -> PathBuf {
    root.join("sim-runtime/crates/sim-lib-world/recipes/01-projection/world-explain/recipe.toml")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sim-product-load-{name}-{}", std::process::id()));
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
