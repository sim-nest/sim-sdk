// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-15` runs `world converge` on the named program.
//!
//! Five revision laws keep their exact details. A private case is a gap
//! and its text is not copied. Caller closure facts do not supply the scope.

use std::path::Path;

use crate::{CheckObservation, PackFailure, PackRequest};

const PRIVATE: &str = "private-material:s3cret-token";
const RECORD_FIELDS: usize = 6;
const GAP_FIELDS: usize = 6;
const RECORD_TAG: &str = "world/converge-v1";
const GAP_TAG: &str = "world/converge-gap-v1";
const RECORD_ORDER: [&str; RECORD_FIELDS] = [
    "case",
    "detail",
    "effects",
    "invokes",
    "resident",
    "subprocesses",
];
const GAP_ORDER: [&str; GAP_FIELDS] = [
    "case",
    "effects",
    "evidence",
    "reason",
    "resident",
    "subprocesses",
];

struct Law {
    case: &'static str,
    detail: &'static str,
    invokes: &'static str,
}

const LAWS: &[Law] = &[
    Law {
        case: "preserve",
        detail: "self-approval held 1 uncertain 1 same 1 later uncertain",
        invokes: "5",
    },
    Law {
        case: "invalidate",
        detail: "affected a closed b changed 1",
        invokes: "3",
    },
    Law {
        case: "oscillate",
        detail: "empty 1 seen note same 1",
        invokes: "5",
    },
    Law {
        case: "parallel",
        detail: "same 1 failures a:proof-failed,b:proof-failed",
        invokes: "2",
    },
    Law {
        case: "honest",
        detail: "incomplete need malformed bad same 1",
        invokes: "4",
    },
];

struct Node {
    tag: String,
    fields: Vec<(String, String)>,
}

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = crate::packs::product::program(request)?;
    let mut out = Vec::with_capacity(LAWS.len() + 1);
    for law in LAWS {
        let stdout = run_stable(&program, law.case)?;
        screen(&stdout)?;
        require_law(&stdout, law)?;
        out.push(observation(
            &format!("release.converge.{}", law.case),
            &stdout,
        ));
    }
    let private_out = run_stable(&program, PRIVATE)?;
    screen(&private_out)?;
    require_private(&private_out)?;
    out.push(observation("release.converge.private", &private_out));
    Ok(out)
}

fn run_stable(program: &Path, case: &str) -> Result<String, PackFailure> {
    let args = ["world", "converge", case];
    let first = crate::packs::product::run(program, &args)?;
    let second = crate::packs::product::run(program, &args)?;
    if first != second {
        let detail = if case == PRIVATE { "private" } else { case };
        return Err(fail("observation-unstable", detail));
    }
    Ok(first)
}

fn screen(stdout: &str) -> Result<(), PackFailure> {
    if stdout.contains("private-material:") || stdout.contains("/home/") {
        return Err(fail("forbidden-output", "private"));
    }
    Ok(())
}

fn require_law(stdout: &str, law: &Law) -> Result<(), PackFailure> {
    let node = require_tag(stdout, RECORD_TAG)?;
    require_string(&node, "case", law.case)?;
    require_string(&node, "detail", law.detail)?;
    require_bare(&node, "effects", "0")?;
    require_bare(&node, "invokes", law.invokes)?;
    require_bare(&node, "resident", "true")?;
    require_bare(&node, "subprocesses", "0")?;
    require_order(&node, &RECORD_ORDER)
}

fn require_private(stdout: &str) -> Result<(), PackFailure> {
    let node = require_tag(stdout, GAP_TAG)?;
    require_string(&node, "case", "")?;
    require_bare(&node, "effects", "0")?;
    require_string(&node, "evidence", "")?;
    require_string(&node, "reason", "private-leakage")?;
    require_bare(&node, "resident", "true")?;
    require_bare(&node, "subprocesses", "0")?;
    require_order(&node, &GAP_ORDER)
}

fn require_order(node: &Node, order: &[&str]) -> Result<(), PackFailure> {
    let names = node.fields.iter().map(|(name, _)| name.as_str());
    if node.fields.len() != order.len() || names.ne(order.iter().copied()) {
        return Err(fail("record-mismatch", "fields"));
    }
    Ok(())
}

fn require_tag(stdout: &str, tag: &str) -> Result<Node, PackFailure> {
    let node = record_node(stdout)?;
    if node.tag != tag {
        return Err(fail("marker-absent", tag));
    }
    Ok(node)
}

fn record_node(stdout: &str) -> Result<Node, PackFailure> {
    let mut lines = stdout.lines();
    let line = lines
        .next()
        .filter(|line| !line.is_empty())
        .ok_or_else(|| fail("marker-absent", "record"))?;
    if let Some(extra) = lines.next() {
        let receipt = extra.starts_with("#(world/load-receipt-v1 ")
            || extra.starts_with("#(prove/load-receipt-v1 ");
        if !receipt || !extra.ends_with(')') || lines.next().is_some() {
            return Err(fail("marker-absent", "load-receipt"));
        }
    }
    parse_node(line)
}

fn parse_node(text: &str) -> Result<Node, PackFailure> {
    let mut cursor = Cursor { text, pos: 0 };
    if !cursor.rest().starts_with("#(") {
        return Err(fail("bad-record", "node"));
    }
    cursor.pos += 2;
    let tag_start = cursor.pos;
    while cursor
        .peek()
        .is_some_and(|ch| !ch.is_whitespace() && ch != ')')
    {
        cursor.bump();
    }
    if tag_start == cursor.pos {
        return Err(fail("bad-record", "tag"));
    }
    let tag = text[tag_start..cursor.pos].to_owned();
    cursor.skip_ws();
    let mut fields = Vec::new();
    loop {
        match cursor.peek() {
            Some(')') => {
                cursor.bump();
                break;
            }
            Some('(') => fields.push(cursor.field()?),
            _ => return Err(fail("bad-record", "field")),
        }
        cursor.skip_ws();
    }
    if cursor.pos != text.len() {
        return Err(fail("bad-record", "node"));
    }
    Ok(Node { tag, fields })
}

struct Cursor<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    fn field(&mut self) -> Result<(String, String), PackFailure> {
        self.bump();
        let name_start = self.pos;
        while self
            .peek()
            .is_some_and(|ch| !ch.is_whitespace() && ch != ')')
        {
            self.bump();
        }
        if name_start == self.pos {
            return Err(fail("bad-record", "field"));
        }
        let name = self.text[name_start..self.pos].to_owned();
        if !self.peek().is_some_and(char::is_whitespace) {
            return Err(fail("bad-record", &name));
        }
        self.skip_ws();
        let value_start = self.pos;
        let mut depth = 0i32;
        let mut quoted = false;
        let mut escaped = false;
        loop {
            let Some(ch) = self.peek() else {
                return Err(fail("bad-record", &name));
            };
            if quoted {
                self.bump();
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
                '"' => {
                    quoted = true;
                    self.bump();
                }
                '(' => {
                    depth += 1;
                    self.bump();
                }
                ')' if depth == 0 => {
                    let value = self.text[value_start..self.pos].to_owned();
                    self.bump();
                    return Ok((name, value));
                }
                ')' => {
                    depth -= 1;
                    self.bump();
                }
                _ => {
                    self.bump();
                }
            }
        }
    }
}

fn one<'a>(node: &'a Node, name: &str) -> Result<&'a str, PackFailure> {
    let mut found = None;
    for (field, value) in &node.fields {
        if field == name {
            if found.is_some() {
                return Err(fail("bad-record", name));
            }
            found = Some(value.as_str());
        }
    }
    found.ok_or_else(|| fail("marker-absent", name))
}

fn require_bare(node: &Node, name: &str, expected: &str) -> Result<(), PackFailure> {
    if one(node, name)? != expected {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

fn require_string(node: &Node, name: &str, expected: &str) -> Result<(), PackFailure> {
    if unquote(one(node, name)?)? != expected {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

fn unquote(value: &str) -> Result<String, PackFailure> {
    let mut chars = value.chars();
    if chars.next() != Some('"') {
        return Err(fail("bad-record", "string"));
    }
    let mut out = String::new();
    let mut escaped = false;
    let mut closed = false;
    for ch in chars {
        if closed {
            return Err(fail("bad-record", "string"));
        }
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => closed = true,
            other => out.push(other),
        }
    }
    if !closed || escaped {
        return Err(fail("bad-record", "string"));
    }
    Ok(out)
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
