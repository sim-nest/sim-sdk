// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Stable-output and specimen checks for the joined product load.

use std::path::{Path, PathBuf};

use super::command;
use crate::PackFailure;

const FRAME: &str = "without placing its behavior in the frame.";
const BOOT: &str = "feature/sim-run/bootloader";

const SPECIMENS: &[(&str, &str)] = &[
    (
        "route/run-world-command",
        "recipe/sim-runtime/crates/sim-lib-world/01-projection/world-explain",
    ),
    (
        "route/run-prove-command",
        "spec-test/sim-run/crates/sim-run/tests/prove_gate",
    ),
    (
        "route/run-deliver-command",
        "spec-test/sim-run/crates/sim-run/tests/deliver_gate",
    ),
    (
        "route/run-roadmap-command",
        "spec-test/sim-run/crates/sim-run/tests/roadmap",
    ),
];

pub(super) fn stable(program: &Path, args: &[&str]) -> Result<String, PackFailure> {
    let first = command::run(program, args)?;
    let second = command::run(program, args)?;
    if first == second {
        Ok(first)
    } else {
        Err(command::fail(
            "unstable",
            args.first().copied().unwrap_or("stdout"),
        ))
    }
}

pub(super) fn require_command(key: &str, stdout: &str) -> Result<(), PackFailure> {
    match key {
        "product.load.world" => require_markers(
            stdout,
            &["world/explanation-v1", "owner/sim-private/disclosure"],
        ),
        "product.load.prove" => require_prove(stdout),
        "product.load.deliver" => {
            require_markers(
                stdout,
                &[
                    "#(world/deliver-v1 ",
                    "(realization \"model\")",
                    "(effects 0)",
                    "(subprocesses 0)",
                    "(resident true)",
                    "(recovered true)",
                ],
            )?;
            if stdout.contains("private-material:") || stdout.contains("/home/") {
                return Err(command::fail("forbidden-output", "deliver"));
            }
            Ok(())
        }
        "product.load.roadmap" => {
            if stdout.trim_end() == command::ROADMAP_LINE {
                Ok(())
            } else {
                Err(command::fail("marker-absent", "roadmap/local-result-v1"))
            }
        }
        _ => unreachable!("command key is one of the four loads"),
    }
}

pub(super) fn require_route(stdout: &str, id: &str, specimen: &str) -> Result<(), PackFailure> {
    let Some(block) = route_block(stdout, id) else {
        return Err(command::fail("marker-absent", id));
    };
    if !has_step(&block, "feature", BOOT) {
        return Err(command::fail("marker-absent", BOOT));
    }
    if !block.contains(FRAME) {
        return Err(command::fail("marker-absent", "frame"));
    }
    let found = specimen_token(&block).ok_or_else(|| command::fail("specimen-absent", id))?;
    specimen_paths(found)?;
    if found == specimen {
        Ok(())
    } else {
        Err(command::fail("specimen-absent", found))
    }
}

pub(super) fn specimen_for(id: &str) -> Result<&'static str, PackFailure> {
    SPECIMENS
        .iter()
        .find(|(route, _)| *route == id)
        .map(|(_, specimen)| *specimen)
        .ok_or_else(|| command::fail("specimen-absent", id))
}

pub(super) fn read_specimen(root: &Path, token: &str) -> Result<(), PackFailure> {
    let (file, brochure) = specimen_paths(token)?;
    let file = root.join(&file);
    let brochure = root.join(&brochure);
    if !file.is_file() {
        return Err(command::fail("specimen-absent", token));
    }
    let file = contained(root, &file, "specimen-absent")?;
    let bytes = std::fs::read(&file).map_err(|_| command::fail("specimen-absent", token))?;
    if bytes.is_empty() {
        return Err(command::fail("specimen-absent", token));
    }
    if !brochure.is_file() {
        return Err(command::fail("brochure-absent", token));
    }
    let brochure = contained(root, &brochure, "brochure-absent")?;
    let text =
        std::fs::read_to_string(&brochure).map_err(|_| command::fail("brochure-absent", token))?;
    if text.contains("In one line:") {
        Ok(())
    } else {
        Err(command::fail("brochure-absent", "In one line:"))
    }
}

fn require_prove(stdout: &str) -> Result<(), PackFailure> {
    let mut lines = stdout.lines();
    let record = lines
        .next()
        .ok_or_else(|| command::fail("marker-absent", "record"))?;
    let receipt = lines
        .next()
        .ok_or_else(|| command::fail("marker-absent", "load-receipt"))?;
    if lines.next().is_some()
        || !record.starts_with("#(prove/plan-v1 ")
        || !record.contains("(denominator 29)")
        || !record.contains("leaf/native-worker-adapter")
        || receipt != command::LOAD_RECEIPT
    {
        return Err(command::fail("marker-absent", "prove/plan-v1"));
    }
    Ok(())
}

fn require_markers(stdout: &str, markers: &[&str]) -> Result<(), PackFailure> {
    for marker in markers {
        if !stdout.contains(marker) {
            return Err(command::fail("marker-absent", marker));
        }
    }
    Ok(())
}

fn route_block(stdout: &str, id: &str) -> Option<String> {
    let prefix = format!("{id}\t");
    let mut lines = stdout.lines();
    while let Some(line) = lines.next() {
        if !line.starts_with(&prefix) {
            continue;
        }
        let mut block = String::from(line);
        for line in lines.by_ref() {
            if line.is_empty() || line.starts_with(' ') {
                block.push('\n');
                block.push_str(line);
                continue;
            }
            break;
        }
        return Some(block);
    }
    None
}

fn has_step(block: &str, kind: &str, id: &str) -> bool {
    block.lines().any(|line| step_id(line, kind) == Some(id))
}

fn specimen_token(block: &str) -> Option<&str> {
    block.lines().find_map(|line| step_id(line, "specimen"))
}

fn step_id<'a>(line: &'a str, kind: &str) -> Option<&'a str> {
    let rest = line.strip_prefix("  ")?;
    let (found, rest) = rest.split_once('\t')?;
    if found != kind {
        return None;
    }
    let id = rest.split('\t').next().unwrap_or(rest);
    (!id.is_empty()).then_some(id)
}

fn specimen_paths(token: &str) -> Result<(PathBuf, PathBuf), PackFailure> {
    let parts: Vec<&str> = token.split('/').collect();
    if parts.iter().any(|part| !segment(part)) {
        return Err(command::fail("specimen-absent", token));
    }
    let (repo, crate_name, file) = match parts.as_slice() {
        ["recipe", repo, "crates", crate_name, rest @ ..] if !rest.is_empty() => {
            let mut file = PathBuf::from("recipes");
            file.extend(rest);
            file.push("recipe.toml");
            (*repo, *crate_name, file)
        }
        ["spec-test", repo, "crates", crate_name, "tests", name] => (
            *repo,
            *crate_name,
            PathBuf::from(format!("tests/{name}.rs")),
        ),
        _ => return Err(command::fail("specimen-absent", token)),
    };
    let crate_dir = PathBuf::from(repo).join("crates").join(crate_name);
    Ok((crate_dir.join(file), crate_dir.join("BROCHURE.md")))
}

fn segment(part: &str) -> bool {
    !part.is_empty()
        && part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn contained(root: &Path, path: &Path, code: &'static str) -> Result<PathBuf, PackFailure> {
    let canonical = std::fs::canonicalize(path).map_err(|_| command::fail(code, "path"))?;
    if canonical.starts_with(root) {
        Ok(canonical)
    } else {
        Err(command::fail(code, "path"))
    }
}
