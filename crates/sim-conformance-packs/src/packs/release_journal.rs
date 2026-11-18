// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Re-execution of one injected journal export.
//!
//! The text names object bytes, entry references, and a side telemetry
//! token. The pack publishes those objects through `sim-lib-journal` and
//! checks replay, reopen, snapshot, shared payload identity, and telemetry
//! removal by running them. A phase digest list is not an export.

use std::collections::BTreeMap;
use std::sync::Arc;

use sim_kernel::Symbol;
use sim_lib_journal::{
    Journal, JournalBackend, JournalEntry, JournalError, JournalHead, JournalObject, MemoryBackend,
    StoredState, replay,
};

use crate::{CheckObservation, PackFailure};

pub(super) const FACT: &str = "journal.export";

const MAX_BYTES: usize = 65_536;
const HEADER: &str = "journal-export-v1";

struct ParsedEntry {
    sequence: u64,
    namespace: String,
    name: String,
    object: usize,
}

struct Parsed {
    objects: Vec<Vec<u8>>,
    entries: Vec<ParsedEntry>,
    telemetry: Vec<String>,
}

pub(super) struct Inspected {
    pub observations: Vec<CheckObservation>,
    pub left: Vec<u8>,
    pub right: Vec<u8>,
    pub storage: String,
}

/// Object bytes, entries, and side telemetry from one executed journal.
pub struct ExecutedJournal {
    /// Payload bytes passed to `JournalObject::from_bytes`, in ordinal order.
    pub objects: Vec<Vec<u8>>,
    /// Entries in sequence order. `object` is an index into `objects`.
    pub entries: Vec<ExecutedEntry>,
    /// Tokens recorded beside the journal, never published as entries.
    pub telemetry: Vec<String>,
}

/// One executed entry. The previous id is not stored; replay derives it.
pub struct ExecutedEntry {
    /// Gapless sequence, starting at zero.
    pub sequence: u64,
    /// Kind namespace.
    pub namespace: String,
    /// Kind name.
    pub name: String,
    /// Ordinal of the payload object.
    pub object: usize,
}

/// Renders `run` as the export text `observe_journal_completion` re-executes.
pub fn format_executed_journal(run: &ExecutedJournal) -> String {
    let mut text = String::from("journal-export-v1\n");
    for object in &run.objects {
        text.push_str("object ");
        text.push_str(&hex_bytes(object));
        text.push('\n');
    }
    for entry in &run.entries {
        text.push_str("entry ");
        text.push_str(&entry.sequence.to_string());
        text.push(' ');
        text.push_str(&entry.namespace);
        text.push(' ');
        text.push_str(&entry.name);
        text.push(' ');
        text.push_str(&entry.object.to_string());
        text.push('\n');
    }
    for token in &run.telemetry {
        text.push_str("telemetry ");
        text.push_str(token);
        text.push('\n');
    }
    text
}

pub(super) fn inspect(body: &str) -> Result<Inspected, PackFailure> {
    let parsed = parse(body, true)?;
    let full = execute(&parsed)?;
    let stripped = execute(&parse(&strip_telemetry(body), false)?)?;
    if full.encoding != stripped.encoding {
        return Err(fail("missing-journal", FACT));
    }
    if full.encoding == body.as_bytes() {
        return Err(fail("bad-record", FACT));
    }
    let shared = shared_payload(&full.entries).ok_or_else(|| fail("missing-journal", FACT))?;
    let observations = vec![
        observation("journal.replay", hex_bytes(&full.encoding)),
        observation("journal.reopen", hex_bytes(&full.reopened)),
        observation("journal.snapshot", hex_bytes(&full.snapshot)),
        observation("journal.reference", hex_bytes(&shared.bytes)),
        observation("journal.telemetry", hex_bytes(&stripped.encoding)),
    ];
    if full.encoding != full.again
        || full.encoding != full.reopened
        || full.encoding != full.snapshot
        || full.encoding != stripped.encoding
    {
        return Err(fail("missing-journal", FACT));
    }
    Ok(Inspected {
        observations,
        left: full.encoding.clone(),
        right: full.again,
        storage: hex_bytes(&full.encoding),
    })
}

struct Run {
    encoding: Vec<u8>,
    again: Vec<u8>,
    reopened: Vec<u8>,
    snapshot: Vec<u8>,
    entries: Vec<JournalEntry>,
}

fn execute(parsed: &Parsed) -> Result<Run, PackFailure> {
    let published = publish(parsed)?;
    let first = replay_entries(&published.state)?;
    let second = replay_entries(&published.state)?;
    let snapshot = published
        .journal
        .verified_snapshot()
        .map_err(journal_fail)?;
    if snapshot.entries() != first.as_slice() {
        return Err(fail("missing-journal", "snapshot"));
    }
    for entry in snapshot.entries() {
        for payload in &entry.payloads {
            if snapshot.datum(payload).is_none() {
                return Err(fail("missing-journal", "snapshot"));
            }
        }
    }
    let reopened = reopen(&published.state)?;
    if reopened != published.head {
        return Err(fail("missing-journal", "reopen"));
    }
    let reopened_entries = replay_entries(&read_reopened(&published.state)?)?;
    if !corrupt_refuses(&published.state) {
        return Err(fail("missing-journal", "corrupt"));
    }
    if shared_payload(&first).is_none() {
        return Err(fail("missing-journal", "reference"));
    }
    let encoding = encode_entries(&first);
    for token in &parsed.telemetry {
        if encoding
            .windows(token.len())
            .any(|window| window == token.as_bytes())
        {
            return Err(fail("missing-journal", "telemetry"));
        }
    }
    Ok(Run {
        encoding,
        again: encode_entries(&second),
        reopened: encode_entries(&reopened_entries),
        snapshot: encode_entries(snapshot.entries()),
        entries: first,
    })
}

struct Published {
    journal: Journal<Arc<MemoryBackend>>,
    state: StoredState,
    head: JournalHead,
}

fn publish(parsed: &Parsed) -> Result<Published, PackFailure> {
    let backend = Arc::new(MemoryBackend::new());
    let journal = Journal::new(Arc::clone(&backend));
    let lease = journal.acquire_lease().map_err(journal_fail)?;
    let objects = parsed
        .objects
        .iter()
        .map(|bytes| JournalObject::from_bytes(bytes.clone()))
        .collect::<Vec<_>>();
    let mut seen = vec![false; objects.len()];
    let mut previous = None;
    let mut head = None;
    for entry in &parsed.entries {
        let object = objects
            .get(entry.object)
            .ok_or_else(|| fail("bad-record", FACT))?;
        let fact = JournalEntry::new(
            entry.sequence,
            previous.clone(),
            Symbol::qualified(entry.namespace.as_str(), entry.name.as_str()),
            vec![object.id.clone()],
        );
        let batch = if seen[entry.object] {
            Vec::new()
        } else {
            seen[entry.object] = true;
            vec![object.clone()]
        };
        head = Some(
            journal
                .publish(&lease, head.as_ref(), batch, vec![fact.clone()])
                .map_err(journal_fail)?,
        );
        previous = Some(fact.id);
    }
    if seen.iter().any(|item| !item) {
        return Err(fail("missing-journal", "reference"));
    }
    let state = backend.read_state().map_err(journal_fail)?;
    let head = head.ok_or_else(|| fail("missing-journal", FACT))?;
    Ok(Published {
        journal,
        state,
        head,
    })
}

fn read_reopened(state: &StoredState) -> Result<StoredState, PackFailure> {
    let backend = Arc::new(MemoryBackend::new());
    let journal = Journal::new(Arc::clone(&backend));
    let head = reopen_into(&journal, state)?;
    let restored = backend.read_state().map_err(journal_fail)?;
    if restored.head.as_ref() != Some(&head) {
        return Err(fail("missing-journal", "reopen"));
    }
    Ok(restored)
}

fn reopen(state: &StoredState) -> Result<JournalHead, PackFailure> {
    let journal = Journal::new(MemoryBackend::new());
    reopen_into(&journal, state)
}

fn reopen_into<B: JournalBackend>(
    journal: &Journal<B>,
    state: &StoredState,
) -> Result<JournalHead, PackFailure> {
    let lease = journal.acquire_lease().map_err(journal_fail)?;
    let mut objects = Vec::new();
    for datum in state.datums.values() {
        objects.push(JournalObject::from_datum(datum.clone()).map_err(journal_fail)?);
    }
    let entries = state.entries.values().cloned().collect::<Vec<_>>();
    journal
        .publish(&lease, None, objects, entries)
        .map_err(journal_fail)
}

fn replay_entries(state: &StoredState) -> Result<Vec<JournalEntry>, PackFailure> {
    Ok(replay(state.clone()).map_err(journal_fail)?.collect())
}

fn corrupt_refuses(state: &StoredState) -> bool {
    let mut corrupt = state.clone();
    let Some(bytes) = corrupt.objects.values_mut().next() else {
        return false;
    };
    if bytes.is_empty() {
        return false;
    }
    bytes[0] ^= 0xff;
    replay(corrupt).is_err()
}

fn shared_payload(entries: &[JournalEntry]) -> Option<sim_kernel::ContentId> {
    let mut counts = BTreeMap::<sim_kernel::ContentId, usize>::new();
    for entry in entries {
        for payload in &entry.payloads {
            *counts.entry(payload.clone()).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .find(|(_, count)| *count >= 2)
        .map(|(id, _)| id)
}

fn encode_entries(entries: &[JournalEntry]) -> Vec<u8> {
    let mut text = String::new();
    for entry in entries {
        text.push_str(&entry.sequence.to_string());
        text.push(' ');
        text.push_str(&entry.id.algorithm.as_qualified_str());
        text.push(' ');
        text.push_str(&hex_bytes(&entry.id.bytes));
        text.push('\n');
    }
    text.into_bytes()
}

fn strip_telemetry(body: &str) -> String {
    let mut text = String::new();
    for line in body.lines() {
        if line.starts_with("telemetry ") {
            continue;
        }
        text.push_str(line);
        text.push('\n');
    }
    text
}

fn parse(text: &str, require_telemetry: bool) -> Result<Parsed, PackFailure> {
    if text.len() > MAX_BYTES {
        return Err(fail("bound", FACT));
    }
    if text.contains('\r') {
        return Err(fail("bad-record", FACT));
    }
    if text.contains("shade-1") {
        return Err(fail("construction-specimen", FACT));
    }
    if text.contains("PortableClosed")
        || text.contains("publication-confirmed")
        || text.contains("completion/claim-")
    {
        return Err(fail("local-publication-claim", FACT));
    }
    if text.is_empty() || text.chars().all(|ch| ch == '\n') {
        return Err(fail("missing-journal", FACT));
    }
    if !text.ends_with('\n') {
        return Err(fail("bad-record", FACT));
    }
    let lines = content_lines(text)?;
    if lines.iter().all(|line| is_phase_digest(line)) {
        return Err(fail("fabricated-digest-list", FACT));
    }
    if lines.first().copied() != Some(HEADER) {
        return Err(fail("bad-record", FACT));
    }
    let mut objects = Vec::new();
    let mut entries = Vec::new();
    let mut telemetry = Vec::new();
    for line in lines.into_iter().skip(1) {
        let mut parts = line.split(' ');
        match parts.next() {
            Some("object") => objects.push(object_bytes(parts)?),
            Some("entry") => entries.push(entry_line(parts, objects.len())?),
            Some("telemetry") => telemetry.push(telemetry_token(parts)?),
            _ => return Err(fail("bad-record", FACT)),
        }
    }
    if objects.is_empty() || entries.is_empty() {
        return Err(fail("missing-journal", FACT));
    }
    if require_telemetry && telemetry.is_empty() {
        return Err(fail("missing-journal", "telemetry"));
    }
    check_entries(&objects, &entries, &telemetry)?;
    Ok(Parsed {
        objects,
        entries,
        telemetry,
    })
}

fn content_lines(text: &str) -> Result<Vec<&str>, PackFailure> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last().copied() == Some("") {
        lines.pop();
    }
    if lines.iter().any(|line| line.is_empty()) {
        return Err(fail("bad-record", FACT));
    }
    Ok(lines)
}

fn object_bytes<'a>(mut parts: impl Iterator<Item = &'a str>) -> Result<Vec<u8>, PackFailure> {
    let Some(hex) = parts.next() else {
        return Err(fail("bad-record", FACT));
    };
    if parts.next().is_some() || hex.len() < 2 || hex.len() % 2 != 0 || !is_lower_hex(hex) {
        return Err(fail("bad-record", FACT));
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let digits = hex.as_bytes();
    for index in (0..digits.len()).step_by(2) {
        let high = hex_value(digits[index])?;
        let low = hex_value(digits[index + 1])?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn entry_line<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    objects: usize,
) -> Result<ParsedEntry, PackFailure> {
    let sequence =
        canonical_u64(parts.next().unwrap_or("")).ok_or_else(|| fail("bad-record", FACT))?;
    let namespace = token(parts.next().unwrap_or(""))?;
    let name = token(parts.next().unwrap_or(""))?;
    let object =
        canonical_u64(parts.next().unwrap_or("")).ok_or_else(|| fail("bad-record", FACT))?;
    if parts.next().is_some() {
        return Err(fail("bad-record", FACT));
    }
    let object = usize::try_from(object).map_err(|_| fail("bad-record", FACT))?;
    if object >= objects {
        return Err(fail("bad-record", FACT));
    }
    Ok(ParsedEntry {
        sequence,
        namespace,
        name,
        object,
    })
}

fn telemetry_token<'a>(mut parts: impl Iterator<Item = &'a str>) -> Result<String, PackFailure> {
    let token = token(parts.next().unwrap_or(""))?;
    if parts.next().is_some() {
        return Err(fail("bad-record", FACT));
    }
    Ok(token)
}

fn check_entries(
    objects: &[Vec<u8>],
    entries: &[ParsedEntry],
    telemetry: &[String],
) -> Result<(), PackFailure> {
    let mut seen_bytes = BTreeMap::<Vec<u8>, usize>::new();
    for (index, bytes) in objects.iter().enumerate() {
        if seen_bytes.insert(bytes.clone(), index).is_some() {
            return Err(fail("missing-journal", "reference"));
        }
    }
    let mut counts = vec![0usize; objects.len()];
    for (index, entry) in entries.iter().enumerate() {
        if entry.sequence != u64::try_from(index).unwrap_or(u64::MAX) {
            return Err(fail("bad-record", FACT));
        }
        counts[entry.object] += 1;
        let kind = format!("{}/{}", entry.namespace, entry.name);
        for token in telemetry {
            if token == &entry.namespace || token == &entry.name || token == &kind {
                return Err(fail("missing-journal", "telemetry"));
            }
            if objects.iter().any(|bytes| {
                token.len() <= bytes.len()
                    && bytes
                        .windows(token.len())
                        .any(|window| window == token.as_bytes())
            }) {
                return Err(fail("missing-journal", "telemetry"));
            }
        }
    }
    if !counts.iter().any(|count| *count >= 2) {
        return Err(fail("missing-journal", "reference"));
    }
    Ok(())
}

fn is_phase_digest(line: &str) -> bool {
    let Some((name, digest)) = line.split_once(' ') else {
        return false;
    };
    let Some(index) = name.strip_prefix("phase/") else {
        return false;
    };
    index.len() == 2
        && index.bytes().all(|byte| byte.is_ascii_digit())
        && digest.len() == 64
        && is_lower_hex(digest)
        && !digest.contains(' ')
}

fn token(value: &str) -> Result<String, PackFailure> {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Ok(value.to_owned())
    } else {
        Err(fail("bad-record", FACT))
    }
}

fn canonical_u64(value: &str) -> Option<u64> {
    if value.is_empty() || (value.len() > 1 && value.starts_with('0')) {
        return None;
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hex_value(byte: u8) -> Result<u8, PackFailure> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(fail("bad-record", FACT)),
    }
}

fn observation(key: &str, value: impl Into<String>) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.into(),
    }
}

fn journal_fail(error: JournalError) -> PackFailure {
    fail("missing-journal", &error.to_string())
}

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}

pub(super) fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
