// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Planning observation for `release/nv12-08`.
//!
//! The scope runs `prove plan` twice and `world why` on the program named by
//! `product.command`. The two plans must match. The plan record keeps a zero
//! effect count and names each native leaf as an exact member of its excluded
//! set. The explanation names the disclosure owner. The recorded values are
//! that program output.

use crate::{CheckObservation, PackFailure, PackRequest};

const NATIVE_LEAVES: [&str; 2] = ["leaf/native-worker-adapter", "leaf/native-assay"];

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = crate::packs::product::program(request)?;
    let first = crate::packs::product::run(&program, &["prove", "plan"])?;
    let second = crate::packs::product::run(&program, &["prove", "plan"])?;
    if first != second {
        return Err(fail("plan-unstable", "prove plan"));
    }
    let record = crate::packs::product::prove_record(&first)?;
    if !record.starts_with("#(prove/plan-v1 ") {
        return Err(fail("marker-absent", "prove/plan-v1"));
    }
    crate::packs::product::require_field(record, "denominator", "29")?;
    crate::packs::product::require_field(record, "effects", "0")?;
    crate::packs::product::require_field(record, "subprocesses", "0")?;
    crate::packs::product::require_field(record, "owner-commands", "0")?;
    crate::packs::product::require_field(record, "resident", "true")?;
    require_exact_leaves(record)?;
    let why = crate::packs::product::run(
        &program,
        &[
            "world",
            "why",
            "conclusion/public-release",
            "policy/no-v3-disclosure",
        ],
    )?;
    if !why.contains("world/explanation-v1") {
        return Err(fail("marker-absent", "world/explanation-v1"));
    }
    if !why.contains("owner/sim-private/disclosure") {
        return Err(fail("marker-absent", "owner/sim-private/disclosure"));
    }
    Ok(vec![
        observation("release.plan", &first),
        observation("release.why", &why),
    ])
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

fn require_exact_leaves(record: &str) -> Result<(), PackFailure> {
    let names = excluded_names(record)?;
    for leaf in NATIVE_LEAVES {
        if !names.iter().any(|name| name == leaf) {
            return Err(fail("marker-absent", leaf));
        }
    }
    Ok(())
}

fn excluded_names(record: &str) -> Result<Vec<String>, PackFailure> {
    quoted_names(field_body(record, "excluded")?)
}

fn field_body<'a>(record: &'a str, name: &str) -> Result<&'a str, PackFailure> {
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
            let end = field_end(value).ok_or_else(|| fail("marker-absent", name))?;
            return Ok(&value[..end]);
        }
        let ch = rest.chars().next().expect("record character");
        rest = &rest[ch.len_utf8()..];
    }
    Err(fail("marker-absent", name))
}

fn field_end(value: &str) -> Option<usize> {
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0i32;
    for (index, ch) in value.char_indices() {
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
            ')' if depth == 0 => return Some(index),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn quoted_names(body: &str) -> Result<Vec<String>, PackFailure> {
    let mut names = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        let mut name = String::new();
        let mut consumed = 0;
        let mut closed = false;
        let mut chars = after.char_indices();
        while let Some((offset, ch)) = chars.next() {
            if ch == '\\' {
                let Some((next_offset, next)) = chars.next() else {
                    return Err(fail("marker-absent", "excluded"));
                };
                name.push(next);
                consumed = next_offset + next.len_utf8();
                continue;
            }
            if ch == '"' {
                consumed = offset + ch.len_utf8();
                closed = true;
                break;
            }
            name.push(ch);
        }
        if !closed {
            return Err(fail("marker-absent", "excluded"));
        }
        names.push(name);
        rest = &after[consumed..];
    }
    Ok(names)
}
