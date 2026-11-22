// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Reads one rendered prove record and its fields.

use crate::PackFailure;

use super::fail;

pub(super) struct Node {
    pub(super) tag: String,
    fields: Vec<(String, String)>,
}

pub(super) fn screen(stdout: &str) -> Result<(), PackFailure> {
    if stdout.contains("private-material:") || stdout.contains("/home/") {
        return Err(fail("forbidden-output", "private"));
    }
    Ok(())
}

pub(super) fn opened(stdout: &str, tag: &str) -> Result<Node, PackFailure> {
    screen(stdout)?;
    let line = crate::packs::product::prove_record(stdout)?;
    let node = parse_node(line)?;
    if node.tag != tag {
        return Err(fail("marker-absent", tag));
    }
    Ok(node)
}

pub(super) fn require_order(node: &Node, order: &[&str]) -> Result<(), PackFailure> {
    let names = node.fields.iter().map(|(name, _)| name.as_str());
    if node.fields.len() != order.len() || names.ne(order.iter().copied()) {
        return Err(fail("record-mismatch", "fields"));
    }
    Ok(())
}

pub(super) fn require_bare(node: &Node, name: &str, expected: &str) -> Result<(), PackFailure> {
    if one(node, name)? != expected {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

pub(super) fn require_string(node: &Node, name: &str, expected: &str) -> Result<(), PackFailure> {
    if string_field(node, name)? != expected {
        return Err(fail("record-mismatch", name));
    }
    Ok(())
}

pub(super) fn string_field(node: &Node, name: &str) -> Result<String, PackFailure> {
    unquote(one(node, name)?).map_err(|_| fail("bad-record", name))
}

pub(super) fn list_field(node: &Node, name: &str) -> Result<Vec<String>, PackFailure> {
    let raw = one(node, name)?;
    if !raw.starts_with('[') || !raw.ends_with(']') {
        return Err(fail("bad-record", name));
    }
    let mut rest = &raw[1..raw.len() - 1];
    if rest.is_empty() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    loop {
        let (item, consumed) = quoted_prefix(rest).map_err(|_| fail("bad-record", name))?;
        names.push(item);
        rest = &rest[consumed..];
        if rest.is_empty() {
            return Ok(names);
        }
        if !rest.starts_with(' ') {
            return Err(fail("bad-record", name));
        }
        rest = &rest[1..];
    }
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

pub(super) fn one<'a>(node: &'a Node, name: &str) -> Result<&'a str, PackFailure> {
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

fn unquote(value: &str) -> Result<String, ()> {
    let (text, consumed) = quoted_prefix(value)?;
    if consumed == value.len() {
        Ok(text)
    } else {
        Err(())
    }
}

fn quoted_prefix(value: &str) -> Result<(String, usize), ()> {
    let mut chars = value.chars();
    if chars.next() != Some('"') {
        return Err(());
    }
    let mut out = String::new();
    let mut escaped = false;
    let mut consumed = 1usize;
    for ch in chars {
        consumed += ch.len_utf8();
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return Ok((out, consumed)),
            other => out.push(other),
        }
    }
    Err(())
}
