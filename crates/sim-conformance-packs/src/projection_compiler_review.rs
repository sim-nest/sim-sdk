// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Neutral owner-issued review of retained compiler-semantic observations.
//!
//! Raw analyzer JSON remains descriptive. This module can issue support only
//! while the exact build, authenticated report channel, and live Ubuntu image
//! observation are held together and the closed v2 classification contract is
//! independently recomputed.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sim_conformance_core::{
    CheckArgument, CheckInputClosureId, CheckInvocation, CheckScopeId, CheckTemplate,
    CheckedSubjectId, CheckerBinding, CheckerReceipt, CheckerResultId,
    CheckerRevocationObservation, CommandId, ConformanceError, ConformancePackId,
    EnvironmentPolicyId, EvidenceGrade, EvidenceProvenanceId, EvidenceSetId, LiveCheckerAuthority,
    LiveCheckerError, LiveCheckerIssuer, LiveCheckerOwner, LiveCheckerReceipt, OutputShapeId,
    OwnerBindingId, PolicyId, ProofCodeId, WorkingDirectoryPolicyId, content_id_datum,
};
use sim_kernel::{ContentId, Datum, Symbol};
use sim_lib_hotload::{
    CurrentProjectionBuildReview, NativeBuildRequest, ProjectionBuildReviewInput,
    ProjectionCompilerSemanticSelection, ProjectionCompilerUnitRoute,
};
use sim_platform_ubuntu_pc::ProjectionConfinementObservation;

use crate::owner_revocation_source;

const SCHEMA: &str = "sim-rustc-projection-semantics-v4";
const TAXONOMY: &str = "sim-rustc-projection-mir-operations-v1";
const CLASSIFICATION_POLICY: &str = "sim-projection-local-slice-policy-v1";
const REVIEW_SCOPE: &str = "boundary/projection-compiler-semantics";
const REVIEW_CHECKER: &str = "checker/projection-compiler-semantics";
const REVIEW_PACK: &str = "pack/projection-compiler-semantics-v1";
const MAX_REPORT_BYTES: usize = 8 * 1024 * 1024;
const MAX_REPORTS: usize = 16_384;
const MAX_EDGES: usize = 131_072;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Disposition {
    Allow,
    RefuseLocalSlice,
}

const POLICY: &[(&str, Disposition)] = &[
    ("mono-item.function", Disposition::Allow),
    ("mono-item.static", Disposition::RefuseLocalSlice),
    ("mono-item.global-assembly", Disposition::RefuseLocalSlice),
    ("instance.mir-body", Disposition::Allow),
    (
        "instance.external-summary-missing",
        Disposition::RefuseLocalSlice,
    ),
    ("instance-kind.item", Disposition::Allow),
    ("instance-kind.intrinsic", Disposition::RefuseLocalSlice),
    ("instance-kind.vtable-shim", Disposition::RefuseLocalSlice),
    ("instance-kind.reify-shim", Disposition::RefuseLocalSlice),
    (
        "instance-kind.function-pointer-shim",
        Disposition::RefuseLocalSlice,
    ),
    ("instance-kind.virtual", Disposition::RefuseLocalSlice),
    (
        "instance-kind.closure-once-shim",
        Disposition::RefuseLocalSlice,
    ),
    (
        "instance-kind.coroutine-closure-shim",
        Disposition::RefuseLocalSlice,
    ),
    (
        "instance-kind.thread-local-shim",
        Disposition::RefuseLocalSlice,
    ),
    (
        "instance-kind.future-drop-poll-shim",
        Disposition::RefuseLocalSlice,
    ),
    ("instance-kind.drop-glue", Disposition::RefuseLocalSlice),
    ("instance-kind.clone-shim", Disposition::RefuseLocalSlice),
    (
        "instance-kind.function-pointer-address-shim",
        Disposition::RefuseLocalSlice,
    ),
    (
        "instance-kind.async-drop-glue-constructor",
        Disposition::RefuseLocalSlice,
    ),
    (
        "instance-kind.async-drop-glue",
        Disposition::RefuseLocalSlice,
    ),
    ("statement.assign", Disposition::Allow),
    ("statement.fake-read", Disposition::Allow),
    ("statement.set-discriminant", Disposition::Allow),
    ("statement.storage-live", Disposition::Allow),
    ("statement.storage-dead", Disposition::Allow),
    ("statement.retag", Disposition::Allow),
    ("statement.place-mention", Disposition::Allow),
    ("statement.ascribe-user-type", Disposition::Allow),
    ("statement.coverage", Disposition::Allow),
    ("statement.intrinsic-assume", Disposition::RefuseLocalSlice),
    (
        "statement.intrinsic-copy-nonoverlapping",
        Disposition::RefuseLocalSlice,
    ),
    ("statement.const-eval-counter", Disposition::Allow),
    (
        "statement.backward-incompatible-drop-hint",
        Disposition::Allow,
    ),
    ("statement.nop", Disposition::Allow),
    ("rvalue.use", Disposition::Allow),
    ("rvalue.repeat", Disposition::Allow),
    ("rvalue.reference", Disposition::Allow),
    (
        "rvalue.thread-local-reference",
        Disposition::RefuseLocalSlice,
    ),
    ("rvalue.raw-pointer", Disposition::RefuseLocalSlice),
    ("rvalue.cast-value", Disposition::Allow),
    ("rvalue.cast-pointer", Disposition::RefuseLocalSlice),
    ("rvalue.cast-transmute", Disposition::RefuseLocalSlice),
    ("rvalue.binary-value", Disposition::Allow),
    ("rvalue.binary-unchecked", Disposition::RefuseLocalSlice),
    (
        "rvalue.binary-pointer-offset",
        Disposition::RefuseLocalSlice,
    ),
    ("rvalue.unary-value", Disposition::Allow),
    (
        "rvalue.unary-pointer-metadata",
        Disposition::RefuseLocalSlice,
    ),
    ("rvalue.discriminant", Disposition::Allow),
    ("rvalue.aggregate", Disposition::Allow),
    ("rvalue.copy-for-deref", Disposition::Allow),
    ("rvalue.unsafe-binder", Disposition::RefuseLocalSlice),
    ("place.inspected", Disposition::Allow),
    (
        "place.raw-pointer-dereference",
        Disposition::RefuseLocalSlice,
    ),
    ("terminator.goto", Disposition::Allow),
    ("terminator.switch-int", Disposition::Allow),
    ("terminator.unwind-resume", Disposition::RefuseLocalSlice),
    ("terminator.unwind-terminate", Disposition::RefuseLocalSlice),
    ("terminator.return", Disposition::Allow),
    ("terminator.unreachable", Disposition::Allow),
    ("terminator.call", Disposition::Allow),
    ("terminator.tail-call", Disposition::Allow),
    ("terminator.drop", Disposition::Allow),
    ("terminator.assert", Disposition::RefuseLocalSlice),
    ("terminator.yield", Disposition::RefuseLocalSlice),
    ("terminator.coroutine-drop", Disposition::RefuseLocalSlice),
    ("terminator.false-edge", Disposition::Allow),
    ("terminator.false-unwind", Disposition::Allow),
    ("terminator.inline-assembly", Disposition::RefuseLocalSlice),
    ("call-target.direct", Disposition::Allow),
    ("call-target.trait", Disposition::Allow),
    ("call-target.vtable", Disposition::RefuseLocalSlice),
    ("call-target.intrinsic", Disposition::RefuseLocalSlice),
    ("call-target.foreign", Disposition::RefuseLocalSlice),
    ("call-target.unsafe", Disposition::RefuseLocalSlice),
    ("call-target.ambient", Disposition::RefuseLocalSlice),
    (
        "call-target.external-summary-missing",
        Disposition::RefuseLocalSlice,
    ),
    (
        "call-target.unresolved-function-pointer-or-dynamic",
        Disposition::RefuseLocalSlice,
    ),
    (
        "call-target.unresolved-instance",
        Disposition::RefuseLocalSlice,
    ),
    (
        "call-target.resolution-failed",
        Disposition::RefuseLocalSlice,
    ),
    ("drop-target.trivial", Disposition::Allow),
    ("drop-target.effectful", Disposition::RefuseLocalSlice),
];

/// Failure while issuing or observing compiler-semantic qualification.
#[derive(Debug)]
pub enum ProjectionCompilerReviewError {
    /// Canonical checker identity construction failed.
    Conformance(ConformanceError),
    /// The neutral checker owner is dead, foreign, stale, or revoked.
    Authority(LiveCheckerError),
    /// The retained build/report owner is unavailable.
    BuildUnavailable(String),
    /// A raw report or closed classification record is malformed or substituted.
    ReportUnavailable(String),
    /// The live executing-image join differs from the reviewed build.
    PlatformMismatch(&'static str),
    /// Exact semantic closure is incomplete under the selected policy.
    SemanticUnavailable(&'static str),
    /// A receipt, build owner, report set, or live platform observation was substituted.
    Substituted(&'static str),
}

impl fmt::Display for ProjectionCompilerReviewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ProjectionCompilerReviewError {}
impl From<ConformanceError> for ProjectionCompilerReviewError {
    fn from(e: ConformanceError) -> Self {
        Self::Conformance(e)
    }
}
impl From<LiveCheckerError> for ProjectionCompilerReviewError {
    fn from(e: LiveCheckerError) -> Self {
        Self::Authority(e)
    }
}

#[derive(Debug)]
pub(crate) struct ProjectionCompilerReviewOwner {
    live: LiveCheckerOwner,
    issuer: LiveCheckerIssuer,
}

/// Weak currentness handle for one exact compiler-semantic checker generation.
#[derive(Clone, Debug)]
pub struct ProjectionCompilerReviewAuthority {
    live: LiveCheckerAuthority,
}

/// Nonserializable qualification of one exact held build/report/platform join.
pub struct QualifiedProjectionCompilerReview {
    live: LiveCheckerReceipt,
    build: ProjectionBuildReviewInput,
    snapshot: ReviewedCompilerSemantics,
}

impl fmt::Debug for QualifiedProjectionCompilerReview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QualifiedProjectionCompilerReview")
            .field("invocation", self.live.invocation().id())
            .field("build_receipt", &self.snapshot.build_receipt)
            .finish_non_exhaustive()
    }
}

impl ProjectionCompilerReviewOwner {
    pub(crate) fn boot(generation: ContentId) -> Result<Self, ProjectionCompilerReviewError> {
        let (live, issuer) = LiveCheckerOwner::boot(generation, checker_binding()?, policy_id()?);
        Ok(Self { live, issuer })
    }

    pub(crate) fn authority(&self) -> ProjectionCompilerReviewAuthority {
        ProjectionCompilerReviewAuthority {
            live: self.live.authority(),
        }
    }

    pub(crate) fn review_and_issue(
        &self,
        build: &ProjectionBuildReviewInput,
        platform: &ProjectionConfinementObservation<'_>,
    ) -> Result<QualifiedProjectionCompilerReview, ProjectionCompilerReviewError> {
        let snapshot = observe_and_review(build, platform)?;
        let binding = checker_binding()?;
        let invocation = invocation(&binding, &snapshot)?;
        self.live.mark_current(&invocation)?;
        let live = self.issuer.issue(
            invocation,
            result_id(&snapshot)?,
            EvidenceGrade::Reproducible,
            provenance_id(&snapshot)?,
            support_id(&snapshot)?,
        )?;
        let qualified = QualifiedProjectionCompilerReview {
            live,
            build: build.clone(),
            snapshot,
        };
        qualified.verify_current(
            &ProjectionCompilerReviewAuthority {
                live: self.live.authority(),
            },
            platform,
        )?;
        Ok(qualified)
    }

    pub(crate) fn revoke(
        &self,
        review: &QualifiedProjectionCompilerReview,
    ) -> Result<(), ProjectionCompilerReviewError> {
        self.live.revoke(review.live.invocation())?;
        Ok(())
    }
}

impl ProjectionCompilerReviewAuthority {
    /// Confirms the exact checker generation remains live.
    pub fn ensure_live(&self) -> Result<(), ProjectionCompilerReviewError> {
        self.live.ensure_live().map_err(Into::into)
    }
}

impl QualifiedProjectionCompilerReview {
    /// Returns whether this qualification retains the exact supplied build owner.
    ///
    /// Equal artifact, receipt, or ledger identities cannot substitute a
    /// separately reconstructed owner handle.
    #[must_use]
    pub(crate) fn shares_build_owner(&self, expected: &ProjectionBuildReviewInput) -> bool {
        self.build.shares_owner_with(expected)
    }

    /// Returns the exact owner-issued neutral receipt.
    pub const fn receipt(&self) -> &CheckerReceipt {
        self.live.receipt()
    }
    /// Returns the exact checker invocation.
    pub const fn invocation(&self) -> &CheckInvocation {
        self.live.invocation()
    }
    /// Returns the receipt-bound issue-time revocation observation.
    pub const fn issue_observation(&self) -> &CheckerRevocationObservation {
        self.live.issue_observation()
    }

    /// Returns the exact reviewed build receipt identity.
    pub const fn build_receipt(&self) -> &ContentId {
        &self.snapshot.build_receipt
    }

    /// Returns the exact reviewed artifact identity.
    pub const fn artifact(&self) -> &ContentId {
        &self.snapshot.artifact
    }

    /// Returns the unique final-link unit selected by the retained Cargo owner.
    pub const fn final_unit(&self) -> &ContentId {
        &self.snapshot.final_unit
    }

    /// Returns the independently recomputed closed-classification identity.
    pub const fn classification(&self) -> &ContentId {
        &self.snapshot.classification
    }

    /// Returns the reviewed subject identity for the parent admission join.
    pub fn subject(&self) -> &CheckedSubjectId {
        self.live.invocation().subject()
    }

    /// Re-enters checker, build/report, and supplied live platform custody.
    pub fn verify_current(
        &self,
        authority: &ProjectionCompilerReviewAuthority,
        platform: &ProjectionConfinementObservation<'_>,
    ) -> Result<(), ProjectionCompilerReviewError> {
        self.live.verify_current(&authority.live)?;
        let current = observe_and_review(&self.build, platform)?;
        if current != self.snapshot
            || invocation(self.live.binding(), &current)? != *self.live.invocation()
        {
            return Err(ProjectionCompilerReviewError::Substituted(
                "compiler semantic evidence",
            ));
        }
        Ok(())
    }

    /// Runs an action while checker, build/report, and platform custody remain current.
    pub fn while_current<T>(
        &self,
        authority: &ProjectionCompilerReviewAuthority,
        expected_build: &ProjectionBuildReviewInput,
        platform: &ProjectionConfinementObservation<'_>,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionCompilerReviewError> {
        if !self.build.shares_owner_with(expected_build) {
            return Err(ProjectionCompilerReviewError::Substituted("build owner"));
        }
        let mut action = Some(action);
        let nested = self.live.while_current(&authority.live, || {
            self.build.with_current(|current| {
                let observed = review_current(current, platform)?;
                if observed != self.snapshot {
                    return Err(ProjectionCompilerReviewError::Substituted(
                        "compiler semantic evidence",
                    ));
                }
                Ok(action.take().expect("compiler review action runs once")())
            })
        })?;
        nested.map_err(|e| ProjectionCompilerReviewError::BuildUnavailable(e.to_string()))?
    }

    /// Runs an action while checker liveness, supplied live platform custody,
    /// and the ALREADY-BORROWED current build review stay current, without
    /// itself entering the retained build's currentness guard.
    ///
    /// Callers that also need [`QualifiedProjectionSourceReview`]'s own
    /// currentness for the SAME retained build must acquire that guard exactly
    /// once (via [`ProjectionBuildReviewInput::with_current`]) and pass the
    /// resulting borrow to both reviews through this method -- entering the
    /// guard a second time on the same thread deadlocks, since it wraps a
    /// non-reentrant lock.
    ///
    /// Deliberately `pub(crate)`: this method trusts that `current` was
    /// observed for the exact same retained build owner this review holds,
    /// and does not itself verify that -- only [`projection_authority`]'s own
    /// callers establish that via `shares_build_owner` before calling this.
    /// A caller outside this crate accepting an arbitrary
    /// `CurrentProjectionBuildReview` here could pass one borrowed from an
    /// unrelated build owner and only fail the descriptive snapshot
    /// comparison, not a real ownership check.
    pub(crate) fn while_current_with<T>(
        &self,
        authority: &ProjectionCompilerReviewAuthority,
        current: &CurrentProjectionBuildReview<'_>,
        platform: &ProjectionConfinementObservation<'_>,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionCompilerReviewError> {
        let mut action = Some(action);
        self.live.while_current(&authority.live, || {
            let observed = review_current(current, platform)?;
            if observed != self.snapshot {
                return Err(ProjectionCompilerReviewError::Substituted(
                    "compiler semantic evidence",
                ));
            }
            Ok(action.take().expect("compiler review action runs once")())
        })?
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReviewedCompilerSemantics {
    selection: ContentId,
    report_channel: ContentId,
    report_set: ContentId,
    classification: ContentId,
    entry_identity: ContentId,
    entry_definition: String,
    final_unit: ContentId,
    artifact: ContentId,
    build_receipt: ContentId,
    input_ledger: ContentId,
    implementation: ContentId,
    complete_build_inputs: ContentId,
    runtime_closure: ContentId,
    platform_generation: ContentId,
    native_confinement: ContentId,
    selected_policy_set: ContentId,
    bound_policy: ContentId,
    selected_policies: ContentId,
    terminal_binding: ContentId,
}

fn observe_and_review(
    build: &ProjectionBuildReviewInput,
    platform: &ProjectionConfinementObservation<'_>,
) -> Result<ReviewedCompilerSemantics, ProjectionCompilerReviewError> {
    build
        .with_current(|current| review_current(current, platform))
        .map_err(|e| ProjectionCompilerReviewError::BuildUnavailable(e.to_string()))?
}

fn review_current(
    current: &CurrentProjectionBuildReview<'_>,
    platform: &ProjectionConfinementObservation<'_>,
) -> Result<ReviewedCompilerSemantics, ProjectionCompilerReviewError> {
    let ledger = current.consumed_build_inputs().map_err(|e| {
        ProjectionCompilerReviewError::BuildUnavailable(format!(
            "consumed-input prerequisite {:?}: {}",
            e.kind(),
            e.detail()
        ))
    })?;
    if platform.implementation() != current.candidate().content.content_id() {
        return Err(ProjectionCompilerReviewError::PlatformMismatch(
            "implementation",
        ));
    }
    if platform.build_receipt() != current.candidate().build_receipt.content_id()
        || platform.build_receipt() != ledger.build_receipt().content_id()
    {
        return Err(ProjectionCompilerReviewError::PlatformMismatch(
            "build receipt",
        ));
    }
    current
        .with_current_bound_compiler_reports(|selection, channel, reports| {
            review_reports(
                selection,
                channel,
                reports,
                current,
                platform,
                ledger.semantic_identity(),
            )
        })
        .map_err(|e| {
            ProjectionCompilerReviewError::BuildUnavailable(format!(
                "compiler report prerequisite {:?}: {}",
                e.kind(),
                e.detail()
            ))
        })?
}

fn review_reports(
    selection: &ProjectionCompilerSemanticSelection,
    channel: &ContentId,
    reports: &[&dyn sim_lib_exec::SandboxCompilerSemanticReport],
    current: &CurrentProjectionBuildReview<'_>,
    platform: &ProjectionConfinementObservation<'_>,
    ledger: &ContentId,
) -> Result<ReviewedCompilerSemantics, ProjectionCompilerReviewError> {
    if selection.unit_schema() != SCHEMA || reports.is_empty() || reports.len() > MAX_REPORTS {
        return Err(ProjectionCompilerReviewError::SemanticUnavailable(
            "exact v3 report set is unavailable",
        ));
    }
    let final_units = selection
        .expected_units()
        .iter()
        .filter(|unit| unit.final_link())
        .collect::<Vec<_>>();
    if final_units.len() != 1 {
        return Err(ProjectionCompilerReviewError::SemanticUnavailable(
            "unique final selected unit is unavailable",
        ));
    }
    let final_unit = route_id(final_units[0])?;
    let mut parsed = Vec::with_capacity(reports.len());
    for report in reports {
        parsed.push(parse_report(*report, selection, current.request())?);
    }
    let report_routes = match_routes(selection, &parsed)?;
    validate_process_set(&parsed)?;
    let entry_report_index = *report_routes
        .get(&selection.entry_unit_index())
        .ok_or_else(|| unavailable("selected entry unit report"))?;
    let entry =
        validate_reachable_closure(selection.entry_definition(), entry_report_index, &parsed)?;
    let entry_identity =
        entry_identity_id(selection.entry_unit(), &parsed[entry_report_index], &entry)?;
    let report_set = report_set_id(&parsed)?;
    let classification = classification_set_id(&parsed)?;
    Ok(ReviewedCompilerSemantics {
        selection: selection
            .canonical_datum()
            .content_id()
            .map_err(|_| ConformanceError::NoncanonicalDatum)?,
        report_channel: channel.clone(),
        report_set,
        classification,
        entry_identity,
        entry_definition: selection.entry_definition().into(),
        final_unit,
        artifact: current.candidate().content.content_id().clone(),
        build_receipt: current.candidate().build_receipt.content_id().clone(),
        input_ledger: ledger.clone(),
        implementation: platform.implementation().clone(),
        complete_build_inputs: platform.complete_build_inputs().clone(),
        runtime_closure: platform.runtime_closure().clone(),
        platform_generation: platform.generation().clone(),
        native_confinement: platform.native_confinement().clone(),
        selected_policy_set: platform.selected_policy_set().clone(),
        bound_policy: platform.bound_policy().clone(),
        selected_policies: Datum::Set(
            platform
                .selected_policies()
                .iter()
                .map(content_id_datum)
                .collect(),
        )
        .content_id()
        .map_err(|_| ConformanceError::NoncanonicalDatum)?,
        terminal_binding: platform.terminal_binding().clone(),
    })
}

#[derive(Clone, Debug)]
struct ParsedReport {
    bytes_sha256: [u8; 32],
    analyzer_sha256: [u8; 32],
    analyzer_pid: u32,
    analyzer_start: u64,
    cargo_pid: u32,
    cargo_start: u64,
    crate_name: String,
    stable_crate_id: String,
    crate_hash: String,
    target: String,
    crate_types: BTreeSet<String>,
    features: BTreeSet<String>,
    source: String,
    entry_matches: Vec<EntryMatch>,
    instances: BTreeMap<String, String>,
    edges: Vec<ParsedEdge>,
    classification: [u8; 32],
}

#[derive(Clone, Debug)]
struct ParsedEdge {
    from: String,
    to: String,
    kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EntryMatch {
    definition: String,
    symbol: String,
}

fn parse_report(
    report: &dyn sim_lib_exec::SandboxCompilerSemanticReport,
    selection: &ProjectionCompilerSemanticSelection,
    request: &NativeBuildRequest,
) -> Result<ParsedReport, ProjectionCompilerReviewError> {
    let bytes = report
        .read()
        .map_err(|e| ProjectionCompilerReviewError::ReportUnavailable(e.reason))?;
    if bytes.is_empty()
        || bytes.len() > MAX_REPORT_BYTES
        || bytes.len() > selection.maximum_report_bytes()
    {
        return report_error("report byte bound");
    }
    if report.analyzer_sha256() != selection.analyzer_sha256()
        || report.analyzer_pid() == 0
        || report.analyzer_start_time_ticks() == 0
        || report.cargo_pid() == 0
        || report.cargo_start_time_ticks() == 0
    {
        return report_error("analyzer or process identity");
    }
    let root: Value = serde_json::from_slice(&bytes).map_err(|_| {
        ProjectionCompilerReviewError::ReportUnavailable("report is not JSON".into())
    })?;
    let root_object = object(&root, "root")?;
    exact_keys(
        root_object,
        &[
            "schema",
            "analyzer",
            "invocation",
            "unit",
            "entry_definition",
            "entry_matches",
            "instances",
            "edges",
            "hazards",
            "statics",
            "global_assembly",
            "operation_classification",
            "completeness",
        ],
        "root",
    )?;
    if string(root_object.get("schema"), "schema")? != SCHEMA {
        return report_error("v4 schema");
    }
    let analyzer = object(
        root_object
            .get("analyzer")
            .ok_or_else(|| unavailable("analyzer"))?,
        "analyzer",
    )?;
    exact_keys(analyzer, &["executable_sha256", "schema"], "analyzer")?;
    if string(analyzer.get("schema"), "analyzer schema")? != SCHEMA
        || string(analyzer.get("executable_sha256"), "analyzer digest")?
            != hex(&selection.analyzer_sha256())
    {
        return report_error("analyzer binding");
    }
    let invocation = object(
        root_object
            .get("invocation")
            .ok_or_else(|| unavailable("invocation"))?,
        "invocation",
    )?;
    exact_keys(
        invocation,
        &[
            "arguments",
            "environment",
            "target",
            "crate_types",
            "toolchain_owner",
        ],
        "invocation",
    )?;
    let arguments = strings(invocation.get("arguments"), "arguments")?;
    validate_report_environment(invocation.get("environment"), request)?;
    if invocation.get("toolchain_owner").and_then(Value::as_str)
        != Some(request.toolchain.content.as_str())
    {
        return report_error("toolchain owner binding");
    }
    let crate_name = unique_flag(&arguments, "--crate-name")?;
    let source = unique_source(&arguments)?;
    let features = feature_flags(&arguments)?;
    let target = string(invocation.get("target"), "target")?.into();
    let crate_types = strings(invocation.get("crate_types"), "crate types")?
        .into_iter()
        .map(|value| normalize_crate_type(&value))
        .collect();
    let unit = object(
        root_object.get("unit").ok_or_else(|| unavailable("unit"))?,
        "unit",
    )?;
    exact_keys(
        unit,
        &["crate_name", "stable_crate_id", "crate_hash"],
        "unit",
    )?;
    let stable_crate_id = string(unit.get("stable_crate_id"), "stable crate")?.to_owned();
    let crate_hash = string(unit.get("crate_hash"), "crate hash")?.to_owned();
    if string(unit.get("crate_name"), "unit crate")? != crate_name
        || stable_crate_id.len() != 16
        || crate_hash.is_empty()
    {
        return report_error("unit identity");
    }
    if string(root_object.get("entry_definition"), "entry definition")?
        != selection.entry_definition()
    {
        return report_error("entry binding");
    }
    if !array(root_object.get("hazards"), "hazards")?.is_empty()
        || !array(root_object.get("statics"), "statics")?.is_empty()
        || !array(root_object.get("global_assembly"), "global assembly")?.is_empty()
    {
        return Err(ProjectionCompilerReviewError::SemanticUnavailable(
            "hazard, static, or assembly remains",
        ));
    }
    let entry_matches = parse_entry_matches(root_object.get("entry_matches"))?;
    let (instances, instance_kinds) = parse_instances(root_object.get("instances"))?;
    let edges = parse_edges(root_object.get("edges"))?;
    let classification = validate_classification(root_object.get("operation_classification"))?;
    validate_completeness(root_object.get("completeness"))?;
    validate_structural_counts(
        &classification.counts,
        instances.len(),
        &instance_kinds,
        &edges,
    )?;
    Ok(ParsedReport {
        bytes_sha256: Sha256::digest(&bytes).into(),
        analyzer_sha256: report.analyzer_sha256(),
        analyzer_pid: report.analyzer_pid(),
        analyzer_start: report.analyzer_start_time_ticks(),
        cargo_pid: report.cargo_pid(),
        cargo_start: report.cargo_start_time_ticks(),
        crate_name,
        stable_crate_id,
        crate_hash,
        target,
        crate_types,
        features,
        source,
        entry_matches,
        instances,
        edges,
        classification: classification.identity,
    })
}

#[derive(Debug)]
struct ValidatedClassification {
    identity: [u8; 32],
    counts: BTreeMap<String, u64>,
}

fn validate_classification(
    value: Option<&Value>,
) -> Result<ValidatedClassification, ProjectionCompilerReviewError> {
    let value = value.ok_or_else(|| unavailable("operation classification"))?;
    let o = object(value, "operation classification")?;
    exact_keys(
        o,
        &[
            "taxonomy",
            "policy",
            "policy_entries",
            "counts",
            "inspected_operations",
            "classified_operations",
            "refused_operations",
            "identity_sha256",
        ],
        "operation classification",
    )?;
    if string(o.get("taxonomy"), "taxonomy")? != TAXONOMY
        || string(o.get("policy"), "policy")? != CLASSIFICATION_POLICY
    {
        return report_error("classification policy identity");
    }
    let entries = array(o.get("policy_entries"), "policy entries")?;
    if entries.len() != POLICY.len() {
        return report_error("classification policy cardinality");
    }
    for (value, (category, disposition)) in entries.iter().zip(POLICY) {
        let entry = object(value, "policy entry")?;
        exact_keys(entry, &["category", "disposition"], "policy entry")?;
        let expected = match disposition {
            Disposition::Allow => "allow",
            Disposition::RefuseLocalSlice => "refuse-local-slice",
        };
        if string(entry.get("category"), "policy category")? != *category
            || string(entry.get("disposition"), "policy disposition")? != expected
        {
            return report_error("classification policy entry");
        }
    }
    let counts = object(
        o.get("counts")
            .ok_or_else(|| unavailable("classification counts"))?,
        "classification counts",
    )?;
    if counts.len() != POLICY.len() {
        return report_error("classification count cardinality");
    }
    let mut inspected = 0_u64;
    let mut refused = 0_u64;
    let mut validated_counts = BTreeMap::new();
    for (category, disposition) in POLICY {
        let count = counts
            .get(*category)
            .and_then(Value::as_u64)
            .ok_or_else(|| unavailable("classification count"))?;
        inspected = inspected
            .checked_add(count)
            .ok_or_else(|| unavailable("classification count overflow"))?;
        if *disposition == Disposition::RefuseLocalSlice {
            refused = refused
                .checked_add(count)
                .ok_or_else(|| unavailable("refused count overflow"))?;
        }
        validated_counts.insert((*category).to_owned(), count);
    }
    if o.get("inspected_operations").and_then(Value::as_u64) != Some(inspected)
        || o.get("classified_operations").and_then(Value::as_u64) != Some(inspected)
        || o.get("refused_operations").and_then(Value::as_u64) != Some(refused)
        || refused != 0
    {
        return Err(ProjectionCompilerReviewError::SemanticUnavailable(
            "classification totals are incomplete or refused",
        ));
    }
    let entries_json = POLICY.iter().map(|(category, disposition)| json!({"category": category, "disposition": match disposition { Disposition::Allow => "allow", Disposition::RefuseLocalSlice => "refuse-local-slice" }})).collect::<Vec<_>>();
    let identity_input = json!({"taxonomy": TAXONOMY, "policy": CLASSIFICATION_POLICY, "policy_entries": entries_json, "counts": counts, "inspected_operations": inspected, "classified_operations": inspected, "refused_operations": refused});
    let digest: [u8; 32] = Sha256::digest(
        serde_json::to_vec(&identity_input)
            .map_err(|_| unavailable("classification identity encoding"))?,
    )
    .into();
    if string(o.get("identity_sha256"), "classification identity")? != hex(&digest) {
        return report_error("classification identity digest");
    }
    Ok(ValidatedClassification {
        identity: digest,
        counts: validated_counts,
    })
}

fn validate_structural_counts(
    counts: &BTreeMap<String, u64>,
    instances: usize,
    instance_kinds: &BTreeMap<String, u64>,
    edges: &[ParsedEdge],
) -> Result<(), ProjectionCompilerReviewError> {
    let instances = u64::try_from(instances).map_err(|_| unavailable("instance count"))?;
    if counts.get("mono-item.function") != Some(&instances)
        || counts.get("instance.mir-body") != Some(&instances)
    {
        return report_error("instance classification accounting");
    }
    let instance_kind_total = POLICY
        .iter()
        .filter(|(category, _)| category.starts_with("instance-kind."))
        .try_fold(0_u64, |sum, (category, _)| {
            sum.checked_add(counts[&(*category).to_owned()])
                .ok_or_else(|| unavailable("instance-kind count overflow"))
        })?;
    if instance_kind_total != instances {
        return report_error("instance-kind classification accounting");
    }
    for (kind, count) in instance_kinds {
        let category = format!("instance-kind.{kind}");
        if counts.get(&category) != Some(count) {
            return report_error("instance kind-to-classification accounting");
        }
    }
    let calls = counts["terminator.call"]
        .checked_add(counts["terminator.tail-call"])
        .ok_or_else(|| unavailable("call count overflow"))?;
    let resolved_calls = counts["call-target.direct"]
        .checked_add(counts["call-target.trait"])
        .ok_or_else(|| unavailable("resolved call count overflow"))?;
    let resolved_edges = edges
        .iter()
        .filter(|edge| {
            matches!(
                edge.kind.as_str(),
                "resolved-direct-call" | "resolved-trait-call"
            )
        })
        .count();
    if calls != resolved_calls || usize::try_from(calls).ok() != Some(resolved_edges) {
        return report_error("call-edge classification accounting");
    }
    Ok(())
}

fn validate_report_environment(
    value: Option<&Value>,
    request: &NativeBuildRequest,
) -> Result<(), ProjectionCompilerReviewError> {
    let actual = object(
        value.ok_or_else(|| unavailable("invocation environment"))?,
        "invocation environment",
    )?;
    if actual.len() > 4_096 {
        return report_error("invocation environment bound");
    }
    let mut expected = BTreeMap::new();
    for (key, value) in &request.toolchain.environment {
        if key.starts_with("CARGO_CFG_")
            || key.starts_with("CARGO_FEATURE_")
            || matches!(
                key.as_str(),
                "CARGO_ENCODED_RUSTFLAGS"
                    | "RUSTFLAGS"
                    | "RUSTC"
                    | "RUSTC_WRAPPER"
                    | "RUSTC_WORKSPACE_WRAPPER"
                    | "CC"
                    | "CXX"
                    | "AR"
                    | "CFLAGS"
                    | "CXXFLAGS"
                    | "LDFLAGS"
                    | "TARGET"
                    | "HOST"
                    | "OPT_LEVEL"
                    | "DEBUG"
                    | "PROFILE"
                    | "SIM_PROJECTION_TOOLCHAIN_ID"
            )
        {
            if expected.insert(key.as_str(), value.as_str()).is_some() {
                return report_error("duplicate selected environment");
            }
        }
    }
    if expected.get("SIM_PROJECTION_TOOLCHAIN_ID") != Some(&request.toolchain.content.as_str())
        || actual.len() != expected.len()
        || actual.iter().any(|(key, value)| match value.as_str() {
            Some(value) => expected.get(key.as_str()) != Some(&value),
            None => true,
        })
    {
        return report_error("invocation environment binding");
    }
    Ok(())
}

fn validate_completeness(value: Option<&Value>) -> Result<(), ProjectionCompilerReviewError> {
    let o = object(
        value.ok_or_else(|| unavailable("completeness"))?,
        "completeness",
    )?;
    exact_keys(
        o,
        &[
            "local_slice_resolved",
            "admission_complete",
            "unresolved",
            "admission_prerequisites",
            "instance_limit",
            "edge_limit",
        ],
        "completeness",
    )?;
    if o.get("local_slice_resolved").and_then(Value::as_bool) != Some(true)
        || o.get("admission_complete").and_then(Value::as_bool) != Some(false)
        || o.get("instance_limit").and_then(Value::as_u64) != Some(16_384)
        || o.get("edge_limit").and_then(Value::as_u64) != Some(131_072)
        || !array(o.get("unresolved"), "unresolved")?.is_empty()
    {
        return Err(ProjectionCompilerReviewError::SemanticUnavailable(
            "local slice is unresolved",
        ));
    }
    let actual = strings(o.get("admission_prerequisites"), "admission prerequisites")?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let expected = [
        "owner-held-report-channel",
        "complete-external-summary-closure",
        "build-script-and-proc-macro-linkage",
        "final-native-link-participants",
        "artifact-build-receipt-and-input-ledger-owner-binding",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if actual != expected {
        return report_error("v3 admission prerequisite contract");
    }
    Ok(())
}

fn parse_entry_matches(
    value: Option<&Value>,
) -> Result<Vec<EntryMatch>, ProjectionCompilerReviewError> {
    array(value, "entry matches")?
        .iter()
        .map(|value| {
            let entry = object(value, "entry match")?;
            exact_keys(entry, &["definition", "symbol"], "entry match")?;
            Ok(EntryMatch {
                definition: string(entry.get("definition"), "entry definition")?.to_owned(),
                symbol: string(entry.get("symbol"), "entry symbol")?.to_owned(),
            })
        })
        .collect()
}

fn parse_instances(
    value: Option<&Value>,
) -> Result<(BTreeMap<String, String>, BTreeMap<String, u64>), ProjectionCompilerReviewError> {
    let mut out = BTreeMap::new();
    let mut kinds = BTreeMap::<String, u64>::new();
    let values = array(value, "instances")?;
    if values.len() > MAX_REPORTS {
        return report_error("instance bound");
    }
    for item in values {
        let o = object(item, "instance")?;
        exact_keys(o, &["symbol", "definition", "kind", "local"], "instance")?;
        let symbol = string(o.get("symbol"), "instance symbol")?.to_owned();
        let definition = string(o.get("definition"), "instance definition")?.to_owned();
        if out.insert(symbol, definition).is_some() {
            return report_error("duplicate instance");
        }
        let kind = string(o.get("kind"), "instance kind")?;
        if !POLICY
            .iter()
            .any(|(category, _)| *category == format!("instance-kind.{kind}"))
        {
            return report_error("instance kind taxonomy");
        }
        let count = kinds.entry(kind.to_owned()).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| unavailable("instance kind count overflow"))?;
        o.get("local")
            .and_then(Value::as_bool)
            .ok_or_else(|| unavailable("instance locality"))?;
    }
    Ok((out, kinds))
}
fn parse_edges(value: Option<&Value>) -> Result<Vec<ParsedEdge>, ProjectionCompilerReviewError> {
    let mut out = Vec::new();
    let values = array(value, "edges")?;
    if values.len() > MAX_EDGES {
        return report_error("edge bound");
    }
    for item in values {
        let o = object(item, "edge")?;
        exact_keys(
            o,
            &[
                "from",
                "to",
                "kind",
                "trait_definition",
                "abi",
                "unsafe_call",
                "unwind",
                "ambient_class",
            ],
            "edge",
        )?;
        let from = string(o.get("from"), "edge source")?.to_owned();
        let to = string(o.get("to"), "edge target")?.to_owned();
        let kind = string(o.get("kind"), "edge kind")?.to_owned();
        if o.get("unsafe_call").and_then(Value::as_bool) != Some(false)
            || o.get("ambient_class") != Some(&Value::Null)
        {
            return Err(ProjectionCompilerReviewError::SemanticUnavailable(
                "unsafe, ambient, or unresolved edge",
            ));
        }
        match kind.as_str() {
            "resolved-direct-call" => {
                if o.get("trait_definition") != Some(&Value::Null)
                    || o.get("abi").and_then(Value::as_str).is_none()
                {
                    return report_error("direct-call edge metadata");
                }
            }
            "resolved-trait-call" => {
                string(o.get("trait_definition"), "trait definition")?;
                string(o.get("abi"), "call ABI")?;
            }
            _ => {
                return Err(ProjectionCompilerReviewError::SemanticUnavailable(
                    "unsupported or refused edge class remains",
                ));
            }
        }
        string(o.get("unwind"), "edge unwind")?;
        out.push(ParsedEdge { from, to, kind });
    }
    Ok(out)
}

fn validate_process_set(reports: &[ParsedReport]) -> Result<(), ProjectionCompilerReviewError> {
    let first = reports.first().ok_or_else(|| unavailable("report set"))?;
    let mut analyzer = BTreeSet::new();
    let mut bytes = BTreeSet::new();
    for r in reports {
        if (r.cargo_pid, r.cargo_start) != (first.cargo_pid, first.cargo_start)
            || !analyzer.insert((r.analyzer_pid, r.analyzer_start))
            || !bytes.insert(r.bytes_sha256)
        {
            return report_error("report ancestry or uniqueness");
        }
    }
    Ok(())
}
fn validate_reachable_closure(
    entry_definition: &str,
    entry_report_index: usize,
    reports: &[ParsedReport],
) -> Result<EntryMatch, ProjectionCompilerReviewError> {
    let mut instances = BTreeMap::new();
    let mut edges = Vec::new();
    let mut matches = Vec::new();
    for (report_index, r) in reports.iter().enumerate() {
        for (symbol, definition) in &r.instances {
            if instances
                .insert(symbol.clone(), definition.clone())
                .is_some()
            {
                return report_error("duplicate cross-unit instance");
            }
        }
        edges.extend(r.edges.iter().cloned());
        matches.extend(
            r.entry_matches
                .iter()
                .cloned()
                .map(|entry| (report_index, entry)),
        );
    }
    if matches.len() != 1
        || matches[0].0 != entry_report_index
        || matches[0].1.definition != entry_definition
        || instances.get(&matches[0].1.symbol) != Some(&matches[0].1.definition)
    {
        return Err(ProjectionCompilerReviewError::SemanticUnavailable(
            "unique reachable entry is unavailable",
        ));
    }
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for edge in edges {
        if !instances.contains_key(&edge.from) || !instances.contains_key(&edge.to) {
            return Err(ProjectionCompilerReviewError::SemanticUnavailable(
                "reachable external summary is unavailable",
            ));
        }
        outgoing.entry(edge.from).or_default().push(edge.to);
    }
    let mut queue = VecDeque::from([matches[0].1.symbol.clone()]);
    let mut reached = BTreeSet::new();
    while let Some(s) = queue.pop_front() {
        if reached.insert(s.clone()) {
            queue.extend(outgoing.remove(&s).unwrap_or_default());
        }
    }
    if outgoing.keys().any(|source| reached.contains(source)) {
        return Err(ProjectionCompilerReviewError::SemanticUnavailable(
            "reachable edge closure is incomplete",
        ));
    }
    Ok(matches.remove(0).1)
}
fn match_routes(
    selection: &ProjectionCompilerSemanticSelection,
    reports: &[ParsedReport],
) -> Result<BTreeMap<usize, usize>, ProjectionCompilerReviewError> {
    let mut used = BTreeSet::new();
    let mut routes = BTreeMap::new();
    for route in selection.expected_units() {
        let matches = reports
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                r.crate_name == route.crate_name()
                    && r.target == route.target_triple()
                    && r.crate_types == *route.crate_types()
                    && r.features == *route.features()
                    && r.source == route.source()
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if matches.len() != 1 || !used.insert(matches[0]) {
            return report_error("report-to-selection routing");
        }
        routes.insert(route.unit_index(), matches[0]);
    }
    if used.len() != reports.len() {
        return report_error("extra report route");
    }
    Ok(routes)
}

fn entry_identity_id(
    route: &ProjectionCompilerUnitRoute,
    report: &ParsedReport,
    entry: &EntryMatch,
) -> Result<ContentId, ProjectionCompilerReviewError> {
    Datum::Node {
        tag: Symbol::qualified("projection", "CompilerEntryIdentityV1"),
        fields: vec![
            (Symbol::new("unit-route"), route.canonical_datum()),
            (
                Symbol::new("stable-crate-id"),
                Datum::String(report.stable_crate_id.clone()),
            ),
            (
                Symbol::new("crate-hash"),
                Datum::String(report.crate_hash.clone()),
            ),
            (
                Symbol::new("definition"),
                Datum::String(entry.definition.clone()),
            ),
            (Symbol::new("symbol"), Datum::String(entry.symbol.clone())),
        ],
    }
    .content_id()
    .map_err(|_| ConformanceError::NoncanonicalDatum.into())
}

fn route_id(
    route: &ProjectionCompilerUnitRoute,
) -> Result<ContentId, ProjectionCompilerReviewError> {
    Ok(route
        .canonical_datum()
        .content_id()
        .map_err(|_| ConformanceError::NoncanonicalDatum)?)
}
fn report_set_id(reports: &[ParsedReport]) -> Result<ContentId, ProjectionCompilerReviewError> {
    let values = reports
        .iter()
        .map(|r| {
            Datum::Vector(vec![
                Datum::Bytes(r.bytes_sha256.to_vec()),
                Datum::Bytes(r.analyzer_sha256.to_vec()),
                Datum::String(r.analyzer_pid.to_string()),
                Datum::String(r.analyzer_start.to_string()),
                Datum::String(r.cargo_pid.to_string()),
                Datum::String(r.cargo_start.to_string()),
            ])
        })
        .collect();
    Ok(Datum::Vector(values)
        .content_id()
        .map_err(|_| ConformanceError::NoncanonicalDatum)?)
}
fn classification_set_id(
    reports: &[ParsedReport],
) -> Result<ContentId, ProjectionCompilerReviewError> {
    Ok(Datum::Vector(
        reports
            .iter()
            .map(|r| Datum::Bytes(r.classification.to_vec()))
            .collect(),
    )
    .content_id()
    .map_err(|_| ConformanceError::NoncanonicalDatum)?)
}

fn checker_binding() -> Result<CheckerBinding, ConformanceError> {
    let scope = CheckScopeId::from_text(REVIEW_SCOPE)?;
    let template = CheckTemplate::new(
        "sim_conformance_packs::ProjectionCompilerReviewOwner::review_and_issue".into(),
        vec![
            CheckArgument::Literal(REVIEW_CHECKER.into()),
            CheckArgument::BindingSlot,
            CheckArgument::SubjectSlot,
            CheckArgument::ScopeSlot,
        ],
        WorkingDirectoryPolicyId::from_text("call/no-working-directory")?,
        EnvironmentPolicyId::from_text("call/retained-build-report-platform-custody-v1")?,
        OutputShapeId::from_text("check/result-v1")?,
    )?;
    CheckerBinding::new(
        REVIEW_CHECKER.into(),
        OwnerBindingId::from_fields(vec![(
            Symbol::qualified("projection", "owner"),
            Datum::String("sim-sdk/sim-conformance-packs".into()),
        )])?,
        "sim_conformance_packs::QualifiedProjectionCompilerReview".into(),
        vec![pack_id()?],
        OutputShapeId::from_text("check/receipt-v1")?,
        owner_revocation_source()?,
        CommandId::from_text("cargo test -p sim-conformance-packs projection_compiler_review")?,
        CommandId::from_text("cargo run -p xtask -- simdoc --check")?,
        BTreeSet::from([scope]),
        template,
    )
}
fn checker_code_id() -> Result<ProofCodeId, ConformanceError> {
    ProofCodeId::from_text(&format!(
        "sim-conformance-packs@{};build-inputs-sha256={}",
        env!("CARGO_PKG_VERSION"),
        env!("SIM_CONFORMANCE_PACKS_BUILD_INPUTS_SHA256")
    ))
}
fn pack_id() -> Result<ConformancePackId, ConformanceError> {
    ConformancePackId::from_text(REVIEW_PACK)
}
fn policy_id() -> Result<PolicyId, ConformanceError> {
    PolicyId::from_fields(vec![
        (
            Symbol::qualified("projection", "policy"),
            Datum::String(CLASSIFICATION_POLICY.into()),
        ),
        (
            Symbol::qualified("projection", "taxonomy"),
            Datum::String(TAXONOMY.into()),
        ),
        (
            Symbol::qualified("projection", "checker-code"),
            checker_code_id()?.to_datum(),
        ),
    ])
}
fn invocation(
    binding: &CheckerBinding,
    s: &ReviewedCompilerSemantics,
) -> Result<CheckInvocation, ConformanceError> {
    binding.instantiate(
        checker_code_id()?,
        pack_id()?,
        subject_id(s)?,
        CheckScopeId::from_text(REVIEW_SCOPE)?,
        input_id(s)?,
    )
}
fn subject_id(s: &ReviewedCompilerSemantics) -> Result<CheckedSubjectId, ConformanceError> {
    CheckedSubjectId::from_fields(vec![
        (
            Symbol::qualified("projection", "artifact"),
            content_id_datum(&s.artifact),
        ),
        (
            Symbol::qualified("projection", "build-receipt"),
            content_id_datum(&s.build_receipt),
        ),
        (
            Symbol::qualified("projection", "selection"),
            content_id_datum(&s.selection),
        ),
    ])
}
fn input_id(s: &ReviewedCompilerSemantics) -> Result<CheckInputClosureId, ConformanceError> {
    CheckInputClosureId::from_fields(snapshot_fields(s))
}
fn result_id(s: &ReviewedCompilerSemantics) -> Result<CheckerResultId, ConformanceError> {
    CheckerResultId::from_fields(vec![
        (
            Symbol::qualified("projection", "verdict"),
            Datum::String("accepted".into()),
        ),
        (
            Symbol::qualified("projection", "subject"),
            subject_id(s)?.to_datum(),
        ),
        (
            Symbol::qualified("projection", "classification"),
            content_id_datum(&s.classification),
        ),
    ])
}
fn provenance_id(s: &ReviewedCompilerSemantics) -> Result<EvidenceProvenanceId, ConformanceError> {
    EvidenceProvenanceId::from_fields(vec![
        (
            Symbol::qualified("projection", "report-channel"),
            content_id_datum(&s.report_channel),
        ),
        (
            Symbol::qualified("projection", "report-set"),
            content_id_datum(&s.report_set),
        ),
        (
            Symbol::qualified("projection", "checker-code"),
            checker_code_id()?.to_datum(),
        ),
    ])
}
fn support_id(s: &ReviewedCompilerSemantics) -> Result<EvidenceSetId, ConformanceError> {
    EvidenceSetId::from_fields(snapshot_fields(s))
}
fn snapshot_fields(s: &ReviewedCompilerSemantics) -> Vec<(Symbol, Datum)> {
    [
        ("selection", &s.selection),
        ("report-channel", &s.report_channel),
        ("report-set", &s.report_set),
        ("classification", &s.classification),
        ("entry-identity", &s.entry_identity),
        ("final-unit", &s.final_unit),
        ("artifact", &s.artifact),
        ("build-receipt", &s.build_receipt),
        ("input-ledger", &s.input_ledger),
        ("implementation", &s.implementation),
        ("complete-build-inputs", &s.complete_build_inputs),
        ("runtime-closure", &s.runtime_closure),
        ("platform-generation", &s.platform_generation),
        ("native-confinement", &s.native_confinement),
        ("selected-policy-set", &s.selected_policy_set),
        ("bound-policy", &s.bound_policy),
        ("selected-policies", &s.selected_policies),
        ("terminal-binding", &s.terminal_binding),
    ]
    .into_iter()
    .map(|(n, id)| (Symbol::qualified("projection", n), content_id_datum(id)))
    .chain(std::iter::once((
        Symbol::qualified("projection", "entry-definition"),
        Datum::String(s.entry_definition.clone()),
    )))
    .collect()
}

fn exact_keys(
    o: &serde_json::Map<String, Value>,
    expected: &[&str],
    label: &str,
) -> Result<(), ProjectionCompilerReviewError> {
    let actual = o.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return report_error(label);
    }
    Ok(())
}
fn object<'a>(
    v: &'a Value,
    label: &str,
) -> Result<&'a serde_json::Map<String, Value>, ProjectionCompilerReviewError> {
    v.as_object().ok_or_else(|| unavailable(label))
}
fn array<'a>(
    v: Option<&'a Value>,
    label: &str,
) -> Result<&'a [Value], ProjectionCompilerReviewError> {
    let a = v
        .and_then(Value::as_array)
        .ok_or_else(|| unavailable(label))?;
    if a.len() > MAX_EDGES {
        return report_error(label);
    }
    Ok(a)
}
fn string<'a>(v: Option<&'a Value>, label: &str) -> Result<&'a str, ProjectionCompilerReviewError> {
    v.and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 16 * 1024)
        .ok_or_else(|| unavailable(label))
}
fn strings(v: Option<&Value>, label: &str) -> Result<Vec<String>, ProjectionCompilerReviewError> {
    array(v, label)?
        .iter()
        .map(|v| string(Some(v), label).map(str::to_owned))
        .collect()
}
fn unique_flag(args: &[String], flag: &str) -> Result<String, ProjectionCompilerReviewError> {
    let values = args
        .windows(2)
        .filter(|w| w[0] == flag)
        .map(|w| w[1].clone())
        .collect::<Vec<_>>();
    if values.len() != 1 {
        return report_error(flag);
    }
    Ok(values[0].clone())
}
fn unique_source(args: &[String]) -> Result<String, ProjectionCompilerReviewError> {
    let values = args
        .iter()
        .filter(|a| {
            (a.starts_with("/source/") || a.starts_with("/toolchain/")) && a.ends_with(".rs")
        })
        .cloned()
        .collect::<Vec<_>>();
    if values.len() != 1 {
        return report_error("source argument");
    }
    Ok(values[0].clone())
}
fn feature_flags(args: &[String]) -> Result<BTreeSet<String>, ProjectionCompilerReviewError> {
    let mut out = BTreeSet::new();
    for w in args.windows(2) {
        if w[0] == "--cfg" {
            if let Some(v) = w[1]
                .strip_prefix("feature=\"")
                .and_then(|v| v.strip_suffix('"'))
            {
                if !out.insert(v.into()) {
                    return report_error("duplicate feature");
                }
            }
        }
    }
    Ok(out)
}

fn normalize_crate_type(value: &str) -> String {
    match value {
        "ProcMacro" => "proc-macro".into(),
        other => other.to_ascii_lowercase(),
    }
}
fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(H[(b >> 4) as usize] as char);
        s.push(H[(b & 15) as usize] as char);
    }
    s
}
fn unavailable(label: &str) -> ProjectionCompilerReviewError {
    ProjectionCompilerReviewError::ReportUnavailable(format!(
        "compiler semantic {label} is missing or malformed"
    ))
}
fn report_error<T>(label: &str) -> Result<T, ProjectionCompilerReviewError> {
    Err(unavailable(label))
}

#[cfg(test)]
#[path = "projection_compiler_review_tests.rs"]
mod tests;
