// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Domain-port observation for `release/nv12-09`.
//!
//! The scope runs `world snapshot` and `world snapshot-diff` on the program
//! named by `product.command`. Each admitted row keeps its own status, and
//! the read list is exactly the requested selector. An unchanged batch reports
//! no reads. A native realization stays unavailable and names no fallback.

use std::path::Path;

use crate::{CheckObservation, PackFailure, PackRequest};

const ROWS: &[(&str, &str, &str)] = &[
    ("ci/run", "uncertain", "ci"),
    ("filesystem/readme", "observed", "filesystem"),
    ("filesystem/secret", "denied", "filesystem"),
    ("git/HEAD", "timed-out", "git"),
    ("process/probe", "unavailable", "process"),
    ("registry/index", "absent", "registry"),
];

const ROWS_DOC: &str = "\
sim-world-rows-v1
label admitted
envelope model-log
ci ci/run uncertain -
filesystem filesystem/readme observed verified-source
filesystem filesystem/secret denied -
git git/HEAD timed-out -
process process/probe unavailable -
registry registry/index absent -
";

const NATIVE_DOC: &str = "\
sim-world-rows-v1
label native
realization native
";

const BEFORE_DOC: &str = "\
sim-world-rows-v1
label before
git git/HEAD timed-out -
";

const AFTER_DOC: &str = "\
sim-world-rows-v1
label after
git git/HEAD observed abc
";

const MODEL_POLICY: &str = "policy/explicit-model-v1";
const NATIVE_POLICY: &str = "policy/no-native-fallback-v1";

struct Node {
    tag: String,
    fields: Vec<(String, String)>,
}

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = crate::packs::product::program(request)?;
    let mut out = Vec::with_capacity(ROWS.len() + 3);
    for (selector, status, port) in ROWS {
        let stdout = run_stable(&program, &["world", "snapshot", ROWS_DOC, selector])?;
        require_snapshot(&stdout, "admitted", selector, status, port)?;
        out.push(observation(&format!("release.observe.{selector}"), &stdout));
    }
    let native = run_stable(
        &program,
        &["world", "snapshot", NATIVE_DOC, "filesystem/readme"],
    )?;
    require_native(&native)?;
    out.push(observation("release.observe.native", &native));
    let changed = run_stable(
        &program,
        &["world", "snapshot-diff", BEFORE_DOC, AFTER_DOC, "git/HEAD"],
    )?;
    require_changed(&changed)?;
    out.push(observation("release.observe.changed", &changed));
    let quiet = run_stable(
        &program,
        &[
            "world",
            "snapshot-diff",
            ROWS_DOC,
            ROWS_DOC,
            "filesystem/readme",
        ],
    )?;
    require_quiet(&quiet)?;
    out.push(observation("release.observe.quiet", &quiet));
    Ok(out)
}

fn run_stable(program: &Path, args: &[&str]) -> Result<String, PackFailure> {
    let first = crate::packs::product::run(program, args)?;
    let second = crate::packs::product::run(program, args)?;
    if first != second {
        let detail = match args {
            [_, _, _, selector] | [_, _, _, _, selector] => *selector,
            _ => "world",
        };
        return Err(fail("observation-unstable", detail));
    }
    Ok(first)
}

fn require_snapshot(
    stdout: &str,
    batch: &str,
    selector: &str,
    status: &str,
    port: &str,
) -> Result<(), PackFailure> {
    let node = require_tag(stdout, "world/snapshot-v1")?;
    require_meter(&node)?;
    require_string(&node, "batch", batch)?;
    require_string(&node, "selector", selector)?;
    require_list(&node, "reads", &[selector])?;
    require_list(&node, "ports", &[port])?;
    let row = observation_node(&node)?;
    require_string(&row, "selector", selector)?;
    require_string(&row, "status", status)?;
    require_bare(&row, "matched", "true")?;
    require_string(&row, "policy", MODEL_POLICY)?;
    require_string(&row, "port", port)?;
    require_nonempty(&row, "freshness")?;
    require_nonempty(&row, "provenance")?;
    if status == "observed" {
        require_string(&row, "value", "verified-source")?;
    } else {
        require_absent(&row, "value")?;
    }
    Ok(())
}

fn require_native(stdout: &str) -> Result<(), PackFailure> {
    let node = require_tag(stdout, "world/snapshot-v1")?;
    require_meter(&node)?;
    require_string(&node, "batch", "native")?;
    require_string(&node, "selector", "filesystem/readme")?;
    require_list(&node, "reads", &[])?;
    require_list(&node, "ports", &[])?;
    let row = observation_node(&node)?;
    require_string(&row, "selector", "filesystem/readme")?;
    require_string(&row, "status", "unavailable")?;
    require_bare(&row, "matched", "false")?;
    require_string(&row, "policy", NATIVE_POLICY)?;
    require_string(&row, "port", "native")?;
    require_string(&row, "provenance", "native-capability-missing")?;
    require_absent(&row, "value")?;
    Ok(())
}

fn require_quiet(stdout: &str) -> Result<(), PackFailure> {
    let node = require_tag(stdout, "world/observation-diff-v1")?;
    require_meter(&node)?;
    require_string(&node, "selector", "filesystem/readme")?;
    require_string(&node, "before", "admitted")?;
    require_string(&node, "after", "admitted")?;
    require_bare(&node, "changed", "false")?;
    require_list(&node, "reads", &[])?;
    require_list(&node, "ports", &[])?;
    require_absent(&node, "before-status")?;
    require_absent(&node, "after-status")?;
    require_absent(&node, "before-value")?;
    require_absent(&node, "after-value")?;
    Ok(())
}

fn require_changed(stdout: &str) -> Result<(), PackFailure> {
    let node = require_tag(stdout, "world/observation-diff-v1")?;
    require_meter(&node)?;
    require_string(&node, "selector", "git/HEAD")?;
    require_string(&node, "before", "before")?;
    require_string(&node, "after", "after")?;
    require_bare(&node, "changed", "true")?;
    require_string(&node, "before-status", "timed-out")?;
    require_string(&node, "after-status", "observed")?;
    require_list(&node, "reads", &["git/HEAD", "git/HEAD"])?;
    require_list(&node, "ports", &["git", "git"])?;
    Ok(())
}

fn require_meter(node: &Node) -> Result<(), PackFailure> {
    require_bare(node, "effects", "0")?;
    require_bare(node, "subprocesses", "0")?;
    require_bare(node, "resident", "true")?;
    Ok(())
}

fn observation_node(node: &Node) -> Result<Node, PackFailure> {
    let value = one(node, "observations")?;
    let inner = value
        .trim()
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| fail("bad-record", "observations"))?
        .trim();
    let row = parse_node(inner)?;
    if row.tag != "world/observation-v1" {
        return Err(fail("marker-absent", "world/observation-v1"));
    }
    Ok(row)
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

fn require_nonempty(node: &Node, name: &str) -> Result<(), PackFailure> {
    if unquote(one(node, name)?)?.is_empty() {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

fn require_absent(node: &Node, name: &str) -> Result<(), PackFailure> {
    if node.fields.iter().any(|(field, _)| field == name) {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

fn require_list(node: &Node, name: &str, expected: &[&str]) -> Result<(), PackFailure> {
    let got = quoted_list(one(node, name)?)?;
    if got.len() != expected.len() || got.iter().zip(expected).any(|(got, want)| got != want) {
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

fn quoted_list(value: &str) -> Result<Vec<String>, PackFailure> {
    let inner = value
        .trim()
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| fail("bad-record", "list"))?;
    let mut items = Vec::new();
    let mut rest = inner.trim();
    while !rest.is_empty() {
        if !rest.starts_with('"') {
            return Err(fail("bad-record", "list"));
        }
        let (item, consumed) = take_quoted(rest)?;
        items.push(item);
        rest = rest[consumed..].trim_start();
    }
    Ok(items)
}

fn take_quoted(value: &str) -> Result<(String, usize), PackFailure> {
    let mut escaped = false;
    let mut closed = None;
    let mut out = String::new();
    for (offset, ch) in value.char_indices().skip(1) {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => {
                closed = Some(offset + ch.len_utf8());
                break;
            }
            other => out.push(other),
        }
    }
    let Some(consumed) = closed else {
        return Err(fail("bad-record", "list"));
    };
    if escaped {
        return Err(fail("bad-record", "list"));
    }
    Ok((out, consumed))
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
