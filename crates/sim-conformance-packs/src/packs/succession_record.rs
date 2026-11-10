// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Canonical succession record.
//!
//! The checker reads one injected record. It does not load a package,
//! open a checkout, or accept a caller boolean as portable closure.

use std::collections::BTreeMap;

use crate::PackFailure;

const FIXTURES: &[&str] = &[
    "lantern",
    "lantern-alias",
    "lantern-revised",
    "lantern-stub",
    "model",
    "revised",
    "envelope",
    "native",
    "real",
    "owning",
    "changed",
    "missing",
    "corrupt",
    "stub",
    "absent",
    "wrong-target",
    "nested",
    "alias",
    "git-missing",
    "version-mismatch",
    "precise",
    "precise-toolchain",
    "precise-feature",
    "precise-generated",
    "precise-edge",
    "precise-closure",
    "precise-role",
];

const KEYS: &[&str] = &[
    "change",
    "project",
    "origin",
    "warm",
    "fixture-change",
    "observed",
    "proved",
    "seal",
    "converge-first",
    "converge-second",
    "land",
    "deliver-first",
    "deliver-second",
    "recovery",
    "recovery-reads",
    "holds",
    "linux",
    "closed",
    "preimage",
    "postimage",
];

pub(crate) struct Record {
    pub change: String,
    pub project: String,
    pub holds: u64,
    pub converge_second: u64,
    pub deliver_second: u64,
    pub closed: String,
    pub linux: String,
}

pub(crate) fn derive(text: &str) -> Result<Record, PackFailure> {
    if text.len() > 8_192 || text.contains('\r') {
        return Err(fail("bad-record", "succession.record"));
    }
    let body = text.strip_suffix('\n').unwrap_or(text).replace('\n', ";");
    let mut fields = BTreeMap::<String, String>::new();
    for part in body.split(';') {
        let part = part.trim();
        if part.is_empty() {
            return Err(fail("bad-record", "succession.record"));
        }
        let Some((key, value)) = part.split_once(' ') else {
            return Err(fail("bad-record", "succession.record"));
        };
        if value.contains(' ') || fields.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(fail("bad-record", "succession.record"));
        }
    }
    if fields.len() != KEYS.len() || KEYS.iter().any(|key| !fields.contains_key(*key)) {
        return Err(fail("bad-record", "succession.record"));
    }
    let change = fields["change"].clone();
    let project = fields["project"].clone();
    if FIXTURES.contains(&change.as_str()) || fields["fixture-change"] != "false" {
        return Err(fail("fixture-change", "change"));
    }
    if warm_change(&change) {
        return Err(fail("warm-state", "change"));
    }
    if !change_token(&change) {
        return Err(fail("bad-record", "change"));
    }
    if project != "lantern" {
        return Err(fail("foreign-project-absent", "project"));
    }
    if fields["origin"] != "registry" {
        return Err(fail("path-origin", "origin"));
    }
    if fields["warm"] != "false" {
        return Err(fail("warm-state", "warm"));
    }
    if fields["linux"] != "absent" {
        return Err(fail("linux-claim", "linux"));
    }
    if fields["recovery"] != "match" {
        return Err(fail("recovery-mismatch", "recovery"));
    }
    if fields["closed"] != "portable" {
        return Err(fail("not-closed", "closed"));
    }
    let observed = number(&fields["observed"], "observed")?;
    let proved = number(&fields["proved"], "proved")?;
    let seal = number(&fields["seal"], "seal")?;
    let converge_first = number(&fields["converge-first"], "converge-first")?;
    let converge_second = number(&fields["converge-second"], "converge-second")?;
    let land = number(&fields["land"], "land")?;
    let deliver_first = number(&fields["deliver-first"], "deliver-first")?;
    let deliver_second = number(&fields["deliver-second"], "deliver-second")?;
    let recovery_reads = number(&fields["recovery-reads"], "recovery-reads")?;
    let holds = number(&fields["holds"], "holds")?;
    if observed == 0 || proved != observed || seal == 0 || recovery_reads == 0 {
        return Err(fail("unproved", "proved"));
    }
    if converge_first == 0 || land == 0 || deliver_first == 0 {
        return Err(fail("no-work", "deliver-first"));
    }
    if converge_second != 0 || deliver_second != 0 {
        return Err(fail("second-work", "deliver-second"));
    }
    if holds != 0 {
        return Err(fail("hold", "holds"));
    }
    if !hex64(&fields["preimage"])
        || !hex64(&fields["postimage"])
        || fields["preimage"] == fields["postimage"]
    {
        return Err(fail("seal", "postimage"));
    }
    Ok(Record {
        change,
        project,
        holds,
        converge_second,
        deliver_second,
        closed: fields["closed"].clone(),
        linux: fields["linux"].clone(),
    })
}

fn warm_change(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("file://")
        || lower.contains(".git")
        || lower.contains("transcript")
        || lower.contains("checkout")
        || value.contains('\\')
        || value.contains("..")
        || value.contains("target/")
        || value.contains("/target")
        || value.contains("cache/")
        || value.contains("/cache")
        || value.starts_with('/')
        || value.starts_with('~')
}

fn change_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && !value.starts_with('.')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' || ch == '_')
}

fn number(value: &str, key: &str) -> Result<u64, PackFailure> {
    value.parse::<u64>().map_err(|_| fail("bad-record", key))
}

fn hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.to_owned(),
    }
}
