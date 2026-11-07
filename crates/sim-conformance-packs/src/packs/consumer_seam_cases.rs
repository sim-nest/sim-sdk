// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Scenario checks for the outside consumer.

use super::{
    PAGE_ROWS, PROVIDERS, SeamFault, SeamReport, Session, coverage_sufficient, providers_used,
};

/// Refuses a provider set that names a Linux adapter.
pub fn open_with_linux() -> Result<(), SeamFault> {
    Session::open(&["model/snapshot", "linux/native"]).map(|_| ())
}

/// Reads two pages at one revision, resolves one evidence ref, then requires a gap.
pub fn snapshot_pages() -> Result<(usize, u64), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    let mut cursor = 0usize;
    let held = session.live;
    let first = session.read_page(held, &mut cursor)?;
    let second = session.read_page(held, &mut cursor)?;
    let mixed = first
        .iter()
        .any(|(id, _)| second.iter().any(|(other, _)| id == other));
    if first.len() != PAGE_ROWS || second.len() != PAGE_ROWS || mixed {
        return Err(SeamFault::Bound);
    }
    if session.resolve_evidence(held, "fact/alpha")? != "a" {
        return Err(SeamFault::MissingObject);
    }
    session.advance();
    if session.read_page(held, &mut cursor).is_ok() {
        return Err(SeamFault::RevisionMoved { retained: 0 });
    }
    match session.resolve_evidence(held, "fact/alpha") {
        Err(SeamFault::RevisionMoved { retained }) if retained == held => {
            Ok((first.len(), retained))
        }
        Err(fault) => Err(fault),
        Ok(_) => Err(SeamFault::RevisionMoved { retained: 0 }),
    }
}

/// Accepts one reply, ignores an exact replay, and refuses the stale fence.
pub fn direction_once() -> Result<(u64, u64), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    let request = session.ask()?;
    session.reply(
        &request.execution,
        request.revision,
        &request.answer,
        "decision",
    )?;
    let err = session.reply(
        &request.execution,
        request.revision,
        &request.answer,
        "decision",
    );
    if err != Err(SeamFault::Replayed) || session.transitions != 1 {
        return Err(SeamFault::Replayed);
    }
    let effects = session.effects;
    session.advance();
    match session.reply(
        &request.execution,
        request.revision,
        &request.answer,
        "decision",
    ) {
        Err(SeamFault::StaleDirection) if session.effects == effects => {
            Ok((session.transitions, session.effects))
        }
        Err(fault) => Err(fault),
        Ok(()) => Err(SeamFault::StaleDirection),
    }
}

/// Refuses a reply that names a different execution and changes nothing.
pub fn wrong_execution() -> Result<(), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    let request = session.ask()?;
    let err = session.reply(
        "execution/other",
        request.revision,
        &request.answer,
        "decision",
    );
    if err == Err(SeamFault::WrongExecution) && session.transitions == 0 {
        Ok(())
    } else {
        Err(SeamFault::WrongExecution)
    }
}

/// Refuses a different answer.
pub fn changed_answer() -> Result<(), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    let request = session.ask()?;
    let err = session.reply(
        &request.execution,
        request.revision,
        "not-the-answer",
        "decision",
    );
    if err == Err(SeamFault::ChangedAnswer) && session.transitions == 0 {
        Ok(())
    } else {
        Err(SeamFault::ChangedAnswer)
    }
}

/// Refuses advice placed in the decision slot.
pub fn advice_slot() -> Result<(), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    let request = session.ask()?;
    let err = session.reply(
        &request.execution,
        request.revision,
        &request.answer,
        "advice",
    );
    if err == Err(SeamFault::AdviceInDecision) && session.transitions == 0 {
        Ok(())
    } else {
        Err(SeamFault::AdviceInDecision)
    }
}

/// Selects, refuses execution without begin, then lands once begin is separate.
pub fn selection_without_begin() -> Result<(u64, usize, String), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    session.note_source("source/a", "alpha")?;
    session.note_source("source/b", "beta")?;
    let unrelated = session.proof_key("source/b")?;
    session.note_commission("commission/a", "source/a")?;
    session.select("candidate/1")?;
    if session.execute() != Err(SeamFault::MissingBegin) || session.effects != 0 {
        return Err(SeamFault::MissingBegin);
    }
    session.grant_begin();
    session.execute()?;
    if !coverage_sufficient(&session.proof_view()) {
        return Err(SeamFault::InsufficientCoverage);
    }
    session.revise_source("source/a", "alpha-next")?;
    let stale = session.edges.iter().filter(|edge| edge.stale).count();
    let affected = session.proof_view();
    if session.proof_key("source/b")? != unrelated
        || stale != 1
        || affected.gaps.len() != 1
        || coverage_sufficient(&affected)
    {
        return Err(SeamFault::InsufficientCoverage);
    }
    Ok((session.effects, stale, unrelated))
}

/// Refuses advice as a receipt or a decision, and a missing owner as coverage.
pub fn advice_rejected() -> Result<(), SeamFault> {
    let session = Session::open(PROVIDERS)?;
    let view = session.proof_view();
    let advice = session.advise(&view);
    if session.attach_receipt(&advice) != Err(SeamFault::AdviceRejected) {
        return Err(SeamFault::AdviceRejected);
    }
    if session.attach_decision(&advice) != Err(SeamFault::AdviceRejected) {
        return Err(SeamFault::AdviceRejected);
    }
    let mut missing = view.clone();
    missing.owner = None;
    missing.leaves.clear();
    if coverage_sufficient(&missing) {
        return Err(SeamFault::InsufficientCoverage);
    }
    if !coverage_sufficient(&view) {
        return Err(SeamFault::InsufficientCoverage);
    }
    Ok(())
}

/// Reopens the same landing, and refuses an export that lost an object.
pub fn reopen_same() -> Result<(), SeamFault> {
    let session = landed()?;
    let export = session.export();
    let mut restored = Session::reopen(&export)?;
    if restored.consumed != session.consumed
        || restored.landing != session.landing
        || restored.edges != session.edges
        || restored.candidate != session.candidate
        || restored.selected != session.selected
        || restored.begin != session.begin
        || restored.landing_digest() != restored.landing
        || restored.hot_pass()?
    {
        return Err(SeamFault::MissingObject);
    }
    let round = restored.export();
    let twice = Session::reopen(&round)?;
    if twice.landing != restored.landing
        || twice.consumed != restored.consumed
        || twice.edges != restored.edges
    {
        return Err(SeamFault::MissingObject);
    }
    let broken = export.clone().without("artifact/1");
    if !matches!(Session::reopen(&broken), Err(SeamFault::MissingObject)) {
        return Err(SeamFault::MissingObject);
    }
    let broken_candidate = export.clone().without("candidate/1");
    if !matches!(
        Session::reopen(&broken_candidate),
        Err(SeamFault::MissingObject)
    ) {
        return Err(SeamFault::MissingObject);
    }
    let broken_source = export.clone().without("source/a");
    if !matches!(
        Session::reopen(&broken_source),
        Err(SeamFault::MissingObject)
    ) {
        return Err(SeamFault::MissingObject);
    }
    let broken_commission = export.without("commission/a");
    match Session::reopen(&broken_commission) {
        Err(SeamFault::MissingObject) => Ok(()),
        Ok(_) => Err(SeamFault::MissingObject),
        Err(fault) => Err(fault),
    }
}

/// Interrupts a pending effect before it counts, and fences a landing.
pub fn interrupt_once() -> Result<(), SeamFault> {
    let mut session = landed()?;
    let effects = session.effects;
    session.begin_effect()?;
    if session.execute() != Err(SeamFault::Interrupted) || session.effects != effects {
        return Err(SeamFault::Interrupted);
    }
    if session.hot_pass() != Err(SeamFault::Interrupted) || session.effects != effects {
        return Err(SeamFault::Interrupted);
    }
    session.interrupt()?;
    if session.pending || session.hot_pass()? || session.effects != effects {
        return Err(SeamFault::Interrupted);
    }
    Ok(())
}

/// A revoked subject stays unreadable.
pub fn revoked_subject() -> Result<(), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    session.revoke("subject/a");
    match session.read_subject("subject/a") {
        Err(SeamFault::Revoked) => Ok(()),
        Ok(()) => Err(SeamFault::Revoked),
        Err(fault) => Err(fault),
    }
}

/// A second pass over an unchanged landing performs no effect.
pub fn hot_noop() -> Result<u64, SeamFault> {
    let mut session = landed()?;
    let before = session.effects;
    let worked = session.hot_pass()?;
    if worked || session.effects != before {
        return Err(SeamFault::Bound);
    }
    Ok(session.effects - before)
}

/// Noting an existing source revises it and stales only its edges.
pub fn renote_stales() -> Result<(), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    session.note_source("source/a", "alpha")?;
    session.note_commission("commission/a", "source/a")?;
    let before = session.proof_key("source/a")?;
    session.note_source("source/a", "alpha-next")?;
    let stale = session.edges.iter().filter(|edge| edge.stale).count();
    if session.proof_key("source/a")? == before
        || stale != 1
        || coverage_sufficient(&session.proof_view())
    {
        return Err(SeamFault::InsufficientCoverage);
    }
    for name in ["source/b", "source/c", "source/d"] {
        session.note_source(name, name)?;
    }
    session.note_source("source/a", "alpha-again")?;
    if session.sources.len() != 4 {
        return Err(SeamFault::Bound);
    }
    Ok(())
}

/// Reopen keeps the revision, so the consumed answer cannot be spent again.
pub fn reopen_keeps_direction() -> Result<(), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    session.advance();
    let request = session.ask()?;
    session.reply(
        &request.execution,
        request.revision,
        &request.answer,
        "decision",
    )?;
    let export = session.export();
    let mut restored = Session::reopen(&export)?;
    if restored.live != session.live
        || restored.consumed != session.consumed
        || restored.selected
        || restored.begin
        || !restored.candidate.is_empty()
    {
        return Err(SeamFault::MissingObject);
    }
    if restored.execute() != Err(SeamFault::MissingObject) || restored.effects != 0 {
        return Err(SeamFault::MissingBegin);
    }
    if restored.hot_pass() != Err(SeamFault::MissingObject) || restored.effects != 0 {
        return Err(SeamFault::MissingBegin);
    }
    let again = restored.ask()?;
    let err = restored.reply(&again.execution, again.revision, &again.answer, "decision");
    if err == Err(SeamFault::Replayed) && restored.transitions == 0 {
        Ok(())
    } else {
        Err(SeamFault::Replayed)
    }
}

/// A new landing needs the same selection and begin gates as execution.
pub fn hot_needs_begin() -> Result<(), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    if session.hot_pass() != Err(SeamFault::MissingObject) || session.effects != 0 {
        return Err(SeamFault::MissingBegin);
    }
    if session.select("") != Err(SeamFault::MissingObject) {
        return Err(SeamFault::MissingObject);
    }
    session.select("candidate/1")?;
    if session.hot_pass() != Err(SeamFault::MissingBegin) || session.effects != 0 {
        return Err(SeamFault::MissingBegin);
    }
    Ok(())
}

/// Two candidates are different lineage, and reopen keeps the selected one.
pub fn selection_is_distinct() -> Result<(), SeamFault> {
    let mut first = Session::open(PROVIDERS)?;
    first.note_source("source/a", "alpha")?;
    first.note_commission("commission/a", "source/a")?;
    first.select("candidate/1")?;
    let mut second = Session::open(PROVIDERS)?;
    second.note_source("source/a", "alpha")?;
    second.note_commission("commission/a", "source/a")?;
    second.select("candidate/2")?;
    if first.landing_digest() == second.landing_digest() {
        return Err(SeamFault::MissingObject);
    }
    let restored = Session::reopen(&first.export())?;
    if restored.candidate != "candidate/1" || !restored.selected || restored.begin {
        return Err(SeamFault::MissingObject);
    }
    let mut session = Session::open(PROVIDERS)?;
    if session.note_source("a:b", "x") != Err(SeamFault::MissingObject)
        || session.note_source("a\nb", "x") != Err(SeamFault::MissingObject)
    {
        return Err(SeamFault::MissingObject);
    }
    let hex_id = "a".repeat(64);
    session.note_source(&hex_id, "alpha")?;
    if session.note_commission(&hex_id, &hex_id) != Err(SeamFault::MissingObject) {
        return Err(SeamFault::MissingObject);
    }
    session.note_commission("commission/a", &hex_id)?;
    session.select("candidate/1")?;
    session.grant_begin();
    session.execute()?;
    let reopened = Session::reopen(&session.export())?;
    if reopened.sources.len() == 1 && reopened.landing_digest() == reopened.landing {
        Ok(())
    } else {
        Err(SeamFault::MissingObject)
    }
}

/// The fifth source is past the bound. The count is the sources that fit.
pub fn over_bound() -> Result<usize, SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    for name in ["source/a", "source/b", "source/c", "source/d"] {
        session.note_source(name, name)?;
    }
    let accepted = session.sources.len();
    match session.note_source("source/e", "past") {
        Err(SeamFault::Bound) => Ok(accepted),
        Ok(()) => Err(SeamFault::Bound),
        Err(fault) => Err(fault),
    }
}

/// Refuses a foreign scope used as an affordance, a provider, or a decision.
pub fn scope_confusion() -> Result<(), SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    if session.discover("portability/foreign-project") != Err(SeamFault::MissingObject) {
        return Err(SeamFault::MissingObject);
    }
    if session.touch("portability/foreign-project") != Err(SeamFault::MissingObject) {
        return Err(SeamFault::MissingObject);
    }
    let request = session.ask()?;
    let err = session.reply(
        &request.execution,
        request.revision,
        &request.answer,
        "portability/consumer-seams",
    );
    if err == Err(SeamFault::AdviceInDecision) && session.transitions == 0 {
        Ok(())
    } else {
        Err(SeamFault::AdviceInDecision)
    }
}

/// Discovers a registered affordance and refuses a built-in domain name.
pub fn discover_affordance() -> Result<(), SeamFault> {
    let session = Session::open(PROVIDERS)?;
    if session.discover("affordance/review")? != "review" {
        return Err(SeamFault::MissingObject);
    }
    match session.discover("domain/linux") {
        Err(SeamFault::MissingObject) => Ok(()),
        Ok(_) => Err(SeamFault::LinuxProvider),
        Err(fault) => Err(fault),
    }
}

pub(super) fn run_canonical() -> Result<SeamReport, SeamFault> {
    let (first_page, retained_revision) = snapshot_pages()?;
    let (transitions, stale_effects) = direction_once()?;
    wrong_execution()?;
    changed_answer()?;
    advice_slot()?;
    let (effects, stale_edges, unrelated_proof) = selection_without_begin()?;
    advice_rejected()?;
    reopen_same()?;
    interrupt_once()?;
    revoked_subject()?;
    scope_confusion()?;
    let hot_delta = hot_noop()?;
    discover_affordance()?;
    renote_stales()?;
    reopen_keeps_direction()?;
    hot_needs_begin()?;
    selection_is_distinct()?;
    let source_bound = over_bound()?;
    let providers = providers_used()?;
    Ok(SeamReport {
        first_page,
        retained_revision,
        transitions,
        stale_effects,
        effects,
        hot_delta,
        providers,
        stale_edges,
        unrelated_proof,
        source_bound,
    })
}

fn landed() -> Result<Session, SeamFault> {
    let mut session = Session::open(PROVIDERS)?;
    session.note_source("source/a", "alpha")?;
    session.note_commission("commission/a", "source/a")?;
    session.select("candidate/1")?;
    session.grant_begin();
    session.execute()?;
    Ok(session)
}
