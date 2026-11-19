// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Runs `world converge` and reads each case from its record.
//!
//! A caller fact is not a convergence query. Nested text is not a plan field.

use crate::{CheckObservation, PackFailure, PackRequest};

const PRIVATE: &str = "private-material:s3cret-token";
const LAWS: [&str; 6] = [
    "product.converge-preserve",
    "product.converge-invalidate",
    "product.converge-oscillate",
    "product.converge-parallel",
    "product.converge-honest",
    "product.converge-private",
];

pub(super) fn queries(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = super::command::program(request)?;
    let preserve_out = super::command::run(&program, &["world", "converge", "preserve"])?;
    let invalidate_out = super::command::run(&program, &["world", "converge", "invalidate"])?;
    let oscillate_out = super::command::run(&program, &["world", "converge", "oscillate"])?;
    let parallel_out = super::command::run(&program, &["world", "converge", "parallel"])?;
    let honest_out = super::command::run(&program, &["world", "converge", "honest"])?;
    let private_out = super::command::run(&program, &["world", "converge", PRIVATE])?;
    let preserve = one_line(&preserve_out)?;
    let invalidate = one_line(&invalidate_out)?;
    let oscillate = one_line(&oscillate_out)?;
    let parallel = one_line(&parallel_out)?;
    let honest = one_line(&honest_out)?;
    let private = one_line(&private_out)?;
    for line in [preserve, invalidate, oscillate, parallel, honest, private] {
        if line.contains("private-material:") || line.contains("/home/") {
            return Err(fail("forbidden-output", "private"));
        }
        require_count(line, "effects", 0)?;
        require_count(line, "subprocesses", 0)?;
        require_plain(line, "resident", "true")?;
    }
    laws(preserve, invalidate, oscillate, parallel, honest, private)?;
    Ok(vec![
        observation("product.converge-preserve", &preserve_out),
        observation("product.converge-invalidate", &invalidate_out),
        observation("product.converge-oscillate", &oscillate_out),
        observation("product.converge-parallel", &parallel_out),
        observation("product.converge-honest", &honest_out),
        observation("product.converge-private", &private_out),
    ])
}

fn laws(
    preserve: &str,
    invalidate: &str,
    oscillate: &str,
    parallel: &str,
    honest: &str,
    private: &str,
) -> Result<(), PackFailure> {
    if !preserved(preserve) {
        return Err(mismatch(0));
    }
    if !invalidated(invalidate) {
        return Err(mismatch(1));
    }
    if !oscillated(oscillate) {
        return Err(mismatch(2));
    }
    if !paralleled(parallel) {
        return Err(mismatch(3));
    }
    if !honest_case(honest) {
        return Err(mismatch(4));
    }
    if !private_refused(private) {
        return Err(mismatch(5));
    }
    Ok(())
}

fn preserved(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("preserve")
        && plain(line, "detail").ok()
            == Some("self-approval held 1 uncertain 1 same 1 later uncertain")
        && count(line, "invokes").ok() == Some(5)
}

fn invalidated(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("invalidate")
        && plain(line, "detail").ok() == Some("affected a closed b changed 1")
        && count(line, "invokes").ok() == Some(3)
}

fn oscillated(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("oscillate")
        && plain(line, "detail").ok() == Some("empty 1 seen note same 1")
        && count(line, "invokes").ok() == Some(5)
}

fn paralleled(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("parallel")
        && plain(line, "detail").ok() == Some("same 1 failures a:proof-failed,b:proof-failed")
        && count(line, "invokes").ok() == Some(2)
}

fn honest_case(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("honest")
        && plain(line, "detail").ok() == Some("incomplete need malformed bad same 1")
        && count(line, "invokes").ok() == Some(4)
}

fn private_refused(line: &str) -> bool {
    is_gap(line)
        && plain(line, "case").ok() == Some("")
        && plain(line, "reason").ok() == Some("private-leakage")
        && plain(line, "evidence").ok() == Some("")
}

fn mismatch(index: usize) -> PackFailure {
    fail("record-mismatch", LAWS[index])
}

fn is_record(line: &str) -> bool {
    line.starts_with("#(world/converge-v1 ") || line.starts_with("#(world/converge-v1)")
}

fn is_gap(line: &str) -> bool {
    line.starts_with("#(world/converge-gap-v1 ") || line.starts_with("#(world/converge-gap-v1)")
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
        return Err(fail("marker-absent", "world/converge-v1"));
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
