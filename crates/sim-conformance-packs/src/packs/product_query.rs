// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Cached why measurement recorded from the named program.
//!
//! The program runs `world measure why` once. That command times the resident
//! query in process. Caller-supplied latency facts are not a measurement.

use super::command;
use crate::{CheckObservation, PackFailure, PackRequest};

const SAMPLES: usize = 200;
const CEILING_US: u64 = 200_000;

pub(super) fn measure(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = command::program(request)?;
    let stdout = command::run(&program, &["world", "measure", "why"])?;
    let record = one_line(&stdout)?;
    if !record.starts_with("#(world/query-latency-v1 ") {
        return Err(command::fail("marker-absent", "world/query-latency-v1"));
    }
    if unquote(command::field(record, "command")?) != "why" {
        return Err(command::fail("record-mismatch", "command"));
    }
    command::require_field(record, "records", "100000")?;
    command::require_field(record, "units", "10000")?;
    command::require_field(record, "warmups", "20")?;
    command::require_field(record, "samples", "200")?;
    command::require_field(record, "effects", "0")?;
    command::require_field(record, "subprocesses", "0")?;
    command::require_field(record, "resident", "true")?;
    let samples = raw_samples(record)?;
    if samples.len() != SAMPLES {
        return Err(command::fail("record-mismatch", "samples"));
    }
    let p95_us = samples[nearest_rank(samples.len())];
    let p95_ms = p95_us / 1_000;
    command::require_field(record, "p95-ms", &p95_ms.to_string())?;
    if p95_us > CEILING_US {
        return Err(command::fail("record-mismatch", "ceiling"));
    }
    Ok(vec![command::observation("product.query-latency", &stdout)])
}

fn one_line(stdout: &str) -> Result<&str, PackFailure> {
    let record = stdout
        .strip_suffix('\n')
        .ok_or_else(|| command::fail("marker-absent", "record"))?;
    if record.contains('\n') {
        return Err(command::fail("marker-absent", "extra"));
    }
    Ok(record)
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(value)
}

fn raw_samples(record: &str) -> Result<Vec<u64>, PackFailure> {
    let key = "(raw-us [";
    let mut rest = record;
    let mut quoted = false;
    let mut escaped = false;
    let mut body = None;
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
        if let Some(stripped) = rest.strip_prefix(key) {
            body = Some(stripped);
            break;
        }
        let ch = rest.chars().next().expect("record character");
        rest = &rest[ch.len_utf8()..];
    }
    let Some(body) = body else {
        return Err(command::fail("marker-absent", "raw-us"));
    };
    let end = body
        .find("])")
        .ok_or_else(|| command::fail("marker-absent", "raw-us"))?;
    let text = &body[..end];
    if text.contains('"') {
        return Err(command::fail("bad-record", "raw-us"));
    }
    let mut samples = Vec::new();
    for token in text.split_whitespace() {
        let value = token
            .parse::<u64>()
            .map_err(|_| command::fail("bad-record", "raw-us"))?;
        samples.push(value);
    }
    let mut ordered = samples.clone();
    ordered.sort_unstable();
    Ok(ordered)
}

fn nearest_rank(samples: usize) -> usize {
    (samples.saturating_mul(95).saturating_add(99) / 100).saturating_sub(1)
}
