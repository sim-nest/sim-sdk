// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Dependency-ordered release closure for `release/nv12-27`.
//!
//! The checker reads one injected closure. It does not contact a registry,
//! create a tag, or accept a caller boolean as publication.

use sha2::{Digest, Sha256};

use crate::{CheckObservation, PackFailure, PackRequest};

const MAX_CRATES: usize = 64;
const MAX_DEPS: usize = 32;
const MAX_BYTES: usize = 65_536;

struct Row {
    name: String,
    version: String,
    checksum: String,
    tag: String,
    pin: String,
    mirror: String,
    ci: String,
    boot: String,
    deps: Vec<String>,
}

pub(super) fn check_nv12_27(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    if request
        .evidence
        .fact("release.publication-confirmed")
        .is_some()
    {
        return Err(fail(
            "local-publication-claim",
            "release.publication-confirmed",
        ));
    }
    let mut closure = None;
    let mut dry_run = None;
    for key in request.evidence.fact_keys() {
        match key {
            "release.closure" => closure = request.evidence.fact(key),
            "release.dry-run" => dry_run = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    let Some(closure) = closure else {
        let code = if dry_run.is_some() {
            "publication-unreconciled"
        } else {
            "missing-closure"
        };
        return Err(fail(code, "release.closure"));
    };
    if closure.len() > MAX_BYTES {
        return Err(fail("bound", "release.closure"));
    }
    let rows = parse_closure(closure)?;
    if let Some(dry_run) = dry_run {
        if dry_run.len() > MAX_BYTES {
            return Err(fail("bound", "release.dry-run"));
        }
        match_dry_run(&rows, dry_run)?;
    }
    let order = rows
        .iter()
        .map(|row| row.name.as_str())
        .collect::<Vec<_>>()
        .join(",");
    Ok(vec![
        observation("release.closure-digest", digest_rows(&rows)),
        observation("release.crate-count", rows.len().to_string()),
        observation("release.order", order),
        observation("release.outward", "consistent"),
    ])
}

fn parse_closure(text: &str) -> Result<Vec<Row>, PackFailure> {
    if text.contains('\r') {
        return Err(fail("bad-record", "release.closure"));
    }
    let body = text.strip_suffix('\n').unwrap_or(text);
    if body.is_empty() {
        return Err(fail("empty-closure", "release.closure"));
    }
    let mut rows = Vec::new();
    for block in body.split("\n\n") {
        if block.is_empty() {
            return Err(fail("bad-record", "release.closure"));
        }
        rows.push(parse_block(block)?);
        if rows.len() > MAX_CRATES {
            return Err(fail("bound", "release.closure"));
        }
    }
    let mut seen = std::collections::BTreeMap::<&str, usize>::new();
    for (index, row) in rows.iter().enumerate() {
        if seen.insert(row.name.as_str(), index).is_some() {
            return Err(fail("duplicate-crate", &row.name));
        }
    }
    for (index, row) in rows.iter().enumerate() {
        for dep in &row.deps {
            match seen.get(dep.as_str()) {
                Some(position) if *position < index => {}
                Some(_) => return Err(fail("order", dep)),
                None => return Err(fail("missing-dependency", dep)),
            }
        }
    }
    Ok(rows)
}

fn parse_block(block: &str) -> Result<Row, PackFailure> {
    let lines = block.split('\n').collect::<Vec<_>>();
    if lines.len() != 9 {
        return Err(fail("bad-record", "release.closure"));
    }
    let name = owned(take(lines[0], "crate")?, is_name, "bad-name")?;
    let version = owned(take(lines[1], "version")?, is_version, "bad-version")?;
    let checksum = owned(
        take(lines[2], "checksum")?,
        |value| is_hex(value, 64),
        "bad-checksum",
    )?;
    let tag = take(lines[3], "tag")?;
    let expected_tag = format!("{name}-v{version}");
    if tag != expected_tag {
        return Err(fail("tag-mismatch", tag));
    }
    let pin = owned(take(lines[4], "pin")?, |value| is_hex(value, 40), "bad-pin")?;
    let mirror = owned(
        take(lines[5], "mirror")?,
        |value| is_hex(value, 40),
        "bad-mirror",
    )?;
    if mirror != pin {
        return Err(fail("mirror-mismatch", &mirror));
    }
    let ci = owned(take(lines[6], "ci")?, |value| is_hex(value, 64), "bad-ci")?;
    let boot = owned(
        take(lines[7], "boot")?,
        |value| is_hex(value, 64),
        "bad-boot",
    )?;
    let deps = parse_deps(lines[8])?;
    Ok(Row {
        name,
        version,
        checksum,
        tag: expected_tag,
        pin,
        mirror,
        ci,
        boot,
        deps,
    })
}

fn parse_deps(line: &str) -> Result<Vec<String>, PackFailure> {
    if line == "deps" {
        return Ok(Vec::new());
    }
    let Some(rest) = line.strip_prefix("deps ") else {
        return Err(fail("bad-record", "deps"));
    };
    if rest.is_empty() {
        return Err(fail("bad-record", "deps"));
    }
    let mut deps = Vec::new();
    for name in rest.split(' ') {
        if !is_name(name) {
            return Err(fail("bad-name", name));
        }
        if deps.iter().any(|existing| existing == name) {
            return Err(fail("duplicate-dependency", name));
        }
        deps.push(name.to_owned());
        if deps.len() > MAX_DEPS {
            return Err(fail("bound", "deps"));
        }
    }
    Ok(deps)
}

fn match_dry_run(rows: &[Row], text: &str) -> Result<(), PackFailure> {
    if text.contains('\r') {
        return Err(fail("bad-record", "release.dry-run"));
    }
    let body = text.strip_suffix('\n').unwrap_or(text);
    if body.is_empty() {
        return Err(fail("dry-run-mismatch", "release.dry-run"));
    }
    let mut seen = std::collections::BTreeMap::<String, String>::new();
    for line in body.split('\n') {
        let mut parts = line.split(' ');
        let Some(name) = parts.next() else {
            return Err(fail("bad-record", "release.dry-run"));
        };
        let Some(version) = parts.next() else {
            return Err(fail("bad-record", "release.dry-run"));
        };
        if parts.next().is_some() || !is_name(name) || !is_version(version) {
            return Err(fail("bad-record", "release.dry-run"));
        }
        if seen.insert(name.to_owned(), version.to_owned()).is_some() {
            return Err(fail("duplicate-crate", name));
        }
    }
    if seen.len() != rows.len() {
        return Err(fail("dry-run-mismatch", "release.dry-run"));
    }
    for row in rows {
        match seen.get(&row.name) {
            Some(version) if version == &row.version => {}
            _ => return Err(fail("dry-run-mismatch", &row.name)),
        }
    }
    Ok(())
}

fn digest_rows(rows: &[Row]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((rows.len() as u64).to_be_bytes());
    for row in rows {
        add(&mut hasher, row.name.as_bytes());
        add(&mut hasher, row.version.as_bytes());
        add(&mut hasher, row.checksum.as_bytes());
        add(&mut hasher, row.tag.as_bytes());
        add(&mut hasher, row.pin.as_bytes());
        add(&mut hasher, row.mirror.as_bytes());
        add(&mut hasher, row.ci.as_bytes());
        add(&mut hasher, row.boot.as_bytes());
        hasher.update((row.deps.len() as u64).to_be_bytes());
        for dep in &row.deps {
            add(&mut hasher, dep.as_bytes());
        }
    }
    hex(&hasher.finalize())
}

fn add(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn take<'a>(line: &'a str, key: &str) -> Result<&'a str, PackFailure> {
    let Some(rest) = line.strip_prefix(key) else {
        return Err(fail("bad-record", key));
    };
    let Some(rest) = rest.strip_prefix(' ') else {
        return Err(fail("bad-record", key));
    };
    if rest.is_empty()
        || rest
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(fail("bad-record", key));
    }
    Ok(rest)
}

fn owned(
    value: &str,
    accept: impl Fn(&str) -> bool,
    code: &'static str,
) -> Result<String, PackFailure> {
    if accept(value) {
        Ok(value.to_owned())
    } else {
        Err(fail(code, value))
    }
}

fn is_name(value: &str) -> bool {
    let mut part = false;
    for byte in value.bytes() {
        if byte == b'-' {
            if !part {
                return false;
            }
            part = false;
            continue;
        }
        if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() {
            return false;
        }
        part = true;
    }
    part
}

fn is_version(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(major) = parts.next() else {
        return false;
    };
    let Some(minor) = parts.next() else {
        return false;
    };
    let Some(patch) = parts.next() else {
        return false;
    };
    parts.next().is_none() && is_component(major) && is_component(minor) && is_component(patch)
}

fn is_component(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value.len() == 1 || !value.starts_with('0'))
}

fn is_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn observation(key: &str, value: impl Into<String>) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.into(),
    }
}

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}
