// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Runs `world qualify` and reads the composition laws from its records.
//!
//! A caller fact is not a composition. Nested text is not a detail field.

use crate::{CheckObservation, PackFailure, PackRequest};

const PRIVATE: &str = "private-material:s3cret-token";
const COMPOSITION: &str = "pages 1 revision 8eed83fd708e5c7a0944c8627663af1f8bc105a9c5a6f7d5659122e4661be28b transitions 1 stale-effects 0 effects 1 hot-delta 0 samples 20 providers model/direction,model/landing,model/snapshot stale-edges 1 bound 1 proof 0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e linux 0";
const BEGIN: &str = "distinct begin-required landed 1 stale 1 proof 0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e";

struct Law {
    key: &'static str,
    case: &'static str,
    detail: &'static str,
    invokes: u64,
}

const LAWS: &[Law] = &[
    Law {
        key: "product.qualify-composition",
        case: "composition",
        detail: COMPOSITION,
        invokes: 1,
    },
    Law {
        key: "product.qualify-linux",
        case: "linux",
        detail: "linux-provider",
        invokes: 0,
    },
    Law {
        key: "product.qualify-freeze",
        case: "freeze",
        detail: "freeze-mismatch empty freeze-mismatch",
        invokes: 0,
    },
    Law {
        key: "product.qualify-advice",
        case: "advice",
        detail: "advice-rejected",
        invokes: 0,
    },
    Law {
        key: "product.qualify-begin",
        case: "begin",
        detail: BEGIN,
        invokes: 1,
    },
];

pub(super) fn records(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = super::command::program(request)?;
    let mut outputs = Vec::with_capacity(LAWS.len() + 1);
    for law in LAWS {
        let stdout = super::command::run(&program, &["world", "qualify", law.case])?;
        let line = owned_line(&stdout)?;
        outputs.push((law, stdout, line));
    }
    let private_out = super::command::run(&program, &["world", "qualify", PRIVATE])?;
    let private = owned_line(&private_out)?;
    for (_, _, line) in &outputs {
        bound(line)?;
    }
    bound(&private)?;
    for (law, _, line) in &outputs {
        if !qualified(line, law) {
            return Err(fail("record-mismatch", law.key));
        }
    }
    if !private_refused(&private) {
        return Err(fail("record-mismatch", "product.qualify-private"));
    }
    let mut observations = Vec::with_capacity(outputs.len() + 1);
    for (law, stdout, _) in outputs {
        observations.push(observation(law.key, &stdout));
    }
    observations.push(observation("product.qualify-private", &private_out));
    Ok(observations)
}

fn qualified(line: &str, law: &Law) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some(law.case)
        && plain(line, "detail").ok() == Some(law.detail)
        && count(line, "invokes").ok() == Some(law.invokes)
}

fn private_refused(line: &str) -> bool {
    is_gap(line)
        && plain(line, "case").ok() == Some("")
        && plain(line, "reason").ok() == Some("private-leakage")
        && plain(line, "evidence").ok() == Some("")
}

fn bound(line: &str) -> Result<(), PackFailure> {
    if line.contains("private-material:") || line.contains("/home/") {
        return Err(fail("forbidden-output", "private"));
    }
    require_count(line, "effects", 0)?;
    require_count(line, "subprocesses", 0)?;
    require_plain(line, "resident", "true")?;
    Ok(())
}

fn owned_line(stdout: &str) -> Result<String, PackFailure> {
    one_line(stdout).map(str::to_owned)
}

fn is_record(line: &str) -> bool {
    line.starts_with("#(world/qualify-v1 ") || line.starts_with("#(world/qualify-v1)")
}

fn is_gap(line: &str) -> bool {
    line.starts_with("#(world/qualify-gap-v1 ") || line.starts_with("#(world/qualify-gap-v1)")
}

fn require_plain(record: &str, name: &str, expected: &str) -> Result<(), PackFailure> {
    if plain(record, name).ok() != Some(expected) {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

fn require_count(record: &str, name: &str, expected: u64) -> Result<(), PackFailure> {
    if count(record, name).ok() != Some(expected) {
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
                return field_token(&rest[key.len()..], name);
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

fn field_token<'a>(value: &'a str, name: &str) -> Result<&'a str, PackFailure> {
    if value.starts_with('"') {
        let mut escaped = false;
        for (index, ch) in value.char_indices().skip(1) {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => return Ok(&value[..=index]),
                _ => {}
            }
        }
        return Err(fail("bad-record", name));
    }
    let end = value
        .find([' ', ')'])
        .ok_or_else(|| fail("marker-absent", name))?;
    Ok(&value[..end])
}

fn one_line(stdout: &str) -> Result<&str, PackFailure> {
    let Some(line) = stdout.strip_suffix('\n') else {
        return Err(fail("marker-absent", "record"));
    };
    if line.contains('\n') {
        return Err(fail("marker-absent", "extra"));
    }
    if !is_record(line) && !is_gap(line) {
        return Err(fail("marker-absent", "world/qualify-v1"));
    }
    let root = balanced(line)?;
    if root.len() != line.len() {
        return Err(fail("marker-absent", "extra"));
    }
    Ok(root)
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
