// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Qualified support for the frozen executor.
//!
//! The live checker owner stays installed for the process so a later call can
//! resolve the receipt it issued. The receipt itself is returned to the caller.
//! This module does not retain receipts.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use sim_conformance_core::{
    CheckArgument, CheckInputClosureId, CheckInvocation, CheckScopeId, CheckTemplate,
    CheckedSubjectId, CheckerBinding, CheckerResultId, EnvironmentPolicyId, EvidenceGrade,
    EvidenceProvenanceId, EvidenceSetId, LiveCheckerIssuer, LiveCheckerOwner, LiveCheckerReceipt,
    OutputShapeId, PolicyId, ProofCodeId, WorkingDirectoryPolicyId,
};
use sim_kernel::{Datum, Symbol};
use sim_lib_roadmap_runner::{
    ProcessCancellation, ProofCatalog, ProofDisposition, ProofLeaf, SandboxAttempt,
    SandboxLauncher, SandboxRequest, execute_proof_leaf,
};

use crate::authority::OperationLocalSupportDefinition;
use crate::{PackFailure, PackSubject};

struct Installed {
    owner: LiveCheckerOwner,
    issuer: LiveCheckerIssuer,
    authority: sim_conformance_core::LiveCheckerAuthority,
}

/// Issues a receipt after the support proof passes for this definition.
pub(crate) fn issue(
    subject: &str,
    definition: &OperationLocalSupportDefinition,
) -> Result<LiveCheckerReceipt, PackFailure> {
    let installed = installed()?;
    let expected = rendered(definition);
    let (result, invocation, disposition) = prove(subject, &expected, &expected)?;
    if disposition != ProofDisposition::Passed {
        return Err(crate::executor::unqualified());
    }
    installed.owner.mark_current(&invocation).map_err(fail)?;
    installed
        .issuer
        .issue(
            invocation,
            result,
            EvidenceGrade::Reproducible,
            EvidenceProvenanceId::from_text("provenance/executor-support-proof").map_err(fail)?,
            EvidenceSetId::from_text("support/executor-admission").map_err(fail)?,
        )
        .map_err(fail)
}

/// Re-runs the support proof and requires the installed owner to resolve it.
pub(crate) fn require_current(
    subject: &str,
    definition: &OperationLocalSupportDefinition,
    evidence: &dyn PackSubject,
    receipt: &LiveCheckerReceipt,
) -> Result<(), PackFailure> {
    let installed = installed()?;
    let expected = rendered(definition);
    let actual = evidence_members(evidence, definition);
    let (result, invocation, disposition) = prove(subject, &expected, &actual)?;
    if disposition != ProofDisposition::Passed
        || receipt.receipt().result() != &result
        || receipt.invocation() != &invocation
    {
        return Err(crate::executor::unqualified());
    }
    receipt
        .verify_current(&installed.authority)
        .map(|_| ())
        .map_err(fail)
}

fn installed() -> Result<&'static Installed, PackFailure> {
    static SLOT: OnceLock<Installed> = OnceLock::new();
    if let Some(slot) = SLOT.get() {
        return Ok(slot);
    }
    let built = boot()?;
    if SLOT.set(built).is_err() {
        // Another caller installed the owner. This issuer is dropped unused.
    }
    SLOT.get()
        .ok_or_else(|| fail("checker owner did not install"))
}

fn boot() -> Result<Installed, PackFailure> {
    let (owner, issuer) = LiveCheckerOwner::boot(
        crate::executor::runner_id(),
        support_binding()?,
        PolicyId::from_text("sim-sdk/executor-support-currentness-v1").map_err(fail)?,
    );
    let authority = owner.authority();
    Ok(Installed {
        owner,
        issuer,
        authority,
    })
}

fn prove(
    subject: &str,
    expected_members: &BTreeMap<String, String>,
    actual_members: &BTreeMap<String, String>,
) -> Result<(CheckerResultId, CheckInvocation, ProofDisposition), PackFailure> {
    let observations = crate::source_invoke::admission(subject)?;
    let owner = observations
        .iter()
        .find(|item| item.key == "source.subject")
        .map(|item| item.value.as_str())
        .unwrap_or("");
    if owner != subject {
        return Err(crate::executor::unqualified());
    }
    let expected = image(expected_members, owner);
    let actual = image(actual_members, owner);
    let claims = BTreeSet::from(["support/executor-admission".to_owned()]);
    let catalog = ProofCatalog::new([ProofLeaf::SourceDeckPredicate {
        name: "support/executor".to_owned(),
        actual_deck: actual,
        expected_deck: expected.clone(),
        required_claims: claims.clone(),
        present_claims: claims,
    }])
    .map_err(fail)?;
    let executed = execute_proof_leaf(
        &catalog,
        "support/executor",
        &PureLauncher,
        &ProcessCancellation::default(),
        "executor-support",
    )
    .map_err(fail)?;
    let result = CheckerResultId::from_fields(vec![
        (
            Symbol::qualified("conformance", "image"),
            Datum::String(expected.clone()),
        ),
        (
            Symbol::qualified("conformance", "detail"),
            Datum::String(executed.semantic_detail),
        ),
    ])
    .map_err(fail)?;
    Ok((
        result,
        invocation_for(subject, &expected)?,
        executed.disposition,
    ))
}

fn invocation_for(subject: &str, image: &str) -> Result<CheckInvocation, PackFailure> {
    let spec = crate::find_pack("checker/c-source")
        .ok_or_else(|| fail("checker/c-source is not installed"))?;
    let input = CheckInputClosureId::from_fields(vec![(
        Symbol::qualified("conformance", "support"),
        Datum::String(image.to_owned()),
    )])
    .map_err(fail)?;
    support_binding()?
        .instantiate(
            support_code()?,
            spec.pack_id().map_err(fail)?,
            CheckedSubjectId::from_text(subject).map_err(fail)?,
            CheckScopeId::from_text("source/admission").map_err(fail)?,
            input,
        )
        .map_err(fail)
}

/// Binding for the support proof that `prove` runs.
///
/// The source pack's maintenance binding names a Cargo command. This receipt
/// does not use that command. Its call is `prove`.
fn support_binding() -> Result<CheckerBinding, PackFailure> {
    let spec = crate::find_pack("checker/c-source")
        .ok_or_else(|| fail("checker/c-source is not installed"))?;
    let maintenance = spec.checker_binding().map_err(fail)?;
    let scope = CheckScopeId::from_text("source/admission").map_err(fail)?;
    let template = CheckTemplate::new(
        "sim_conformance_packs::executor_support::prove".to_owned(),
        vec![
            CheckArgument::Literal("support/executor".to_owned()),
            CheckArgument::BindingSlot,
            CheckArgument::SubjectSlot,
            CheckArgument::ScopeSlot,
        ],
        WorkingDirectoryPolicyId::from_text("call/no-working-directory").map_err(fail)?,
        EnvironmentPolicyId::from_text("call/native-rust-v1").map_err(fail)?,
        OutputShapeId::from_text("check/result-v1").map_err(fail)?,
    )
    .map_err(fail)?;
    CheckerBinding::new(
        "checker/executor-support".to_owned(),
        maintenance.owner().clone(),
        "sim_conformance_packs::executor_support::prove".to_owned(),
        vec![spec.pack_id().map_err(fail)?],
        OutputShapeId::from_text("check/receipt-v1").map_err(fail)?,
        maintenance.revocation_source().clone(),
        maintenance.owner_validation().clone(),
        maintenance.owner_docs().clone(),
        BTreeSet::from([scope]),
        template,
    )
    .map_err(fail)
}

fn support_code() -> Result<ProofCodeId, PackFailure> {
    ProofCodeId::from_text(&format!(
        "sim_conformance_packs::executor_support::prove@{};build-inputs-sha256={}",
        env!("CARGO_PKG_VERSION"),
        env!("SIM_CONFORMANCE_PACKS_BUILD_INPUTS_SHA256")
    ))
    .map_err(fail)
}

fn rendered(definition: &OperationLocalSupportDefinition) -> BTreeMap<String, String> {
    definition
        .members()
        .iter()
        .map(|(name, identity)| (name.clone(), crate::executor::render_id(identity)))
        .collect()
}

fn evidence_members(
    evidence: &dyn PackSubject,
    definition: &OperationLocalSupportDefinition,
) -> BTreeMap<String, String> {
    definition
        .members()
        .keys()
        .map(|name| (name.clone(), evidence.fact(name).unwrap_or("").to_owned()))
        .collect()
}

fn image(members: &BTreeMap<String, String>, owner: &str) -> String {
    let mut lines = members
        .iter()
        .map(|(name, value)| format!("{name} {value}"))
        .collect::<Vec<_>>();
    lines.push(format!("source.subject {owner}"));
    lines.join("\n")
}

fn fail(error: impl ToString) -> PackFailure {
    PackFailure {
        code: "unqualified-support",
        detail: error.to_string(),
    }
}

struct PureLauncher;

impl SandboxLauncher for PureLauncher {
    fn id(&self) -> &str {
        "executor-support-pure"
    }

    fn launch(&self, _: &SandboxRequest, _: &ProcessCancellation) -> SandboxAttempt {
        panic!("executor support proof is pure and must not launch")
    }
}
