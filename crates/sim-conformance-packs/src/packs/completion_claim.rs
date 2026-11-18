// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Binds receipts the owner checkers already admitted.
//!
//! A journal receipt stays at scope `completion/replay` with result
//! `completion/claim-17`. A source-identity receipt stays at scope
//! `release/source-identity` and is not claim 21. This module does not
//! parse a claim body, hash caller text, or derive a seal. A process
//! observation records the status the operating system returned. That
//! status is not claim 20.

use std::process::Command;

use sim_conformance_v02::{
    CheckScopeId, CheckedSubjectId, CheckerResultId, EvidenceGrade, RevocationStatus,
};
use sim_lib_roadmap_runner::EvidenceDecision;

use crate::PackFailure;

const REPLAY_SCOPE: &str = "completion/replay";
const REPLAY_RESULT: &str = "completion/claim-17";
const SOURCE_SCOPE: &str = "release/source-identity";
const SOURCE_PREFIX: &str = "source-identity ";
const MAX_OUTPUT: usize = 1_048_576;

/// Journal receipt bound at its own scope.
pub struct BoundJournalClaim {
    /// Receipt `observe_journal_completion` admitted.
    pub observation: super::release::ObservedJournalCompletion,
}

/// Source-identity receipt bound at its own scope.
///
/// The result names git identities from dependency tables. It does not
/// discharge claim 21.
pub struct BoundSourceIdentity {
    /// Receipt `observe_source_identity` admitted.
    pub observation: super::release::ObservedSourceRelease,
}

/// Status captured from one executed process.
///
/// `status` is the code the process returned. The buffers are the bytes
/// the process wrote. This is not a completion claim.
pub struct ExecutedProcess {
    /// Program that was spawned.
    pub program: String,
    /// Exit status. Only zero is returned.
    pub status: i32,
    /// Standard output the process wrote.
    pub stdout: Vec<u8>,
    /// Standard error the process wrote.
    pub stderr: Vec<u8>,
}

/// Binds a journal receipt the release checker already admitted.
///
/// # Errors
///
/// Returns a binding refusal when the receipt does not name its subject,
/// the replay scope, current revocation, and `completion/claim-17`.
pub fn bind_journal_receipt(
    observation: super::release::ObservedJournalCompletion,
) -> Result<BoundJournalClaim, PackFailure> {
    require_replay(&observation)?;
    Ok(BoundJournalClaim { observation })
}

/// Binds a source-identity receipt the release checker already admitted.
///
/// # Errors
///
/// Returns a binding refusal when the receipt is not current
/// source-identity evidence, or when its result names a completion claim.
pub fn bind_source_receipt(
    observation: super::release::ObservedSourceRelease,
) -> Result<BoundSourceIdentity, PackFailure> {
    require_source(&observation)?;
    Ok(BoundSourceIdentity { observation })
}

/// Spawns `program` and returns the status it exited with.
///
/// The command string is the program to run. It is not evidence that an
/// owner validation passed. A non-zero status, a signal, or output past
/// the bound produces no observation.
///
/// # Errors
///
/// Returns `process-unrun` when the program cannot be spawned, and
/// `process-failed` when it exits non-zero.
pub fn execute_process(program: &str, args: &[&str]) -> Result<ExecutedProcess, PackFailure> {
    if program.is_empty() || program.contains('\0') || args.iter().any(|arg| arg.contains('\0')) {
        return Err(fail("process-unrun", "argv"));
    }
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|_| fail("process-unrun", program))?;
    if output.stdout.len() > MAX_OUTPUT || output.stderr.len() > MAX_OUTPUT {
        return Err(fail("process-output", program));
    }
    let status = output
        .status
        .code()
        .ok_or_else(|| fail("process-unrun", "signal"))?;
    if status != 0 {
        return Err(fail("process-failed", &status.to_string()));
    }
    Ok(ExecutedProcess {
        program: program.to_owned(),
        status,
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

fn require_replay(
    observation: &super::release::ObservedJournalCompletion,
) -> Result<(), PackFailure> {
    if observation.result != REPLAY_RESULT || observation.subject.is_empty() {
        return Err(fail("claim-relabelled", REPLAY_RESULT));
    }
    require_scope(&observation.scope, REPLAY_SCOPE)?;
    require_catalog(
        &observation.subject,
        &observation.result,
        &observation.scope,
        &observation.checker,
        &observation.catalog,
        &observation.admitted,
        observation.revocation.clone(),
    )
}

fn require_source(observation: &super::release::ObservedSourceRelease) -> Result<(), PackFailure> {
    if !observation.result.starts_with(SOURCE_PREFIX)
        || observation.result.contains("completion/claim-")
        || observation.subject.is_empty()
    {
        return Err(fail("claim-relabelled", SOURCE_SCOPE));
    }
    require_scope(&observation.scope, SOURCE_SCOPE)?;
    require_catalog(
        &observation.subject,
        &observation.result,
        &observation.scope,
        &observation.checker,
        &observation.catalog,
        &observation.admitted,
        observation.revocation.clone(),
    )
}

fn require_scope(scope: &CheckScopeId, expected: &str) -> Result<(), PackFailure> {
    let parsed = CheckScopeId::from_text(expected)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    if scope != &parsed {
        return Err(fail("scope-mismatch", expected));
    }
    Ok(())
}

fn require_catalog(
    subject: &str,
    result: &str,
    scope: &CheckScopeId,
    checker: &sim_conformance_v02::CheckerReceiptId,
    catalog: &sim_lib_roadmap_runner::ProofCatalog,
    admitted: &sim_lib_roadmap_runner::AdmittedEvidence,
    revocation: RevocationStatus,
) -> Result<(), PackFailure> {
    if revocation != RevocationStatus::Current {
        return Err(fail("revocation", "current"));
    }
    let receipt = catalog
        .checker_receipt(checker, revocation)
        .map_err(|error| fail("evidence-not-verified", &error.to_string()))?;
    let subject_id = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    if receipt.subject() != &subject_id {
        return Err(fail("subject-mismatch", subject));
    }
    let result_id = CheckerResultId::from_text(result)
        .map_err(|error| fail("receipt-refused", &error.to_string()))?;
    if receipt.result() != &result_id {
        return Err(fail("result-mismatch", result));
    }
    if receipt.scope() != scope {
        return Err(fail("scope-mismatch", "receipt"));
    }
    match catalog.decide_evidence(&admitted.key, EvidenceGrade::Release, |asked| {
        if asked == checker {
            RevocationStatus::Current
        } else {
            RevocationStatus::Unknown
        }
    }) {
        EvidenceDecision::Reuse { receipt } if receipt == admitted.id => Ok(()),
        _ => Err(fail("evidence-not-verified", "release-reuse")),
    }
}

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}

#[cfg(test)]
#[path = "completion_claim_tests.rs"]
mod tests;
