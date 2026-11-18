// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Runs `world deliver model` and reads the seven laws from its record.
//!
//! The stdout line is one closed `world/deliver-v1` root. Nested action rows
//! and quoted text are not the plan fields. An already-true action is a
//! completed checkpoint that was not omitted.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::COMMAND_FACT;
use crate::{CheckObservation, PackFailure, PackRequest};

const MAX_OUTPUT: usize = 16 * 1024 * 1024;

struct Row<'a> {
    action: bool,
    cause: &'a str,
    class: &'a str,
    acknowledgement: &'a str,
    dispatches: u64,
    effects: u64,
}

pub(super) fn record(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = program(request)?;
    let stdout = run(&program, &["world", "deliver", "model"])?;
    let line = one_line(&stdout)?;
    if line.contains("private-material:") || line.contains("/home/") {
        return Err(fail("forbidden-output", "private"));
    }
    require_plain(line, "realization", "model")?;
    require_count(line, "effects", 0)?;
    require_count(line, "subprocesses", 0)?;
    require_plain(line, "resident", "true")?;
    require_plain(line, "recovered", "true")?;
    laws(&rows(line)?)?;
    Ok(vec![observation("delivery.model", &stdout)])
}

fn laws(rows: &[Row<'_>]) -> Result<(), PackFailure> {
    if !rows.iter().any(|row| {
        !row.action
            && row.cause == "already-true"
            && row.class == "already-true"
            && row.acknowledgement == "omitted"
            && row.dispatches == 0
            && row.effects == 0
    }) {
        return Err(mismatch(0));
    }
    if !rows.iter().any(|row| {
        row.action
            && row.cause == "stale-observation"
            && row.class == "uncertain"
            && row.acknowledgement == "unsent"
            && row.dispatches == 0
            && row.effects == 0
    }) {
        return Err(mismatch(1));
    }
    if !rows
        .iter()
        .any(|row| row.action && row.acknowledgement == "missing" && row.effects == 0)
    {
        return Err(mismatch(2));
    }
    if !rows.iter().any(|row| {
        row.action
            && (row.cause == "divergent" || row.cause == "wrong-tag")
            && row.class == "diverged"
            && row.acknowledgement == "missing"
            && row.effects == 0
    }) {
        return Err(mismatch(3));
    }
    if !rows.iter().any(|row| {
        row.action
            && row.cause == "registry-lag"
            && row.class == "uncertain"
            && row.acknowledgement == "unsent"
            && row.dispatches == 0
            && row.effects == 0
    }) {
        return Err(mismatch(4));
    }
    if !rows.iter().any(|row| {
        row.action
            && row.cause == "uncertain-reading"
            && row.class == "uncertain"
            && row.acknowledgement == "unsent"
            && row.dispatches == 0
            && row.effects == 0
    }) {
        return Err(mismatch(5));
    }
    if rows
        .iter()
        .any(|row| row.action && row.cause == "already-true")
    {
        return Err(mismatch(6));
    }
    Ok(())
}

fn mismatch(index: usize) -> PackFailure {
    fail("record-mismatch", super::SCENARIOS[index])
}

fn rows(record: &str) -> Result<Vec<Row<'_>>, PackFailure> {
    let mut found = Vec::new();
    for node in nodes(record, "world/deliver-omission-v1")? {
        found.push(row(node, false)?);
    }
    for node in nodes(record, "world/deliver-action-v1")? {
        found.push(row(node, true)?);
    }
    Ok(found)
}

fn row(node: &str, action: bool) -> Result<Row<'_>, PackFailure> {
    Ok(Row {
        action,
        cause: plain(node, "cause")?,
        class: plain(node, "class")?,
        acknowledgement: plain(node, "acknowledgement")?,
        dispatches: count(node, "dispatches")?,
        effects: count(node, "effects")?,
    })
}

fn nodes<'a>(record: &'a str, tag: &str) -> Result<Vec<&'a str>, PackFailure> {
    let marker = format!("#({tag} ");
    let mut rest = record;
    let mut quoted = false;
    let mut escaped = false;
    let mut found = Vec::new();
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
        if rest.starts_with(&marker) {
            let whole = balanced(rest)?;
            found.push(whole);
            rest = &rest[whole.len()..];
            continue;
        }
        let ch = rest.chars().next().expect("record character");
        rest = &rest[ch.len_utf8()..];
    }
    Ok(found)
}

fn balanced(text: &str) -> Result<&str, PackFailure> {
    let mut depth = 0i32;
    let mut quoted = false;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '"' => quoted = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&text[..=index]);
                }
            }
            _ => {}
        }
    }
    Err(fail("bad-record", "unclosed"))
}

fn require_plain(record: &str, name: &str, expected: &str) -> Result<(), PackFailure> {
    if plain(record, name)? != expected {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

fn require_count(record: &str, name: &str, expected: u64) -> Result<(), PackFailure> {
    if count(record, name)? != expected {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

fn plain<'a>(record: &'a str, name: &str) -> Result<&'a str, PackFailure> {
    let raw = at_depth_two(record, name)?;
    match raw.strip_prefix('"') {
        Some(rest) => {
            let inner = rest
                .strip_suffix('"')
                .ok_or_else(|| fail("bad-record", name))?;
            if inner.contains(['"', '\\']) {
                return Err(fail("bad-record", name));
            }
            Ok(inner)
        }
        None => Ok(raw),
    }
}

fn count(record: &str, name: &str) -> Result<u64, PackFailure> {
    plain(record, name)?
        .parse()
        .map_err(|_| fail("bad-record", name))
}

/// Plan fields sit directly inside the root. Fields of a nested row do not.
fn at_depth_two<'a>(record: &'a str, name: &str) -> Result<&'a str, PackFailure> {
    let key = format!("({name} ");
    let mut depth = 0i32;
    let mut quoted = false;
    let mut escaped = false;
    let mut rest = record;
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
        if rest.starts_with('(') {
            depth += 1;
            if depth == 2 && rest.starts_with(&key) {
                let value = &rest[key.len()..];
                let end = value
                    .find([' ', ')'])
                    .ok_or_else(|| fail("marker-absent", name))?;
                return Ok(&value[..end]);
            }
            rest = &rest[1..];
            continue;
        }
        if rest.starts_with(')') {
            depth -= 1;
            rest = &rest[1..];
            continue;
        }
        let ch = rest.chars().next().expect("record character");
        rest = &rest[ch.len_utf8()..];
    }
    Err(fail("marker-absent", name))
}

/// One stdout line whose bytes are exactly one closed `world/deliver-v1` root.
fn one_line(stdout: &str) -> Result<&str, PackFailure> {
    let Some(line) = stdout.strip_suffix('\n') else {
        return Err(fail("marker-absent", "record"));
    };
    if line.contains('\n') {
        return Err(fail("marker-absent", "extra"));
    }
    if !line.starts_with("#(world/deliver-v1 ") && line != "#(world/deliver-v1)" {
        return Err(fail("marker-absent", "world/deliver-v1"));
    }
    let root = balanced(line)?;
    if root.len() != line.len() {
        return Err(fail("marker-absent", "extra"));
    }
    Ok(root)
}

fn program(request: &PackRequest<'_>) -> Result<PathBuf, PackFailure> {
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

fn run(program: &Path, args: &[&str]) -> Result<String, PackFailure> {
    let mut command = Command::new(program);
    if let Some(dir) = checkout_dir(program) {
        command.current_dir(dir);
    }
    for arg in args {
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

fn checkout_dir(program: &Path) -> Option<PathBuf> {
    let mut dir = program.parent()?;
    loop {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
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
