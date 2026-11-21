// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! `release/nv12-11` runs `world succeed` on the named program.
//!
//! One specimen and a second project of the same public shape both close.
//! A private branch stays a gap. Caller closure facts do not supply the scope.

use std::path::Path;

use crate::{CheckObservation, PackFailure, PackRequest};

const CLOSED_FIELDS: usize = 24;
const GAP_FIELDS: usize = 5;
const CLOSED_TAG: &str = "world/succession-v1";
const GAP_TAG: &str = "world/succession-gap-v1";

struct Node {
    tag: String,
    fields: Vec<(String, String)>,
}

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = crate::packs::product::program(request)?;
    let lantern = run_stable(&program, "lantern", false)?;
    let harbor = run_stable(&program, "harbor", false)?;
    let branch = run_stable(&program, "lantern", true)?;
    require_closed(&lantern, "lantern")?;
    require_closed(&harbor, "harbor")?;
    require_gap(&branch)?;
    Ok(vec![
        observation("release.foreign.lantern", &lantern),
        observation("release.foreign.harbor", &harbor),
        observation("release.foreign.branch", &branch),
    ])
}

fn run_stable(program: &Path, project: &str, branch: bool) -> Result<String, PackFailure> {
    let export = export_text(project, branch);
    let change = format!("shade-1 {project} provider/shade observed shade-next");
    let args = ["world", "succeed", export.as_str(), change.as_str()];
    let first = crate::packs::product::run(program, &args)?;
    let second = crate::packs::product::run(program, &args)?;
    if first != second {
        let detail = if branch { "branch" } else { project };
        return Err(fail("observation-unstable", detail));
    }
    Ok(first)
}

fn export_text(project: &str, branch: bool) -> String {
    let mut text = format!(
        "\
export world/succession-export-v1
package sim-incremental-core 0.5.2 registry
package sim-storage-port 0.2.2 registry
package sim-lib-world 0.1.1 registry
project {project}
input provider/widget observed widget-body
input remote/hostile unavailable -
"
    );
    if branch {
        text.push_str("branch private\n");
    }
    text
}

fn require_closed(stdout: &str, project: &str) -> Result<(), PackFailure> {
    let node = require_tag(stdout, CLOSED_TAG)?;
    require_string(&node, "change", "shade-1")?;
    require_nonempty(&node, "change-id")?;
    require_string(&node, "closed", "portable")?;
    require_bare(&node, "converge-first", "1")?;
    require_bare(&node, "converge-second", "0")?;
    require_bare(&node, "deliver-first", "1")?;
    require_bare(&node, "deliver-second", "0")?;
    require_bare(&node, "effects", "0")?;
    require_bare(&node, "holds", "0")?;
    require_ids(&node)?;
    require_bare(&node, "land", "1")?;
    require_string(&node, "linux", "absent")?;
    require_bare(&node, "observed", "3")?;
    require_string(&node, "origin", "registry")?;
    let postimage = require_hex(&node, "postimage")?;
    let preimage = require_hex(&node, "preimage")?;
    if preimage == postimage {
        return Err(fail("record-mismatch", "postimage"));
    }
    require_string(&node, "project", project)?;
    require_bare(&node, "proved", "3")?;
    require_body(&node, project)?;
    require_string(&node, "recovery", "match")?;
    require_bare(&node, "recovery-reads", "2")?;
    require_bare(&node, "resident", "true")?;
    require_bare(&node, "seal", "2")?;
    require_bare(&node, "subprocesses", "0")?;
    if node.fields.len() != CLOSED_FIELDS {
        return Err(fail("record-mismatch", "fields"));
    }
    Ok(())
}

fn require_gap(stdout: &str) -> Result<(), PackFailure> {
    let node = require_tag(stdout, GAP_TAG)?;
    require_string(&node, "closed", "open")?;
    require_bare(&node, "effects", "0")?;
    require_string(&node, "reason", "private-branch")?;
    require_bare(&node, "resident", "true")?;
    require_bare(&node, "subprocesses", "0")?;
    if node.fields.len() != GAP_FIELDS {
        return Err(fail("record-mismatch", "fields"));
    }
    Ok(())
}

fn require_body(node: &Node, project: &str) -> Result<(), PackFailure> {
    let body = unquote(one(node, "record")?)?;
    let names_project = body.contains(&format!("project {project}"));
    if body.contains('\n') || !names_project || !body.contains("closed portable") {
        return Err(fail("record-mismatch", "record"));
    }
    Ok(())
}

fn require_ids(node: &Node) -> Result<(), PackFailure> {
    let value = one(node, "input-ids")?;
    if !value.starts_with('[') || !value.ends_with(']') || value == "[]" {
        return Err(fail("record-mismatch", "input-ids"));
    }
    Ok(())
}

fn require_hex(node: &Node, name: &str) -> Result<String, PackFailure> {
    let text = unquote(one(node, name)?)?;
    let hex = text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    if !hex {
        return Err(fail("record-mismatch", name));
    }
    Ok(text)
}

fn require_nonempty(node: &Node, name: &str) -> Result<(), PackFailure> {
    if unquote(one(node, name)?)?.is_empty() {
        return Err(fail("record-mismatch", name));
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
