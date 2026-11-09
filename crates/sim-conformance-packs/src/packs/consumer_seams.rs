// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Outside consumer-seams specimen.
//!
//! The specimen calls the frozen executor through its public service. Fake
//! ports supply readiness, effects, and cancellation. The accepted closure
//! digest is the closure compiled into this package. An equal digest that is
//! not that closure is refused. This module does not read files, spawn
//! processes, or contact a network.

use std::collections::BTreeMap;

use crate::executor_freeze::{self, ClosureFault};
use crate::{CheckObservation, PackFailure, PackRequest};

#[path = "advice_verify.rs"]
mod advice_verify;

/// Refusal from the outside specimen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeamFault {
    /// The closure digest is missing, blank, or not the same before and after.
    FreezeMismatch,
    /// The closure listed no files.
    EmptyClosure,
    /// A closure path is empty or escapes its root.
    BadPath,
    /// A closure path was repeated.
    DuplicatePath,
    /// A provider name is a Linux adapter.
    LinuxProvider,
    /// The live revision moved and the page would have mixed two revisions.
    RevisionMoved {
        /// Revision the cursor still holds.
        retained: u64,
    },
    /// The reply names another execution.
    WrongExecution,
    /// The reply text is not the authorized answer.
    ChangedAnswer,
    /// The fenced revision is no longer live.
    StaleDirection,
    /// Advice was offered where a decision is required.
    AdviceInDecision,
    /// The same reply was presented again. The transition count stays put.
    Replayed,
    /// Selection is present and begin authority is absent.
    MissingBegin,
    /// A required durable object is absent.
    MissingObject,
    /// The subject was revoked.
    Revoked,
    /// Another source would pass the bound.
    Bound,
    /// A landing is refused while an effect is pending.
    Interrupted,
    /// Coverage treats a missing owner or leaf as sufficient.
    InsufficientCoverage,
    /// Advice was attached as a receipt or a decision.
    AdviceRejected,
}

impl SeamFault {
    /// Stable reason code.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::FreezeMismatch => "freeze-mismatch",
            Self::EmptyClosure => "empty-closure",
            Self::BadPath => "bad-path",
            Self::DuplicatePath => "duplicate-path",
            Self::LinuxProvider => "linux-provider",
            Self::RevisionMoved { .. } => "revision-moved",
            Self::WrongExecution => "wrong-execution",
            Self::ChangedAnswer => "changed-answer",
            Self::StaleDirection => "stale-direction",
            Self::AdviceInDecision => "advice-in-decision",
            Self::Replayed => "replayed-direction",
            Self::MissingBegin => "missing-begin",
            Self::MissingObject => "missing-object",
            Self::Revoked => "revoked",
            Self::Bound => "resource-bound",
            Self::Interrupted => "interrupted",
            Self::InsufficientCoverage => "insufficient-coverage",
            Self::AdviceRejected => "advice-rejected",
        }
    }
}

/// Counts the canonical specimen actually produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeamReport {
    /// Rows on the first retained page.
    pub first_page: usize,
    /// Hex identity of the revision the snapshot retained.
    pub retained_revision: String,
    /// Accepted direction transitions.
    pub transitions: u64,
    /// Effects after a stale reply.
    pub stale_effects: u64,
    /// Landing effects.
    pub effects: u64,
    /// Effects added by the second unchanged pass.
    pub hot_delta: u64,
    /// Model providers the specimen used.
    pub providers: Vec<String>,
    /// Lineage edges that name the revised source and are stale.
    pub stale_edges: usize,
    /// Unrelated proof key, unchanged by the revision.
    pub unrelated_proof: String,
    /// Sources accepted before the next one is refused.
    pub source_bound: usize,
}

/// One durable export of the outside owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Export {
    objects: BTreeMap<String, String>,
    sources: BTreeMap<String, String>,
    consumed: Option<String>,
    landing: String,
    edges: Vec<Edge>,
    live: u64,
    selected: bool,
    begin: bool,
    candidate: String,
}

impl Export {
    /// Drops one durable object.
    pub fn without(mut self, id: &str) -> Self {
        self.objects.remove(id);
        self.sources.remove(id);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Edge {
    commission: String,
    source: String,
    stale: bool,
}

/// Hashes sorted closure entries. The caller reads executor files.
pub fn capture_executor_subject_closure(entries: &[(&str, &[u8])]) -> Result<String, SeamFault> {
    executor_freeze::hash_closure(entries).map_err(|fault| match fault {
        ClosureFault::Empty => SeamFault::EmptyClosure,
        ClosureFault::BadPath => SeamFault::BadPath,
        ClosureFault::Duplicate => SeamFault::DuplicatePath,
    })
}

/// Runs the specimen when both digests are the compiled executor closure.
pub fn qualify(before: &str, after: &str) -> Result<SeamReport, SeamFault> {
    if !is_digest(before)
        || before != after
        || before != executor_freeze::compiled_executor_closure()
    {
        return Err(SeamFault::FreezeMismatch);
    }
    cases::run_canonical()
}

/// Runs the frozen specimen against the requested subject.
pub fn qualify_subject(before: &str, after: &str, subject: &str) -> Result<SeamReport, SeamFault> {
    if !is_digest(before)
        || before != after
        || before != executor_freeze::compiled_executor_closure()
    {
        return Err(SeamFault::FreezeMismatch);
    }
    cases::run_for_subject(subject)
}

/// Judges the consumer-seams scope from injected closure digests.
///
/// Freeze facts are read first. The specimen then runs for the requested
/// subject, and the pass records that subject.
pub fn check_subject(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let before = request
        .evidence
        .fact("executor-freeze-before")
        .ok_or(pack_failure("missing-freeze"))?;
    let after = request
        .evidence
        .fact("executor-freeze-after")
        .ok_or(pack_failure("missing-freeze"))?;
    crate::executor::require_subject(request.subject)?;
    let report = qualify_subject(before, after, request.subject)
        .map_err(|fault| pack_failure(fault.code()))?;
    Ok(vec![
        observation("consumer-seams.subject", request.subject),
        observation("consumer-seams.freeze", before),
        observation("consumer-seams.pages", &report.first_page.to_string()),
        observation(
            "consumer-seams.retained-revision",
            &report.retained_revision.to_string(),
        ),
        observation(
            "consumer-seams.transitions",
            &report.transitions.to_string(),
        ),
        observation(
            "consumer-seams.stale-effects",
            &report.stale_effects.to_string(),
        ),
        observation("consumer-seams.effects", &report.effects.to_string()),
        observation("consumer-seams.hot-delta", &report.hot_delta.to_string()),
        observation("consumer-seams.providers", &report.providers.join(",")),
        observation(
            "consumer-seams.stale-edges",
            &report.stale_edges.to_string(),
        ),
        observation("consumer-seams.unrelated-proof", &report.unrelated_proof),
        observation(
            "consumer-seams.source-bound",
            &report.source_bound.to_string(),
        ),
        observation(
            "consumer-seams.linux-providers",
            &report
                .providers
                .iter()
                .filter(|name| name.contains("linux"))
                .count()
                .to_string(),
        ),
    ])
}

#[path = "consumer_seam_cases.rs"]
mod cases;
pub use cases::{
    advice_rejected, advice_slot, changed_answer, direction_once, discover_affordance,
    hot_needs_begin, hot_noop, interrupt_once, open_with_linux, over_bound, renote_stales,
    reopen_keeps_direction, reopen_same, revoked_subject, scope_confusion, selection_is_distinct,
    selection_without_begin, snapshot_pages, wrong_execution,
};

fn observation(key: &str, value: &str) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

fn pack_failure(code: &'static str) -> PackFailure {
    PackFailure {
        code,
        detail: code.to_owned(),
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
