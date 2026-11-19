// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Runs `world land` and reads each case from its record.
//!
//! A caller fact is not a landing query. Nested text is not a plan field.

use crate::{CheckObservation, PackFailure, PackRequest};

const PRIVATE: &str = "private-material:s3cret-token";
const LAWS: [&str; 7] = [
    "product.land-foreign",
    "product.land-replay",
    "product.land-recover",
    "product.land-generated",
    "product.land-cache",
    "product.land-status",
    "product.land-private",
];

pub(super) fn queries(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = super::command::program(request)?;
    let foreign_out = super::command::run(&program, &["world", "land", "foreign"])?;
    let replay_out = super::command::run(&program, &["world", "land", "replay"])?;
    let recover_out = super::command::run(&program, &["world", "land", "recover"])?;
    let generated_out = super::command::run(&program, &["world", "land", "generated"])?;
    let cache_out = super::command::run(&program, &["world", "land", "cache"])?;
    let status_out = super::command::run(&program, &["world", "land", "status"])?;
    let private_out = super::command::run(&program, &["world", "land", PRIVATE])?;
    let foreign = one_line(&foreign_out)?;
    let replay = one_line(&replay_out)?;
    let recover = one_line(&recover_out)?;
    let generated = one_line(&generated_out)?;
    let cache = one_line(&cache_out)?;
    let status = one_line(&status_out)?;
    let private = one_line(&private_out)?;
    for line in [foreign, replay, recover, generated, cache, status, private] {
        if line.contains("private-material:") || line.contains("/home/") {
            return Err(fail("forbidden-output", "private"));
        }
        require_count(line, "effects", 0)?;
        require_count(line, "subprocesses", 0)?;
        require_plain(line, "resident", "true")?;
    }
    laws(foreign, replay, recover, generated, cache, status, private)?;
    Ok(vec![
        observation("product.land-foreign", &foreign_out),
        observation("product.land-replay", &replay_out),
        observation("product.land-recover", &recover_out),
        observation("product.land-generated", &generated_out),
        observation("product.land-cache", &cache_out),
        observation("product.land-status", &status_out),
        observation("product.land-private", &private_out),
    ])
}

fn laws(
    foreign: &str,
    replay: &str,
    recover: &str,
    generated: &str,
    cache: &str,
    status: &str,
    private: &str,
) -> Result<(), PackFailure> {
    if !foreign_case(foreign) {
        return Err(mismatch(0));
    }
    if !replayed(replay) {
        return Err(mismatch(1));
    }
    if !recovered(recover) {
        return Err(mismatch(2));
    }
    if !generated_case(generated) {
        return Err(mismatch(3));
    }
    if !cached(cache) {
        return Err(mismatch(4));
    }
    if !status_case(status) {
        return Err(mismatch(5));
    }
    if !private_refused(private) {
        return Err(mismatch(6));
    }
    Ok(())
}

fn foreign_case(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("foreign")
        && plain(line, "detail").ok() == Some("would-commit applied kept same 1")
        && count(line, "invokes").ok() == Some(3)
}

fn replayed(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("replay")
        && plain(line, "detail").ok() == Some("already-true same 1")
        && count(line, "invokes").ok() == Some(2)
}

fn recovered(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("recover")
        && plain(line, "detail").ok() == Some("restored exact kept 1")
        && count(line, "invokes").ok() == Some(3)
}

fn generated_case(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("generated")
        && plain(line, "detail").ok() == Some("generated-owner-bypass same 1")
        && count(line, "invokes").ok() == Some(2)
}

fn cached(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("cache")
        && plain(line, "detail").ok() == Some("unknown current stale same 1")
        && count(line, "invokes").ok() == Some(6)
}

fn status_case(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("status")
        && plain(line, "detail").ok() == Some("present missing same 1")
        && count(line, "invokes").ok() == Some(2)
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
    line.starts_with("#(world/land-v1 ") || line.starts_with("#(world/land-v1)")
}

fn is_gap(line: &str) -> bool {
    line.starts_with("#(world/land-gap-v1 ") || line.starts_with("#(world/land-gap-v1)")
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
        return Err(fail("marker-absent", "world/land-v1"));
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
