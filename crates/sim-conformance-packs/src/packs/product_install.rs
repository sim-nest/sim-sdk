// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Runs `world promote` and reads the publication laws from its records.
//!
//! A caller fact is not an installation. Nested text is not a package count.

use crate::{CheckObservation, PackFailure, PackRequest};

const PRIVATE: &str = "private-material:s3cret-token";

struct Gap {
    key: &'static str,
    case: &'static str,
    reason: &'static str,
}

const GAPS: &[Gap] = &[
    Gap {
        key: "product.promote-path",
        case: "path",
        reason: "path-source",
    },
    Gap {
        key: "product.promote-receipt",
        case: "receipt",
        reason: "local-receipt",
    },
    Gap {
        key: "product.promote-model",
        case: "model",
        reason: "model-plan",
    },
    Gap {
        key: "product.promote-version",
        case: "version",
        reason: "missing-version",
    },
    Gap {
        key: "product.promote-checksum",
        case: "checksum",
        reason: "missing-checksum",
    },
    Gap {
        key: "product.promote-source",
        case: "source",
        reason: "missing-source",
    },
    Gap {
        key: "product.promote-order",
        case: "order",
        reason: "dependency-order",
    },
    Gap {
        key: "product.promote-revision",
        case: "revision",
        reason: "dependency-order",
    },
    Gap {
        key: "product.promote-linux",
        case: "linux",
        reason: "linux-requirement",
    },
    Gap {
        key: "product.promote-install",
        case: "install",
        reason: "missing-install",
    },
    Gap {
        key: "product.promote-unreconciled",
        case: "unreconciled",
        reason: "unreconciled",
    },
    Gap {
        key: "product.promote-overlap",
        case: "overlap",
        reason: "overlap",
    },
    Gap {
        key: "product.promote-empty",
        case: "empty",
        reason: "missing-package",
    },
    Gap {
        key: "product.promote-ci",
        case: "ci",
        reason: "missing-ci",
    },
    Gap {
        key: "product.promote-mirror",
        case: "mirror",
        reason: "missing-mirror",
    },
];

pub(super) fn records(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = super::command::program(request)?;
    let registry_out = super::command::run(&program, &["world", "promote", "registry"])?;
    let registry = owned_line(&registry_out)?;
    let mut gaps = Vec::with_capacity(GAPS.len());
    for gap in GAPS {
        let stdout = super::command::run(&program, &["world", "promote", gap.case])?;
        let line = owned_line(&stdout)?;
        gaps.push((gap, stdout, line));
    }
    let private_out = super::command::run(&program, &["world", "promote", PRIVATE])?;
    let private = owned_line(&private_out)?;
    bound(&registry)?;
    for (_, _, line) in &gaps {
        bound(line)?;
    }
    bound(&private)?;
    if !registry_case(&registry) {
        return Err(fail("record-mismatch", "product.promote-registry"));
    }
    for (gap, _, line) in &gaps {
        if !gap_case(line, gap) {
            return Err(fail("record-mismatch", gap.key));
        }
    }
    if !private_refused(&private) {
        return Err(fail("record-mismatch", "product.promote-private"));
    }
    let mut observations = Vec::with_capacity(gaps.len() + 2);
    observations.push(observation("product.promote-registry", &registry_out));
    for (gap, stdout, _) in gaps {
        observations.push(observation(gap.key, &stdout));
    }
    observations.push(observation("product.promote-private", &private_out));
    Ok(observations)
}

fn registry_case(line: &str) -> bool {
    is_record(line)
        && plain(line, "case").ok() == Some("registry")
        && count(line, "needs").ok() == Some(2)
        && count(line, "packages").ok() == Some(3)
}

fn gap_case(line: &str, gap: &Gap) -> bool {
    is_gap(line)
        && plain(line, "case").ok() == Some(gap.case)
        && plain(line, "reason").ok() == Some(gap.reason)
        && plain(line, "evidence").ok() == Some("")
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
    line.starts_with("#(world/promote-v1 ") || line.starts_with("#(world/promote-v1)")
}

fn is_gap(line: &str) -> bool {
    line.starts_with("#(world/promote-gap-v1 ") || line.starts_with("#(world/promote-gap-v1)")
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
        return Err(fail("marker-absent", "world/promote-v1"));
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
