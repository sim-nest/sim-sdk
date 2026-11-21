// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Executes the installed `sim` program for the loadable product scopes.
//!
//! The subject names the program. The observations are that program's output.
//! The program runs in the checkout that contains it, because prove reads that
//! checkout. A boolean fact is not an execution.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::COMMAND_FACT;
use crate::{CheckObservation, PackFailure, PackRequest};

const MAX_OUTPUT: usize = 16 * 1024 * 1024;
const CHUNK: usize = 100_000;
pub(super) const LOAD_RECEIPT: &str = "#(prove/load-receipt-v1 (provider \"platform/site/model\") (loader \"loader/static-v1\") (artifact \"lib/prove-command\"))";
pub(super) const ROADMAP_LINE: &str = "#(roadmap/local-result-v1 (outcome \"ok\") (execution \"local/no-model\") (conduct \"conduct/local\") (model-pick \"model/local\") (proof-catalog \"proof/local\") (runner-generation \"generation/local\") (journal-acknowledged true) (detail \"execution not started\"))";

pub(super) fn world(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = program(request)?;
    let why = require(
        &program,
        &[
            "world",
            "why",
            "conclusion/public-release",
            "policy/no-v3-disclosure",
        ],
        &["world/explanation-v1", "owner/sim-private/disclosure"],
        &[],
    )?;
    let project = require(
        &program,
        &[
            "world",
            "project",
            "world/public-api-v1",
            "source/public-api",
            "pub fn project()",
        ],
        &["world/projection-v1", "conclusion/source-api"],
        &[],
    )?;
    let diff = require(
        &program,
        &[
            "world",
            "diff",
            "no-v3/disclosure-policy-v1",
            "policy/no-v3-disclosure",
            "internal",
            "public",
        ],
        &[
            "world/diff-v1",
            "(changed true)",
            "conclusion/public-release",
        ],
        &["conclusion/source-api"],
    )?;
    Ok(vec![
        observation("product.world-why", &why),
        observation("product.world-project", &project),
        observation("product.world-diff", &diff),
    ])
}

pub(super) fn portable(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    match request.scope {
        "product/prove-plan" => prove_plan(request),
        "product/prove" => prove(request),
        "product/deliver" => deliver(request),
        "product/roadmap" => roadmap(request),
        _ => unreachable!("availability was checked before dispatch"),
    }
}

fn prove_plan(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = program(request)?;
    let stdout = run(&program, &["prove", "plan"])?;
    let record = prove_record(&stdout)?;
    require_prefix(record, "#(prove/plan-v1 ", "prove/plan-v1")?;
    require_text(record, "(denominator 29)", "denominator")?;
    require_text(record, "leaf/native-worker-adapter", "native-leaf")?;
    Ok(vec![observation("product.prove-plan", &stdout)])
}

fn prove(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = program(request)?;
    let cold_stdout = run(&program, &["prove", "run", "cold"])?;
    let cold = prove_record(&cold_stdout)?.to_owned();
    require_prefix(&cold, "#(prove/cold-v1 ", "prove/cold-v1")?;
    let view = view_of(&cold)?;
    let denominator = field(&cold, "denominator")?.to_owned();
    let executed = field(&cold, "executed")?.to_owned();
    let parts = chunks(&view);
    if parts.is_empty() {
        return Err(fail("view-absent", "empty"));
    }
    let status_args = with_prefix(&["prove", "status"], &parts);
    let status_stdout = run(&program, &status_args)?;
    let status = prove_record(&status_stdout)?.to_owned();
    require_prefix(&status, "#(prove/status-v1 ", "prove/status-v1")?;
    require_field(&status, "denominator", &denominator)?;
    require_field(&status, "executed", &executed)?;
    let frontier_args = with_prefix(&["prove", "frontier"], &parts);
    let frontier_stdout = run(&program, &frontier_args)?;
    let frontier = prove_record(&frontier_stdout)?.to_owned();
    require_prefix(&frontier, "#(prove/frontier-v1 ", "prove/frontier-v1")?;
    require_field(&frontier, "complete", "true")?;
    require_field(&frontier, "denominator", &denominator)?;
    require_field(&frontier, "executed", &executed)?;
    Ok(vec![
        observation("product.prove-cold", &cold_stdout),
        observation("product.prove-status", &status_stdout),
        observation("product.prove-frontier", &frontier_stdout),
    ])
}

fn deliver(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = program(request)?;
    let stdout = require(
        &program,
        &["world", "deliver", "model"],
        &[
            "#(world/deliver-v1 ",
            "(realization \"model\")",
            "(effects 0)",
            "(subprocesses 0)",
            "(resident true)",
            "(recovered true)",
        ],
        &["private-material:", "/home/"],
    )?;
    Ok(vec![observation("product.deliver", &stdout)])
}

fn roadmap(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = program(request)?;
    let stdout = run(
        &program,
        &[
            "roadmap",
            "status",
            "--conduct",
            "conduct/local",
            "--model-pick",
            "model/local",
            "--proof-catalog",
            "proof/local",
            "--runner-generation",
            "generation/local",
        ],
    )?;
    if stdout.trim_end() != ROADMAP_LINE {
        return Err(fail("marker-absent", "roadmap/local-result-v1"));
    }
    Ok(vec![observation("product.roadmap", &stdout)])
}

fn require(
    program: &Path,
    args: &[&str],
    markers: &[&str],
    absent: &[&str],
) -> Result<String, PackFailure> {
    let stdout = run(program, args)?;
    for marker in markers {
        require_text(&stdout, marker, marker)?;
    }
    for marker in absent {
        if stdout.contains(marker) {
            return Err(fail("forbidden-output", marker));
        }
    }
    Ok(stdout)
}

fn require_prefix(text: &str, prefix: &str, detail: &str) -> Result<(), PackFailure> {
    if text.starts_with(prefix) {
        Ok(())
    } else {
        Err(fail("marker-absent", detail))
    }
}

fn require_text(text: &str, marker: &str, detail: &str) -> Result<(), PackFailure> {
    if text.contains(marker) {
        Ok(())
    } else {
        Err(fail("marker-absent", detail))
    }
}

pub(super) fn program(request: &PackRequest<'_>) -> Result<PathBuf, PackFailure> {
    let mut body = None;
    for key in request.evidence.fact_keys() {
        if key == COMMAND_FACT {
            body = request.evidence.fact(key);
        } else {
            return Err(fail("unexpected-fact", key));
        }
    }
    let text = body.ok_or_else(|| fail("missing-source", COMMAND_FACT))?;
    if text.contains("completion/claim-") {
        return Err(fail("claim-relabelled", COMMAND_FACT));
    }
    if text.is_empty() || text.contains('\0') || text.contains('\n') || text == "true" {
        return Err(fail("bad-record", COMMAND_FACT));
    }
    let canonical =
        std::fs::canonicalize(text).map_err(|_| fail("missing-source", COMMAND_FACT))?;
    if !canonical.is_file() {
        return Err(fail("missing-source", COMMAND_FACT));
    }
    Ok(canonical)
}

pub(super) fn run(program: &Path, args: &[impl AsRef<str>]) -> Result<String, PackFailure> {
    let mut command = Command::new(program);
    if let Some(dir) = checkout_dir(program) {
        command.current_dir(dir);
    }
    for arg in args {
        let arg = arg.as_ref();
        if arg.contains('\0') {
            return Err(fail("process-unrun", "argv"));
        }
        command.arg(arg);
    }
    let output = command.output().map_err(|_| fail("process-unrun", "sim"))?;
    if output.stdout.len() > MAX_OUTPUT || output.stderr.len() > MAX_OUTPUT {
        return Err(fail("process-output", "sim"));
    }
    let status = output
        .status
        .code()
        .ok_or_else(|| fail("process-unrun", "signal"))?;
    if status != 0 {
        return Err(fail("process-failed", &status.to_string()));
    }
    if !output.stderr.is_empty() {
        return Err(fail("process-failed", "stderr"));
    }
    let stdout = std::str::from_utf8(&output.stdout).map_err(|_| fail("bad-record", "stdout"))?;
    if stdout.contains("completion/claim-") {
        return Err(fail("claim-relabelled", "stdout"));
    }
    if stdout.contains("PortableClosed") || stdout.contains("derive_completion") {
        return Err(fail("forbidden-output", "stdout"));
    }
    Ok(stdout.to_owned())
}

/// Checkout root of `program`: the nearest ancestor with a `.git` file or directory.
fn checkout_dir(program: &Path) -> Option<PathBuf> {
    let mut dir = program.parent()?;
    loop {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

fn prove_record(stdout: &str) -> Result<&str, PackFailure> {
    let mut lines = stdout.lines();
    let record = lines
        .next()
        .ok_or_else(|| fail("marker-absent", "record"))?;
    let receipt = lines
        .next()
        .ok_or_else(|| fail("marker-absent", "load-receipt"))?;
    if lines.next().is_some() {
        return Err(fail("marker-absent", "extra"));
    }
    if receipt != LOAD_RECEIPT {
        return Err(fail("marker-absent", "load-receipt"));
    }
    Ok(record)
}

fn view_of(record: &str) -> Result<String, PackFailure> {
    const KEY: &str = "(view \"";
    let Some(start) = record.find(KEY) else {
        return Err(fail("view-absent", "prove"));
    };
    let mut view = String::new();
    let mut chars = record[start + KEY.len()..].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                return Err(fail("view-absent", "escape"));
            };
            view.push(next);
            continue;
        }
        if ch == '"' {
            return Ok(view);
        }
        view.push(ch);
    }
    Err(fail("view-absent", "unclosed"))
}

pub(super) fn field<'a>(record: &'a str, name: &str) -> Result<&'a str, PackFailure> {
    let key = format!("({name} ");
    let mut rest = record;
    let mut quoted = false;
    let mut escaped = false;
    while !rest.is_empty() {
        if quoted {
            let ch = rest.chars().next().expect("quoted character");
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if rest.starts_with('"') {
            quoted = true;
            rest = &rest[1..];
            continue;
        }
        if rest.starts_with(&key) {
            let value = &rest[key.len()..];
            let end = value
                .find([' ', ')'])
                .ok_or_else(|| fail("marker-absent", name))?;
            return Ok(&value[..end]);
        }
        let ch = rest.chars().next().expect("record character");
        rest = &rest[ch.len_utf8()..];
    }
    Err(fail("marker-absent", name))
}

pub(super) fn require_field(record: &str, name: &str, expected: &str) -> Result<(), PackFailure> {
    if field(record, name)? != expected {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

fn chunks(view: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = view;
    while !rest.is_empty() {
        let mut end = rest.len().min(CHUNK);
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        let (head, tail) = rest.split_at(end);
        parts.push(head);
        rest = tail;
    }
    parts
}

fn with_prefix(prefix: &[&str], parts: &[&str]) -> Vec<String> {
    prefix
        .iter()
        .copied()
        .chain(parts.iter().copied())
        .map(str::to_owned)
        .collect()
}

pub(super) fn observation(key: &str, stdout: &str) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: stdout.to_owned(),
    }
}

pub(super) fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}
