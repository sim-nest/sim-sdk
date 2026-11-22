// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-14` runs the prove command on the named program.
//!
//! The proof export freezes the forty-leaf denominator. D1 through D6 compare
//! that view. False reuse stays zero. D1, D2, and D4 reuse 100, 95, and 70
//! percent of the denominator. Native leaves stay excluded. An unavailable
//! portable leaf is a gap. Caller closure facts do not supply the scope.

use std::collections::BTreeSet;
use std::path::Path;

use crate::{CheckObservation, PackFailure, PackRequest};

#[path = "release_prove_record.rs"]
mod record;

use record::{
    Node, list_field, one, opened, require_bare, require_order, require_string, screen,
    string_field,
};

const HEADER: &str = "sim-world-proof-view-v1";
const DENOMINATOR: usize = 40;
const NATIVE: [&str; 2] = ["leaf/native-worker-adapter", "leaf/native-assay"];

const PROOF_ORDER: [&str; 11] = [
    "denominator",
    "executed",
    "requests",
    "edges",
    "effects",
    "subprocesses",
    "owner-commands",
    "resident",
    "subject",
    "failures",
    "view",
];
const REUSE_ORDER: [&str; 16] = [
    "delta",
    "denominator",
    "executed",
    "reused",
    "reused-leaves",
    "rerun",
    "reuse-basis-points",
    "false-reuse",
    "cold-requests",
    "reuse-requests",
    "revision-delta",
    "effects",
    "subprocesses",
    "owner-commands",
    "reuse-disabled",
    "gaps",
];
const PLAN_ORDER: [&str; 10] = [
    "denominator",
    "page",
    "pages",
    "leaves",
    "excluded",
    "effects",
    "subprocesses",
    "owner-commands",
    "resident",
    "continuation",
];
const STATUS_ORDER: [&str; 9] = [
    "denominator",
    "executed",
    "excluded",
    "requests",
    "effects",
    "subprocesses",
    "owner-commands",
    "resident",
    "gaps",
];
const WHY_ORDER: [&str; 9] = [
    "delta",
    "leaf",
    "reused",
    "false-reuse",
    "exercised",
    "digest",
    "requests",
    "effects",
    "subprocesses",
];
const FRONTIER_ORDER: [&str; 9] = [
    "complete",
    "failures",
    "gaps",
    "denominator",
    "executed",
    "requests",
    "effects",
    "subprocesses",
    "continuation",
];
const GAP_ORDER: [&str; 4] = ["call", "reason", "effects", "subprocesses"];

struct Law {
    delta: &'static str,
    basis: u16,
    requests: u64,
    revision: Option<u64>,
}

const LAWS: &[Law] = &[
    Law {
        delta: "D1",
        basis: 10_000,
        requests: 0,
        revision: None,
    },
    Law {
        delta: "D2",
        basis: 9_500,
        requests: 4,
        revision: None,
    },
    Law {
        delta: "D3",
        basis: 8_000,
        requests: 16,
        revision: None,
    },
    Law {
        delta: "D4",
        basis: 7_000,
        requests: 24,
        revision: None,
    },
    Law {
        delta: "D5",
        basis: 9_000,
        requests: 8,
        revision: None,
    },
    Law {
        delta: "D6",
        basis: 10_000,
        requests: 0,
        revision: Some(1),
    },
];

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = crate::packs::product::program(request)?;
    let proof = stable(&program, &["prove", "run", "proof"], "proof")?;
    let proof_node = opened(&proof, "prove/proof-v1")?;
    let view = require_proof(&proof_node)?;
    let mut out = vec![observation("release.prove.proof", &proof)];
    for law in LAWS {
        let stdout = stable(&program, &["prove", "run", law.delta, &view], law.delta)?;
        require_delta(&opened(&stdout, "prove/reuse-v1")?, law)?;
        out.push(observation(
            &format!("release.prove.{}", law.delta),
            &stdout,
        ));
    }
    let plan = stable(&program, &["prove", "plan", &view], "plan")?;
    let plan_node = opened(&plan, "prove/plan-v1")?;
    let plan_leaves = require_plan(&plan_node, "0")?;
    let plan_token = string_field(&plan_node, "continuation")?;
    out.push(observation("release.prove.plan", &plan));
    let next = stable(
        &program,
        &["prove", "resume", &plan_token, &view],
        "plan-next",
    )?;
    let next_leaves = require_plan(&opened(&next, "prove/plan-v1")?, "1")?;
    if plan_leaves == next_leaves {
        return Err(fail("record-mismatch", "leaves"));
    }
    out.push(observation("release.prove.plan-next", &next));
    let status = stable(&program, &["prove", "status", &view], "status")?;
    require_status(&opened(&status, "prove/status-v1")?)?;
    out.push(observation("release.prove.status", &status));
    let why_rerun = stable(
        &program,
        &["prove", "why", "D2", "conclusion/01", &view],
        "why-rerun",
    )?;
    require_why(
        &opened(&why_rerun, "prove/why-v1")?,
        "conclusion/01",
        false,
        false,
    )?;
    out.push(observation("release.prove.why-rerun", &why_rerun));
    let why_kept = stable(
        &program,
        &["prove", "why", "D2", "conclusion/40", &view],
        "why-kept",
    )?;
    require_why(
        &opened(&why_kept, "prove/why-v1")?,
        "conclusion/40",
        true,
        true,
    )?;
    out.push(observation("release.prove.why-kept", &why_kept));
    let frontier = stable(&program, &["prove", "frontier", &view], "frontier")?;
    let frontier_node = opened(&frontier, "prove/frontier-v1")?;
    let frontier_token = require_frontier(&frontier_node)?;
    out.push(observation("release.prove.frontier", &frontier));
    let resumed = stable(
        &program,
        &["prove", "resume", &frontier_token, &view],
        "resume",
    )?;
    if crate::packs::product::prove_record(&frontier)?
        != crate::packs::product::prove_record(&resumed)?
    {
        return Err(fail("record-mismatch", "resume"));
    }
    out.push(observation("release.prove.resume", &resumed));
    let unavailable = stable(
        &program,
        &["prove", "frontier", "conclusion/41", &view],
        "unavailable",
    )?;
    require_gap(
        &opened(&unavailable, "prove/gap-v1")?,
        "conclusion/41",
        "unavailable-proof",
    )?;
    out.push(observation("release.prove.unavailable", &unavailable));
    for leaf in NATIVE {
        let stdout = stable(&program, &["prove", "frontier", leaf, &view], "excluded")?;
        require_gap(&opened(&stdout, "prove/gap-v1")?, leaf, "excluded-scope")?;
        out.push(observation(
            &format!("release.prove.excluded.{leaf}"),
            &stdout,
        ));
    }
    Ok(out)
}

fn stable(program: &Path, args: &[&str], label: &str) -> Result<String, PackFailure> {
    let first = crate::packs::product::run(program, args)?;
    let second = crate::packs::product::run(program, args)?;
    screen(&first)?;
    screen(&second)?;
    if first != second {
        return Err(fail("observation-unstable", label));
    }
    Ok(first)
}

fn require_proof(node: &Node) -> Result<String, PackFailure> {
    require_order(node, &PROOF_ORDER)?;
    require_bare(node, "denominator", "40")?;
    require_bare(node, "executed", "40")?;
    require_number(node, "requests")?;
    require_number(node, "edges")?;
    require_bare(node, "effects", "0")?;
    require_bare(node, "subprocesses", "0")?;
    require_bare(node, "owner-commands", "0")?;
    require_bare(node, "resident", "true")?;
    if string_field(node, "subject")?.is_empty() {
        return Err(fail("record-mismatch", "subject"));
    }
    require_empty(node, "failures")?;
    let view = string_field(node, "view")?;
    if !view.starts_with(HEADER) {
        return Err(fail("marker-absent", HEADER));
    }
    Ok(view)
}

fn require_delta(node: &Node, law: &Law) -> Result<(), PackFailure> {
    require_order(node, &REUSE_ORDER)?;
    require_string(node, "delta", law.delta)?;
    require_bare(node, "denominator", "40")?;
    require_bare(node, "executed", "40")?;
    let reused = list_field(node, "reused-leaves")?;
    let rerun = list_field(node, "rerun")?;
    require_bare(node, "reused", &reused.len().to_string())?;
    require_partition(&reused, &rerun)?;
    require_rerun(law.delta, &rerun)?;
    let basis = usize::from(law.basis);
    if reused.len().saturating_mul(10_000) / DENOMINATOR != basis {
        return Err(fail("record-mismatch", "reuse-basis-points"));
    }
    require_bare(node, "reuse-basis-points", &law.basis.to_string())?;
    require_bare(node, "false-reuse", "0")?;
    require_number(node, "cold-requests")?;
    require_bare(node, "reuse-requests", &law.requests.to_string())?;
    match law.revision {
        Some(revision) => require_bare(node, "revision-delta", &revision.to_string())?,
        None => require_number(node, "revision-delta")?,
    }
    require_bare(node, "effects", "0")?;
    require_bare(node, "subprocesses", "0")?;
    require_bare(node, "owner-commands", "0")?;
    require_bare(node, "reuse-disabled", "false")?;
    require_empty(node, "gaps")
}

fn require_partition(reused: &[String], rerun: &[String]) -> Result<(), PackFailure> {
    let mut seen = BTreeSet::new();
    for name in reused.iter().chain(rerun) {
        if !seen.insert(name.as_str()) {
            return Err(fail("record-mismatch", "partition"));
        }
    }
    if seen.len() != DENOMINATOR {
        return Err(fail("record-mismatch", "partition"));
    }
    for index in 1..=DENOMINATOR {
        let leaf = format!("conclusion/{index:02}");
        if !seen.contains(leaf.as_str()) {
            return Err(fail("record-mismatch", "partition"));
        }
    }
    Ok(())
}

fn require_rerun(delta: &str, rerun: &[String]) -> Result<(), PackFailure> {
    let ok = match delta {
        "D1" | "D6" => rerun.is_empty(),
        "D2" => rerun == ["conclusion/01", "conclusion/02"],
        "D3" => {
            rerun.len() == 8
                && rerun.iter().any(|leaf| leaf == "conclusion/03")
                && rerun.iter().any(|leaf| leaf == "conclusion/10")
                && rerun.iter().all(|leaf| leaf != "conclusion/11")
        }
        "D4" => {
            rerun.len() == 12
                && rerun.iter().any(|leaf| leaf == "conclusion/11")
                && rerun.iter().any(|leaf| leaf == "conclusion/22")
        }
        "D5" => {
            rerun
                == [
                    "conclusion/23",
                    "conclusion/24",
                    "conclusion/25",
                    "conclusion/26",
                ]
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(fail("record-mismatch", "rerun"))
    }
}

fn require_plan(node: &Node, page: &str) -> Result<Vec<String>, PackFailure> {
    require_order(node, &PLAN_ORDER)?;
    require_bare(node, "denominator", "40")?;
    require_bare(node, "page", page)?;
    require_bare(node, "pages", "5")?;
    require_bare(node, "effects", "0")?;
    require_bare(node, "subprocesses", "0")?;
    require_bare(node, "owner-commands", "0")?;
    require_bare(node, "resident", "true")?;
    let leaves = list_field(node, "leaves")?;
    if leaves.len() != 8 || leaves.iter().any(|leaf| !leaf.starts_with("conclusion/")) {
        return Err(fail("record-mismatch", "leaves"));
    }
    require_names(node, "excluded", &NATIVE)?;
    require_hex(&string_field(node, "continuation")?)?;
    Ok(leaves)
}

fn require_status(node: &Node) -> Result<(), PackFailure> {
    require_order(node, &STATUS_ORDER)?;
    require_bare(node, "denominator", "40")?;
    require_bare(node, "executed", "40")?;
    require_names(node, "excluded", &NATIVE)?;
    require_number(node, "requests")?;
    require_bare(node, "effects", "0")?;
    require_bare(node, "subprocesses", "0")?;
    require_bare(node, "owner-commands", "0")?;
    require_bare(node, "resident", "true")?;
    require_empty(node, "gaps")
}

fn require_why(
    node: &Node,
    leaf: &str,
    reused: bool,
    exercised_empty: bool,
) -> Result<(), PackFailure> {
    require_order(node, &WHY_ORDER)?;
    require_string(node, "delta", "D2")?;
    require_string(node, "leaf", leaf)?;
    require_bare(node, "reused", if reused { "true" } else { "false" })?;
    require_bare(node, "false-reuse", "false")?;
    let exercised = list_field(node, "exercised")?;
    if exercised.is_empty() != exercised_empty {
        return Err(fail("record-mismatch", "exercised"));
    }
    if string_field(node, "digest")?.is_empty() {
        return Err(fail("record-mismatch", "digest"));
    }
    require_number(node, "requests")?;
    require_bare(node, "effects", "0")?;
    require_bare(node, "subprocesses", "0")
}

fn require_frontier(node: &Node) -> Result<String, PackFailure> {
    require_order(node, &FRONTIER_ORDER)?;
    require_bare(node, "complete", "true")?;
    require_empty(node, "failures")?;
    require_empty(node, "gaps")?;
    require_bare(node, "denominator", "40")?;
    require_bare(node, "executed", "40")?;
    require_number(node, "requests")?;
    require_bare(node, "effects", "0")?;
    require_bare(node, "subprocesses", "0")?;
    let token = string_field(node, "continuation")?;
    require_hex(&token)?;
    Ok(token)
}

fn require_gap(node: &Node, call: &str, reason: &str) -> Result<(), PackFailure> {
    require_order(node, &GAP_ORDER)?;
    require_string(node, "call", call)?;
    require_string(node, "reason", reason)?;
    require_bare(node, "effects", "0")?;
    require_bare(node, "subprocesses", "0")
}

fn require_names(node: &Node, name: &str, expected: &[&str]) -> Result<(), PackFailure> {
    let names = list_field(node, name)?;
    if names.len() == expected.len() && names.iter().zip(expected).all(|(got, want)| got == want) {
        Ok(())
    } else {
        Err(fail("record-mismatch", name))
    }
}

fn require_empty(node: &Node, name: &str) -> Result<(), PackFailure> {
    if list_field(node, name)?.is_empty() {
        Ok(())
    } else {
        Err(fail("record-mismatch", name))
    }
}

fn require_number(node: &Node, name: &str) -> Result<(), PackFailure> {
    let value = one(node, name)?;
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(fail("record-mismatch", name))
    }
}

fn require_hex(value: &str) -> Result<(), PackFailure> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(fail("record-mismatch", "continuation"))
    }
}

fn observation(key: &str, stdout: &str) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: stdout.to_owned(),
    }
}

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}
