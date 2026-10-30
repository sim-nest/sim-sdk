//! Live C-BOUNDARY authority for exact projection-admission checks.

use std::{collections::BTreeSet, fmt, sync::Arc};

use sim_conformance_core::{
    CheckInvocation, CheckInvocationId, CheckScopeId, CheckedSubjectId, CheckerBinding,
    CheckerBindingId, CheckerReceipt, CheckerResultId, CheckerRevocationObservation,
    ConformanceError, EvidenceGrade, EvidenceProvenanceId, EvidenceSetId, LiveCheckerAuthority,
    LiveCheckerError, LiveCheckerIssuer, LiveCheckerOwner, LiveCheckerReceipt,
    SupportedCheckInputs, content_id_datum,
};
use sim_kernel::{ContentId, Datum, Symbol};
use sim_lib_hotload::ProjectionBuildReviewInput;
use sim_platform_linux::ExecutingProjectionImage;
use sim_platform_ubuntu_pc::{
    ProjectionConfinementObservation, ProjectionConfinementOwner, ProjectionTerminalObservation,
    UbuntuProjectionGuardedExchange, UbuntuProjectionParentExchange,
    VerifiedProjectionBoundaryRuns,
};
use sim_projection_admission_protocol::ProjectionAdmissionProtocolError;
use sim_run_core::CliError;

use crate::projection_admission_policy_id;
use crate::projection_compiler_review::ProjectionCompilerReviewOwner;
use crate::projection_source_review::ProjectionSourceReviewOwner;
use crate::revocation::projection_spec;
use crate::{
    CheckObservation, OperationLocalEvidence, OperationLocalSupportDefinition, PackFailure,
    PackRequest, PackVerdict, ProjectionCompilerReviewAuthority, ProjectionCompilerReviewError,
    ProjectionSourceReviewAuthority, ProjectionSourceReviewError,
    QualifiedProjectionCompilerReview, QualifiedProjectionSourceReview, packs,
};

#[path = "projection_native_admission.rs"]
mod native;
pub use native::{ProjectionNativeAdmissionAttempt, ProjectionNativeOperationService};
pub use sim_projection_admission_protocol::{
    ActivatedProjectionAdmissionSession, ProjectionChildFactoryObservation,
    ProjectionNativeAdmission, ProjectionNativeAdmissionOffer, ProjectionNativeAdmissionPhase,
};

/// Failure while preparing, issuing, or observing projection support.
#[derive(Debug)]
pub enum ProjectionAdmissionAuthorityError {
    /// The canonical native producer/reader protocol refused its input.
    Protocol(ProjectionAdmissionProtocolError),
    /// A neutral conformance identity was invalid.
    Conformance(ConformanceError),
    /// The live owner refused issuance or currentness.
    Authority(LiveCheckerError),
    /// The C-BOUNDARY checker refused the supplied evidence.
    Refused(PackFailure),
    /// The invocation was not the exact native projection binding.
    Unsupported,
    /// Independently verified producer records do not match this check.
    ProducerMismatch(&'static str),
    /// A concrete retained producer owner was unavailable, stale, or substituted.
    ProducerUnavailable(String),
    /// The SDK-owned source-review lifecycle refused the exact held build.
    SourceReview(ProjectionSourceReviewError),
    /// The SDK-owned compiler-review lifecycle refused the exact held build.
    CompilerReview(ProjectionCompilerReviewError),
    /// The one-shot parent-review session was already reconciled.
    ParentReviewReconciled,
    /// This parent preparation already issued its sole parent-review session.
    ParentReviewAlreadyIssued,
}

impl fmt::Display for ProjectionAdmissionAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => error.fmt(formatter),
            Self::Conformance(error) => error.fmt(formatter),
            Self::Authority(error) => error.fmt(formatter),
            Self::Refused(failure) => write!(formatter, "{}: {}", failure.code, failure.detail),
            Self::Unsupported => formatter.write_str("unsupported projection checker invocation"),
            Self::ProducerMismatch(role) => {
                write!(
                    formatter,
                    "verified projection producer record mismatches `{role}`"
                )
            }
            Self::ProducerUnavailable(detail) => formatter.write_str(detail),
            Self::SourceReview(error) => error.fmt(formatter),
            Self::CompilerReview(error) => error.fmt(formatter),
            Self::ParentReviewReconciled => {
                formatter.write_str("projection parent-review session is already reconciled")
            }
            Self::ParentReviewAlreadyIssued => {
                formatter.write_str("projection parent already issued its review session")
            }
        }
    }
}

impl std::error::Error for ProjectionAdmissionAuthorityError {}

impl From<ProjectionAdmissionProtocolError> for ProjectionAdmissionAuthorityError {
    fn from(error: ProjectionAdmissionProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<ConformanceError> for ProjectionAdmissionAuthorityError {
    fn from(error: ConformanceError) -> Self {
        Self::Conformance(error)
    }
}

impl From<LiveCheckerError> for ProjectionAdmissionAuthorityError {
    fn from(error: LiveCheckerError) -> Self {
        Self::Authority(error)
    }
}

impl From<ProjectionSourceReviewError> for ProjectionAdmissionAuthorityError {
    fn from(error: ProjectionSourceReviewError) -> Self {
        Self::SourceReview(error)
    }
}

impl From<ProjectionCompilerReviewError> for ProjectionAdmissionAuthorityError {
    fn from(error: ProjectionCompilerReviewError) -> Self {
        Self::CompilerReview(error)
    }
}

/// Exact retained-A subject that a scoped source-review owner must qualify.
///
/// This is a request, not a review receipt. Its fields are private so copied
/// identities cannot become evidence. [`ProjectionParentPreparation::issue_reviews`]
/// inspects the real build under its live guard and binds an independently
/// owned receipt to these exact values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionSourceReviewPrerequisite {
    build_receipt: ContentId,
    input_closure: ContentId,
    review_subject: ContentId,
}

impl ProjectionSourceReviewPrerequisite {
    /// Returns the exact independently issued A build receipt to review.
    #[must_use]
    pub const fn build_receipt(&self) -> &ContentId {
        &self.build_receipt
    }

    /// Returns the exact complete A input closure to review.
    #[must_use]
    pub const fn input_closure(&self) -> &ContentId {
        &self.input_closure
    }

    /// Returns the owner-produced readable-input subject to review.
    #[must_use]
    pub const fn review_subject(&self) -> &ContentId {
        &self.review_subject
    }
}

/// Parent-side A/B/confinement/run custody for split native admission.
///
/// This non-cloneable and nonserializable owner consumes the actual B-retained
/// A, Ubuntu confinement owner, and hostile-run corpus. `expected_capture` is a
/// boot-selected routing value only, never actual C or admission evidence.
/// Final code must compare it with actual child C inside the authenticated
/// parent/child currentness exchange.
pub struct ProjectionParentPreparation {
    image: Arc<ExecutingProjectionImage>,
    expected_capture: ContentId,
    confinement: ProjectionConfinementOwner,
    runs: Arc<VerifiedProjectionBoundaryRuns>,
    policies: BTreeSet<ContentId>,
    confinement_policy: ContentId,
    source_review: ProjectionSourceReviewPrerequisite,
    reviews_issued: bool,
}

/// Durable in-memory phase of one SDK-owned parent-review generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionParentReviewSessionPhase {
    /// Both exact review owners and their receipts remain available to admission.
    Active,
    /// Source review is revoked; compiler review remains owned for retry.
    SourceRevoked,
    /// Both exact review invocations were revoked after terminal reconciliation.
    Reconciled,
}

/// SDK-owned source and compiler review authority for one exact parent preparation.
///
/// This value is deliberately non-cloneable and non-serializable. It retains
/// both strong review owners, their weak observation handles, and both exact
/// qualified build reviews. Downstream code can only borrow those inputs while
/// this session remains active; it cannot select either generation or prolong
/// either owner after the session is dropped.
#[derive(Debug)]
pub struct ProjectionParentReviewSession {
    source_owner: ProjectionSourceReviewOwner,
    source_authority: ProjectionSourceReviewAuthority,
    source: QualifiedProjectionSourceReview,
    compiler_owner: ProjectionCompilerReviewOwner,
    compiler_authority: ProjectionCompilerReviewAuthority,
    compiler: QualifiedProjectionCompilerReview,
    terminal_binding: ContentId,
    phase: ProjectionParentReviewSessionPhase,
}

/// Borrowed, non-serializable parent-review inputs for one native attempt.
///
/// Construction is restricted to an active [`ProjectionParentReviewSession`]
/// after exact parent-owner comparisons. Neither qualified review is cloneable,
/// so this value cannot detach review support from either SDK owner.
pub struct ProjectionParentReviewInputs<'a> {
    source: &'a QualifiedProjectionSourceReview,
    source_authority: &'a ProjectionSourceReviewAuthority,
    compiler: &'a QualifiedProjectionCompilerReview,
    compiler_authority: &'a ProjectionCompilerReviewAuthority,
}

impl<'a> ProjectionParentReviewInputs<'a> {
    /// Borrows the exact owner-issued source-review result.
    #[must_use]
    pub const fn qualified(&self) -> &'a QualifiedProjectionSourceReview {
        self.source
    }

    /// Borrows the weak authority paired with the retained strong owner.
    #[must_use]
    pub const fn authority(&self) -> &'a ProjectionSourceReviewAuthority {
        self.source_authority
    }

    /// Borrows the exact owner-issued compiler-semantic review.
    #[must_use]
    pub const fn compiler(&self) -> &'a QualifiedProjectionCompilerReview {
        self.compiler
    }

    /// Borrows the compiler review's weak authority.
    #[must_use]
    pub const fn compiler_authority(&self) -> &'a ProjectionCompilerReviewAuthority {
        self.compiler_authority
    }
}

impl ProjectionParentPreparation {
    /// Takes custody of the real parent-side producer owners.
    ///
    /// # Errors
    ///
    /// Refuses stale A/B custody, empty or mismatched policy selection,
    /// substituted confinement, or a hostile corpus for other program bytes.
    /// Success is a retained preparation, never D or qualification.
    pub fn prepare(
        image: Arc<ExecutingProjectionImage>,
        expected_capture: ContentId,
        confinement: ProjectionConfinementOwner,
        runs: Arc<VerifiedProjectionBoundaryRuns>,
        policies: BTreeSet<ContentId>,
    ) -> Result<Self, ProjectionAdmissionAuthorityError> {
        if policies.is_empty() {
            return Err(unavailable("projection policy selection is empty"));
        }
        image
            .reobserve_current()
            .map_err(|error| unavailable(error.to_string()))?;
        let review_subject = image
            .build_review_input()
            .with_current(|review| review.readable_input_closure().content_id())
            .map_err(|error| unavailable(error.to_string()))?
            .map_err(|error| unavailable(error.to_string()))?;
        if runs.native_program() != image.implementation() {
            return Err(unavailable(
                "hostile-run corpus did not execute the selected projection image",
            ));
        }
        if !confinement.matches_policies(&policies) {
            return Err(unavailable(
                "projection confinement does not retain the selected policy objects",
            ));
        }
        let confinement_id = confinement
            .lease()
            .map_err(|error| unavailable(error.to_string()))?
            .policy_set()
            .clone();
        confinement
            .while_current_image(&image, || ())
            .map_err(|error| unavailable(error.to_string()))?;
        let preparation = Self {
            source_review: ProjectionSourceReviewPrerequisite {
                build_receipt: image.build_receipt().clone(),
                input_closure: image.complete_input_closure().clone(),
                review_subject,
            },
            image,
            expected_capture,
            confinement,
            runs,
            policies,
            confinement_policy: confinement_id,
            reviews_issued: false,
        };
        preparation.while_current(|| ())?;
        Ok(preparation)
    }

    /// Returns A's opaque owner-bound source and toolchain review input.
    #[must_use]
    pub fn build_review_input(&self) -> &ProjectionBuildReviewInput {
        self.image.build_review_input()
    }

    /// Issues source and compiler-review support under this exact held build and platform.
    ///
    /// The caller supplies no generation identity. The SDK derives it inside
    /// the real platform observation from A/B, confinement, policy, and hostile
    /// run owners, then reviews the retained build before releasing the guard.
    /// Success returns the sole strong owner and receipt in one non-cloneable
    /// session; a descriptive identity can never recreate it.
    pub fn issue_reviews(
        &mut self,
    ) -> Result<ProjectionParentReviewSession, ProjectionAdmissionAuthorityError> {
        if self.reviews_issued {
            return Err(ProjectionAdmissionAuthorityError::ParentReviewAlreadyIssued);
        }
        let session = self.while_platform_current(|platform| {
            let source_owner =
                ProjectionSourceReviewOwner::boot(source_review_generation(self, &platform)?)?;
            let source_authority = source_owner.authority();
            let source = source_owner.review_and_issue(self.build_review_input())?;
            source.while_current_for_build(&source_authority, self.build_review_input(), || ())?;

            let compiler_owner =
                ProjectionCompilerReviewOwner::boot(compiler_review_generation(self, &platform)?)?;
            let compiler_authority = compiler_owner.authority();
            let compiler = compiler_owner.review_and_issue(self.build_review_input(), &platform)?;
            compiler.verify_current(&compiler_authority, &platform)?;

            Ok::<_, ProjectionAdmissionAuthorityError>(ProjectionParentReviewSession {
                source_owner,
                source_authority,
                source,
                compiler_owner,
                compiler_authority,
                compiler,
                terminal_binding: self.terminal_binding().clone(),
                phase: ProjectionParentReviewSessionPhase::Active,
            })
        })??;
        self.reviews_issued = true;
        Ok(session)
    }

    /// Returns the exact descriptive source-review request.
    #[must_use]
    pub const fn source_review_prerequisite(&self) -> &ProjectionSourceReviewPrerequisite {
        &self.source_review
    }

    /// Runs an action while every real parent owner remains current.
    pub fn while_current<T>(
        &self,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionAdmissionAuthorityError> {
        self.while_platform_current(|_| action())
    }

    fn while_platform_current<T>(
        &self,
        action: impl FnOnce(ProjectionConfinementObservation<'_>) -> T,
    ) -> Result<T, ProjectionAdmissionAuthorityError> {
        self.require_parent_selection_current()?;
        let output = self
            .runs
            .while_current_observation(self.runs.execution_environment(), |platform| {
                if platform.implementation() != self.image.implementation()
                    || platform.complete_build_inputs() != self.image.complete_input_closure()
                    || platform.build_receipt() != self.image.build_receipt()
                    || platform.runtime_closure() != self.image.runtime_closure()
                    || platform.selected_policies() != &self.policies
                    || platform.bound_policy() != &self.confinement_policy
                    || platform.terminal_binding() != self.terminal_binding()
                {
                    return Err(unavailable(
                        "platform confinement observation differs from retained projection owners",
                    ));
                }
                Ok(action(platform))
            })
            .map_err(|error| unavailable(error.to_string()))??;
        self.require_parent_selection_current()?;
        Ok(output)
    }

    fn require_parent_selection_current(&self) -> Result<(), ProjectionAdmissionAuthorityError> {
        if self.runs.native_program() != self.image.implementation()
            || !self.confinement.matches_policies(&self.policies)
            || self.image.build_receipt() != &self.source_review.build_receipt
            || self.image.complete_input_closure() != &self.source_review.input_closure
        {
            return Err(unavailable("projection parent owner selection changed"));
        }
        let lease = self
            .confinement
            .lease()
            .map_err(|error| unavailable(error.to_string()))?;
        if lease.policy_set() != &self.confinement_policy {
            return Err(unavailable("projection confinement generation changed"));
        }
        let review_subject = self
            .image
            .build_review_input()
            .with_current(|review| review.readable_input_closure().content_id())
            .map_err(|error| unavailable(error.to_string()))?
            .map_err(|error| unavailable(error.to_string()))?;
        if review_subject != self.source_review.review_subject {
            return Err(unavailable("projection source-review subject changed"));
        }
        Ok(())
    }

    /// Runs an action while both independent A reviews and every parent owner
    /// remain current.
    ///
    /// Equal build receipts or closure identities cannot substitute another
    /// retained build allocation: pointer-exact ownership is checked against
    /// the retained build owner before either review's evidence is trusted.
    ///
    /// Enters the retained build's currentness guard EXACTLY ONCE for the
    /// joined source/compiler review interval and shares the resulting borrow
    /// between both reviews (the platform/image observation this method also
    /// performs separately re-enters the same underlying mutex sequentially,
    /// not concurrently with this guard -- each acquisition completes before
    /// the next begins, so this does not self-deadlock; "exactly once" refers
    /// only to the joined-review interval, not the whole method). The two
    /// reviews' own
    /// [`QualifiedProjectionSourceReview::while_current`]/
    /// [`QualifiedProjectionCompilerReview::while_current`] each enter that
    /// same guard independently and must never be nested directly -- the
    /// guard wraps a non-reentrant lock, so nesting two acquisitions on the
    /// same thread deadlocks. This method exists specifically so admission can
    /// join both reviews without ever doing that.
    pub fn while_parent_reviews_current<T>(
        &self,
        source: &QualifiedProjectionSourceReview,
        source_authority: &ProjectionSourceReviewAuthority,
        compiler: &QualifiedProjectionCompilerReview,
        compiler_authority: &ProjectionCompilerReviewAuthority,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionAdmissionAuthorityError> {
        if !source.shares_build_owner(self.build_review_input()) {
            return Err(ProjectionSourceReviewError::Substituted("build owner").into());
        }
        if !compiler.shares_build_owner(self.build_review_input()) {
            return Err(ProjectionCompilerReviewError::Substituted("build owner").into());
        }
        let nested = self.while_platform_current(|platform| {
            self.build_review_input().with_current(|current| {
                source.while_current_with(source_authority, current, || {
                    compiler.while_current_with(compiler_authority, current, &platform, action)
                })
            })
        })?;
        let nested = nested.map_err(|error| unavailable(error.to_string()))?;
        nested.map_err(Into::into)?.map_err(Into::into)
    }

    /// Same single-build-lock discipline as [`Self::while_parent_reviews_current`]
    /// -- enters the retained build's currentness guard exactly once (via the
    /// outer `with_current`) and shares the borrow with both reviews through
    /// [`QualifiedProjectionSourceReview::while_current_with`]/
    /// [`QualifiedProjectionCompilerReview::while_current_with`], never nesting
    /// two independent acquisitions of that same non-reentrant guard.
    pub(crate) fn while_parent_reviews_exchange_then(
        &self,
        source: &QualifiedProjectionSourceReview,
        source_authority: &ProjectionSourceReviewAuthority,
        compiler: &QualifiedProjectionCompilerReview,
        compiler_authority: &ProjectionCompilerReviewAuthority,
        exchange: &UbuntuProjectionParentExchange,
        operation: impl FnOnce(
            &ProjectionConfinementObservation<'_>,
            &UbuntuProjectionGuardedExchange<'_>,
        ) -> Result<(), ProjectionAdmissionAuthorityError>,
        commit: impl FnOnce(
            &UbuntuProjectionGuardedExchange<'_>,
        ) -> Result<(), ProjectionAdmissionAuthorityError>,
    ) -> Result<(), ProjectionAdmissionAuthorityError> {
        self.require_parent_selection_current()?;
        if !source.shares_build_owner(self.build_review_input()) {
            return Err(ProjectionSourceReviewError::Substituted("build owner").into());
        }
        if !compiler.shares_build_owner(self.build_review_input()) {
            return Err(ProjectionCompilerReviewError::Substituted("build owner").into());
        }
        let mut operation = Some(operation);
        let mut commit = Some(commit);
        let mut operation_error = None;
        let mut commit_error = None;
        let nested = self
            .build_review_input()
            .with_current(|current| {
                source.while_current_with(source_authority, current, || {
                    self.runs.while_current_exchange_then_with(
                        self.runs.execution_environment(),
                        current,
                        exchange,
                        &mut |platform, guarded| {
                            let result = compiler.while_current_with(
                                compiler_authority,
                                current,
                                &platform,
                                || {
                                    operation.take().expect("parent operation runs once")(
                                        &platform, guarded,
                                    )
                                },
                            );
                            match result {
                                Ok(Ok(())) => Ok(()),
                                Ok(Err(error)) => {
                                    let message = error.to_string();
                                    operation_error = Some(error);
                                    Err(std::io::Error::other(message))
                                }
                                Err(error) => {
                                    let error = ProjectionAdmissionAuthorityError::from(error);
                                    let message = error.to_string();
                                    operation_error = Some(error);
                                    Err(std::io::Error::other(message))
                                }
                            }
                        },
                        &mut |guarded| match commit
                            .take()
                            .expect("parent commit runs once")(guarded)
                        {
                            Ok(()) => Ok(()),
                            Err(error) => {
                                let message = error.to_string();
                                commit_error = Some(error);
                                Err(std::io::Error::other(message))
                            }
                        },
                    )
                })
            })
            .map_err(|error| unavailable(error.to_string()))?
            .map_err(|error| unavailable(error.to_string()))?;
        if let Some(error) = operation_error.or(commit_error) {
            return Err(error);
        }
        nested.map_err(|error| unavailable(error.to_string()))?;
        self.require_parent_selection_current()
    }

    /// Returns the descriptive child-capture selection for an authenticated offer.
    ///
    /// This value is routing data, not actual C and not qualification evidence.
    #[must_use]
    pub const fn expected_capture(&self) -> &ContentId {
        &self.expected_capture
    }

    /// Returns the policy selection for an authenticated offer.
    ///
    /// The child must compare this set with its visited policy objects.
    #[must_use]
    pub fn policies(&self) -> &BTreeSet<ContentId> {
        &self.policies
    }

    /// Exact platform-issued execution/confinement generation later reissued
    /// only by terminal observation of this same native operation.
    #[must_use]
    pub fn terminal_binding(&self) -> &ContentId {
        self.confinement.terminal_binding()
    }
}

impl ProjectionParentReviewSession {
    /// Returns the current one-shot lifecycle phase.
    #[must_use]
    pub const fn phase(&self) -> ProjectionParentReviewSessionPhase {
        self.phase
    }

    /// Exact platform-issued execution/confinement binding retained here.
    ///
    /// This is correlation data for
    /// [`sim_platform_ubuntu_pc::UbuntuProjectionAdmissionCustody::execution_binding`];
    /// only the nonconstructible terminal observation authorizes reconciliation.
    #[must_use]
    pub const fn execution_binding(&self) -> &ContentId {
        &self.terminal_binding
    }

    /// Borrows the exact inputs accepted by a native-admission attempt.
    ///
    /// This performs a fresh pointer-exact build-owner join and full currentness
    /// check before lending either input. The native attempt independently
    /// repeats its guarded production join when it consumes them.
    pub fn admission_inputs<'a>(
        &'a self,
        parent: &ProjectionParentPreparation,
    ) -> Result<ProjectionParentReviewInputs<'a>, ProjectionAdmissionAuthorityError> {
        self.require_active()?;
        parent.while_parent_reviews_current(
            &self.source,
            &self.source_authority,
            &self.compiler,
            &self.compiler_authority,
            || (),
        )?;
        Ok(ProjectionParentReviewInputs {
            source: &self.source,
            source_authority: &self.source_authority,
            compiler: &self.compiler,
            compiler_authority: &self.compiler_authority,
        })
    }

    /// Revokes this exact review invocation after platform terminal proof.
    ///
    /// The terminal token is nonconstructible outside the Ubuntu platform and
    /// is supplied only by its custody callback after the original process and
    /// finite resource domain are observed terminal. A foreign parent is
    /// refused by allocation identity, not by equal hashes. State changes only
    /// after revocation succeeds, so any error retains the strongest still-owned
    /// review and permits an evidence-preserving retry.
    pub fn reconcile_terminal(
        &mut self,
        parent: &ProjectionParentPreparation,
        terminal: &ProjectionTerminalObservation,
    ) -> Result<(), ProjectionAdmissionAuthorityError> {
        if !self.source.shares_build_owner(parent.build_review_input()) {
            return Err(ProjectionSourceReviewError::Substituted("build owner").into());
        }
        if !self
            .compiler
            .shares_build_owner(parent.build_review_input())
        {
            return Err(ProjectionCompilerReviewError::Substituted("build owner").into());
        }
        if parent.terminal_binding() != &self.terminal_binding
            || terminal.execution_binding() != &self.terminal_binding
        {
            return Err(unavailable(
                "projection terminal observation belongs to another execution",
            ));
        }
        if self.phase == ProjectionParentReviewSessionPhase::Reconciled {
            return Ok(());
        }
        complete_parent_review_reconciliation(
            &mut self.phase,
            || self.source_owner.revoke(&self.source),
            || self.compiler_owner.revoke(&self.compiler),
        )
    }

    fn require_active(&self) -> Result<(), ProjectionAdmissionAuthorityError> {
        require_active_source_review_phase(self.phase)
    }
}

fn require_active_source_review_phase(
    phase: ProjectionParentReviewSessionPhase,
) -> Result<(), ProjectionAdmissionAuthorityError> {
    if phase == ProjectionParentReviewSessionPhase::Active {
        Ok(())
    } else {
        Err(ProjectionAdmissionAuthorityError::ParentReviewReconciled)
    }
}

fn complete_parent_review_reconciliation(
    phase: &mut ProjectionParentReviewSessionPhase,
    revoke_source: impl FnOnce() -> Result<(), ProjectionSourceReviewError>,
    revoke_compiler: impl FnOnce() -> Result<(), ProjectionCompilerReviewError>,
) -> Result<(), ProjectionAdmissionAuthorityError> {
    if *phase == ProjectionParentReviewSessionPhase::Active {
        revoke_source()?;
        *phase = ProjectionParentReviewSessionPhase::SourceRevoked;
    }
    if *phase == ProjectionParentReviewSessionPhase::SourceRevoked {
        revoke_compiler()?;
        *phase = ProjectionParentReviewSessionPhase::Reconciled;
    }
    Ok(())
}

fn source_review_generation(
    parent: &ProjectionParentPreparation,
    platform: &ProjectionConfinementObservation<'_>,
) -> Result<ContentId, ProjectionAdmissionAuthorityError> {
    Datum::Node {
        tag: Symbol::qualified("projection", "source-review-owner-generation-v1"),
        fields: vec![
            (
                Symbol::new("review-subject"),
                content_id_datum(&parent.source_review.review_subject),
            ),
            (
                Symbol::new("implementation"),
                content_id_datum(platform.implementation()),
            ),
            (
                Symbol::new("complete-build-inputs"),
                content_id_datum(platform.complete_build_inputs()),
            ),
            (
                Symbol::new("build-receipt"),
                content_id_datum(platform.build_receipt()),
            ),
            (
                Symbol::new("runtime-closure"),
                content_id_datum(platform.runtime_closure()),
            ),
            (
                Symbol::new("native-confinement"),
                content_id_datum(platform.native_confinement()),
            ),
            (
                Symbol::new("confinement-generation"),
                content_id_datum(platform.generation()),
            ),
            (
                Symbol::new("selected-policy-set"),
                content_id_datum(platform.selected_policy_set()),
            ),
            (
                Symbol::new("bound-policy"),
                content_id_datum(platform.bound_policy()),
            ),
            (
                Symbol::new("boundary-runs"),
                content_id_datum(parent.runs.identity()),
            ),
            (
                Symbol::new("terminal-binding"),
                content_id_datum(platform.terminal_binding()),
            ),
        ],
    }
    .content_id()
    .map_err(|error| unavailable(error.to_string()))
}

fn compiler_review_generation(
    parent: &ProjectionParentPreparation,
    platform: &ProjectionConfinementObservation<'_>,
) -> Result<ContentId, ProjectionAdmissionAuthorityError> {
    Datum::Node {
        tag: Symbol::qualified("projection", "compiler-review-owner-generation-v1"),
        fields: vec![
            (
                Symbol::new("review-subject"),
                content_id_datum(&parent.source_review.review_subject),
            ),
            (
                Symbol::new("implementation"),
                content_id_datum(platform.implementation()),
            ),
            (
                Symbol::new("complete-build-inputs"),
                content_id_datum(platform.complete_build_inputs()),
            ),
            (
                Symbol::new("build-receipt"),
                content_id_datum(platform.build_receipt()),
            ),
            (
                Symbol::new("runtime-closure"),
                content_id_datum(platform.runtime_closure()),
            ),
            (
                Symbol::new("native-confinement"),
                content_id_datum(platform.native_confinement()),
            ),
            (
                Symbol::new("confinement-generation"),
                content_id_datum(platform.generation()),
            ),
            (
                Symbol::new("selected-policy-set"),
                content_id_datum(platform.selected_policy_set()),
            ),
            (
                Symbol::new("bound-policy"),
                content_id_datum(platform.bound_policy()),
            ),
            (
                Symbol::new("boundary-runs"),
                content_id_datum(parent.runs.identity()),
            ),
            (
                Symbol::new("terminal-binding"),
                content_id_datum(platform.terminal_binding()),
            ),
        ],
    }
    .content_id()
    .map_err(|error| unavailable(error.to_string()))
}

#[cfg(test)]
mod source_review_session_tests {
    use super::*;

    #[test]
    fn reconciliation_is_one_shot_and_failure_preserves_active_state() {
        let mut phase = ProjectionParentReviewSessionPhase::Active;
        assert!(
            complete_parent_review_reconciliation(
                &mut phase,
                || Ok(()),
                || Err(ProjectionCompilerReviewError::Authority(
                    LiveCheckerError::Unavailable
                )),
            )
            .is_err()
        );
        assert_eq!(phase, ProjectionParentReviewSessionPhase::SourceRevoked);

        complete_parent_review_reconciliation(
            &mut phase,
            || panic!("source revocation repeated"),
            || Ok(()),
        )
        .unwrap();
        assert_eq!(phase, ProjectionParentReviewSessionPhase::Reconciled);
        assert!(matches!(
            require_active_source_review_phase(phase),
            Err(ProjectionAdmissionAuthorityError::ParentReviewReconciled)
        ));
    }
}

fn canonical_id(tag: &str, identities: &[&ContentId]) -> Result<ContentId, CliError> {
    Datum::Node {
        tag: Symbol::qualified("projection", tag),
        fields: vec![(
            Symbol::new("members"),
            Datum::Vector(
                identities
                    .iter()
                    .map(|identity| content_id_datum(identity))
                    .collect(),
            ),
        )],
    }
    .content_id()
    .map_err(|error| CliError::new(error.to_string()))
}

fn unavailable(detail: impl Into<String>) -> ProjectionAdmissionAuthorityError {
    ProjectionAdmissionAuthorityError::ProducerUnavailable(detail.into())
}

/// Sole strong owner for one neutral C-BOUNDARY checker generation.
///
/// This owner does not retain production A/B/C or platform custody. Its
/// producer-record constructor is test-only; production issuance remains a
/// separate authenticated split-owner integration obligation.
#[derive(Debug)]
pub struct ProjectionAdmissionOwner {
    live: LiveCheckerOwner,
    /// Sole minting handle for this generation. `LiveCheckerOwner::boot`
    /// hands this out exactly once; there is no later method that can
    /// produce a second one, and `LiveCheckerAuthority` (what
    /// [`ProjectionAdmissionOwner::authority`] hands to a consumer) cannot
    /// reach `issue` at all.
    issuer: LiveCheckerIssuer,
}

/// Weak execution/currentness handle for the installed projection checker.
#[derive(Clone, Debug)]
pub struct ProjectionAdmissionAuthority {
    live: LiveCheckerAuthority,
}

/// Opaque, complete producer input for one exact projection check.
///
/// This crate deliberately exposes no constructor. The selected producer
/// integration must first verify the actual source, loaded implementation,
/// retained configuration, confinement and observation records, then add its
/// concrete conversion at this owner boundary. Accepting public semantic ids
/// here would let a caller promote hashes or Boolean fixtures into authority.
/// Until that concrete producer contract is integrated, issuance is therefore
/// typed but fail-closed rather than forgeable.
#[derive(Debug)]
pub struct VerifiedProjectionProducerRecords {
    binding: CheckerBindingId,
    invocation: CheckInvocationId,
    subject: CheckedSubjectId,
    scope: CheckScopeId,
    result: CheckerResultId,
    grade: EvidenceGrade,
    provenance: EvidenceProvenanceId,
    support: EvidenceSetId,
    support_definition: ContentId,
}

impl VerifiedProjectionProducerRecords {
    fn verify(
        &self,
        binding: &CheckerBinding,
        invocation: &CheckInvocation,
        result: &CheckerResultId,
        support_definition: &OperationLocalSupportDefinition,
    ) -> Result<(), ProjectionAdmissionAuthorityError> {
        if &self.binding != binding.id() {
            return Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
                "binding",
            ));
        }
        if &self.invocation != invocation.id() {
            return Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
                "invocation",
            ));
        }
        if &self.subject != invocation.subject() {
            return Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
                "subject",
            ));
        }
        if &self.scope != invocation.scope() {
            return Err(ProjectionAdmissionAuthorityError::ProducerMismatch("scope"));
        }
        if &self.result != result {
            return Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
                "result",
            ));
        }
        if self.grade < EvidenceGrade::Reproducible {
            return Err(ProjectionAdmissionAuthorityError::ProducerMismatch("grade"));
        }
        if self.provenance != projection_provenance_id(invocation, support_definition)? {
            return Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
                "provenance",
            ));
        }
        if &self.support_definition != support_definition.identity() {
            return Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
                "support definition",
            ));
        }
        if self.support != projection_support_id(invocation, support_definition)? {
            return Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
                "support",
            ));
        }
        Ok(())
    }
}

/// Opaque projection result joined to the exact live checker owner.
///
/// Its fields cannot be constructed or replaced by a consumer. The retained
/// weak owner binding makes owner death and foreign generations fail closed.
#[derive(Clone, Debug)]
pub struct QualifiedProjectionAdmissionCheck {
    live: LiveCheckerReceipt,
    observations: Vec<CheckObservation>,
    support_definition: ContentId,
}

#[derive(Clone, Copy)]
pub(crate) struct ProjectionParentCustody<'a> {
    parent: &'a ProjectionParentPreparation,
    source_review: &'a QualifiedProjectionSourceReview,
    source_authority: &'a ProjectionSourceReviewAuthority,
    compiler_review: &'a QualifiedProjectionCompilerReview,
    compiler_authority: &'a ProjectionCompilerReviewAuthority,
}

impl<'a> ProjectionParentCustody<'a> {
    pub(crate) const fn new(
        parent: &'a ProjectionParentPreparation,
        source_review: &'a QualifiedProjectionSourceReview,
        source_authority: &'a ProjectionSourceReviewAuthority,
        compiler_review: &'a QualifiedProjectionCompilerReview,
        compiler_authority: &'a ProjectionCompilerReviewAuthority,
    ) -> Self {
        Self {
            parent,
            source_review,
            source_authority,
            compiler_review,
            compiler_authority,
        }
    }
}

impl ProjectionAdmissionOwner {
    /// Boots one exact installed C-BOUNDARY generation inside the SDK owner.
    ///
    /// Production callers cannot choose this generation. The native admission
    /// session derives it from its platform-owned attempt and complete producer
    /// join; crate-local neutral tests use the same constructor directly.
    pub(crate) fn boot(generation: ContentId) -> Result<Self, ProjectionAdmissionAuthorityError> {
        let binding = projection_spec()?.native_projection_binding()?;
        let (live, issuer) =
            LiveCheckerOwner::boot(generation, binding, projection_admission_policy_id()?);
        Ok(Self { live, issuer })
    }

    /// Issues a weak handle that cannot retain the owner.
    #[must_use]
    pub fn authority(&self) -> ProjectionAdmissionAuthority {
        ProjectionAdmissionAuthority {
            live: self.live.authority(),
        }
    }

    /// Marks the exact prepared invocation current.
    pub fn mark_current(
        &self,
        invocation: &CheckInvocation,
    ) -> Result<(), ProjectionAdmissionAuthorityError> {
        self.live.mark_current(invocation)?;
        Ok(())
    }

    /// Revokes the exact prepared invocation without affecting another subject.
    pub fn revoke(
        &self,
        invocation: &CheckInvocation,
    ) -> Result<(), ProjectionAdmissionAuthorityError> {
        self.live.revoke(invocation)?;
        Ok(())
    }

    /// Executes the exact neutral checker and issues an owner-bound receipt.
    ///
    /// `producer` is intentionally opaque. Only this crate's neutral tests can
    /// currently construct it. A production caller cannot substitute asserted
    /// facts or role-shaped content ids; the authenticated parent/child join
    /// remains a separate explicit integration obligation.
    ///
    /// This is an owner method, not an authority one: minting is reachable
    /// only through the sole [`LiveCheckerIssuer`] returned by
    /// [`LiveCheckerOwner::boot`], which a [`ProjectionAdmissionAuthority`]
    /// handed to an admitted consumer cannot reach.
    pub fn check_and_issue(
        &self,
        invocation: CheckInvocation,
        evidence: &OperationLocalEvidence,
        support_definition: &OperationLocalSupportDefinition,
        producer: VerifiedProjectionProducerRecords,
    ) -> Result<QualifiedProjectionAdmissionCheck, ProjectionAdmissionAuthorityError> {
        verify_projection_support(evidence, support_definition)?;
        verify_invocation(&invocation, evidence, support_definition)?;
        let binding = projection_spec()?.native_projection_binding()?;
        let (result, observations) = execute_projection_check(&binding, &invocation, evidence)?;
        producer.verify(&binding, &invocation, &result, support_definition)?;
        let support_definition = producer.support_definition.clone();
        let live = self.issuer.issue(
            invocation,
            result,
            producer.grade,
            producer.provenance,
            producer.support,
        )?;
        Ok(QualifiedProjectionAdmissionCheck {
            live,
            observations,
            support_definition,
        })
    }
}

impl ProjectionAdmissionAuthority {
    /// Confirms that the exact owner generation remains live.
    pub fn ensure_live(&self) -> Result<(), ProjectionAdmissionAuthorityError> {
        self.live.ensure_live()?;
        Ok(())
    }

    /// Prepares an invocation bound to these exact evidence and support bytes.
    pub fn invocation(
        &self,
        evidence: &OperationLocalEvidence,
        support_definition: &OperationLocalSupportDefinition,
    ) -> Result<CheckInvocation, ProjectionAdmissionAuthorityError> {
        self.live.ensure_live()?;
        verify_projection_support(evidence, support_definition)?;
        let spec = projection_spec()?;
        let inputs = projection_check_inputs(evidence, support_definition)?;
        Ok(spec.native_projection_binding()?.instantiate(
            spec.checker_code_id()?,
            spec.pack_id()?,
            inputs.subject().clone(),
            CheckScopeId::from_text("boundary/projection-admission")?,
            inputs.invocation_input().clone(),
        )?)
    }
}

fn execute_projection_check(
    binding: &CheckerBinding,
    invocation: &CheckInvocation,
    evidence: &OperationLocalEvidence,
) -> Result<(CheckerResultId, Vec<CheckObservation>), ProjectionAdmissionAuthorityError> {
    let binding_text = render(binding.id().content_id());
    let subject_text = render(invocation.subject().content_id());
    let request = PackRequest {
        checker: "checker/c-boundary",
        binding: &binding_text,
        subject: &subject_text,
        scope: "boundary/projection-admission",
        evidence,
    };
    match packs::boundary::check(&request) {
        PackVerdict::Pass {
            result,
            observations,
        } => Ok((result, observations)),
        PackVerdict::Refused(failure) => Err(ProjectionAdmissionAuthorityError::Refused(failure)),
        PackVerdict::UnimplementedPack { .. } => {
            Err(ProjectionAdmissionAuthorityError::Unsupported)
        }
    }
}

impl QualifiedProjectionAdmissionCheck {
    /// Returns the exact installed checker binding.
    pub const fn binding(&self) -> &CheckerBinding {
        self.live.binding()
    }

    /// Returns the exact checker invocation.
    pub const fn invocation(&self) -> &CheckInvocation {
        self.live.invocation()
    }

    /// Returns the immutable owner-issued receipt.
    pub const fn receipt(&self) -> &CheckerReceipt {
        self.live.receipt()
    }

    /// Returns the receipt-bound issue-time revocation observation.
    pub const fn issue_observation(&self) -> &CheckerRevocationObservation {
        self.live.issue_observation()
    }

    /// Returns deterministic observations covered by the checker result.
    pub fn observations(&self) -> &[CheckObservation] {
        &self.observations
    }

    /// Returns the exact independently verified support-definition identity.
    pub const fn support_definition(&self) -> &ContentId {
        &self.support_definition
    }

    /// Reobserves currentness from the exact live owner and verifies the receipt.
    pub fn verify_current(
        &self,
        authority: &ProjectionAdmissionAuthority,
    ) -> Result<CheckerRevocationObservation, ProjectionAdmissionAuthorityError> {
        Ok(self.live.verify_current(&authority.live)?)
    }

    /// Runs an action while the owner holds this exact receipt current.
    pub fn while_current<T>(
        &self,
        authority: &ProjectionAdmissionAuthority,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionAdmissionAuthorityError> {
        Ok(self.live.while_current(&authority.live, action)?)
    }

    /// Runs a parent-side native continuation while D, A/B, both reviews,
    /// hostile runs, and confinement all remain current.
    ///
    /// The canonical child protocol places its callback inside the retained
    /// actual-C guard as well. This method deliberately cannot accept a
    /// serialized child-current identity as a substitute for that per-operation
    /// remote guard.
    pub fn while_parent_production_current<T>(
        &self,
        authority: &ProjectionAdmissionAuthority,
        parent: &ProjectionParentPreparation,
        source_review: &QualifiedProjectionSourceReview,
        source_authority: &ProjectionSourceReviewAuthority,
        compiler_review: &QualifiedProjectionCompilerReview,
        compiler_authority: &ProjectionCompilerReviewAuthority,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionAdmissionAuthorityError> {
        self.while_current(authority, || {
            parent.while_parent_reviews_current(
                source_review,
                source_authority,
                compiler_review,
                compiler_authority,
                action,
            )
        })?
    }

    pub(crate) fn while_parent_exchange_then(
        &self,
        authority: &ProjectionAdmissionAuthority,
        custody: ProjectionParentCustody<'_>,
        exchange: &UbuntuProjectionParentExchange,
        operation: impl FnOnce(
            &ProjectionConfinementObservation<'_>,
            &UbuntuProjectionGuardedExchange<'_>,
        ) -> Result<(), ProjectionAdmissionAuthorityError>,
        commit: impl FnOnce(
            &UbuntuProjectionGuardedExchange<'_>,
        ) -> Result<(), ProjectionAdmissionAuthorityError>,
    ) -> Result<(), ProjectionAdmissionAuthorityError> {
        self.while_current(authority, || {
            custody.parent.while_parent_reviews_exchange_then(
                custody.source_review,
                custody.source_authority,
                custody.compiler_review,
                custody.compiler_authority,
                exchange,
                operation,
                commit,
            )
        })?
    }
}

fn verify_invocation(
    invocation: &CheckInvocation,
    evidence: &OperationLocalEvidence,
    support_definition: &OperationLocalSupportDefinition,
) -> Result<(), ProjectionAdmissionAuthorityError> {
    verify_projection_support(evidence, support_definition)?;
    let spec = projection_spec()?;
    let binding = spec.native_projection_binding()?;
    let inputs = projection_check_inputs(evidence, support_definition)?;
    if invocation.binding() != binding.id()
        || invocation.checker_code() != &spec.checker_code_id()?
        || invocation.pack() != &spec.pack_id()?
        || invocation.scope() != &CheckScopeId::from_text("boundary/projection-admission")?
        || invocation.subject() != inputs.subject()
        || invocation.input_closure() != inputs.invocation_input()
    {
        return Err(ProjectionAdmissionAuthorityError::Unsupported);
    }
    Ok(())
}

fn verify_projection_support(
    evidence: &OperationLocalEvidence,
    support_definition: &OperationLocalSupportDefinition,
) -> Result<(), ProjectionAdmissionAuthorityError> {
    support_definition
        .verify_identity_evidence(evidence)
        .map_err(|_| ProjectionAdmissionAuthorityError::Unsupported)?;
    let required = crate::packs::boundary::PROJECTION_ADMISSION_FACTS;
    if support_definition.members().len() != required.len()
        || required
            .into_iter()
            .any(|name| !support_definition.members().contains_key(name))
        || support_definition
            .members()
            .values()
            .collect::<BTreeSet<_>>()
            .len()
            != required.len()
    {
        return Err(ProjectionAdmissionAuthorityError::Unsupported);
    }
    Ok(())
}

pub(crate) fn projection_check_inputs(
    evidence: &OperationLocalEvidence,
    support_definition: &OperationLocalSupportDefinition,
) -> Result<SupportedCheckInputs, ConformanceError> {
    SupportedCheckInputs::new(
        evidence.subject_id()?,
        evidence.input_closure_id()?,
        support_definition.identity().clone(),
    )
}

fn projection_support_id(
    invocation: &CheckInvocation,
    support_definition: &OperationLocalSupportDefinition,
) -> Result<EvidenceSetId, ConformanceError> {
    EvidenceSetId::from_fields(vec![
        (
            Symbol::qualified("conformance", "projection-invocation"),
            invocation.id().to_datum(),
        ),
        (
            Symbol::qualified("conformance", "projection-support-definition"),
            content_id_datum(support_definition.identity()),
        ),
        (
            Symbol::qualified("conformance", "projection-support-members"),
            Datum::Vector(
                support_definition
                    .members()
                    .iter()
                    .map(|(name, identity)| Datum::Node {
                        tag: Symbol::qualified("conformance", "projection-support-member-v1"),
                        fields: vec![
                            (
                                Symbol::qualified("conformance", "name"),
                                Datum::String(name.clone()),
                            ),
                            (
                                Symbol::qualified("conformance", "identity"),
                                content_id_datum(identity),
                            ),
                        ],
                    })
                    .collect(),
            ),
        ),
    ])
}

fn projection_provenance_id(
    invocation: &CheckInvocation,
    support_definition: &OperationLocalSupportDefinition,
) -> Result<EvidenceProvenanceId, ConformanceError> {
    EvidenceProvenanceId::from_fields(vec![
        (
            Symbol::qualified("projection", "checker-invocation"),
            invocation.id().to_datum(),
        ),
        (
            Symbol::qualified("projection", "owner-support-definition"),
            content_id_datum(support_definition.identity()),
        ),
    ])
}

fn render(id: &ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

#[cfg(test)]
pub(super) fn verified_producer_records_for_test(
    invocation: &CheckInvocation,
    result: CheckerResultId,
    grade: EvidenceGrade,
    provenance: EvidenceProvenanceId,
    support: EvidenceSetId,
    support_definition: &OperationLocalSupportDefinition,
) -> VerifiedProjectionProducerRecords {
    VerifiedProjectionProducerRecords {
        binding: invocation.binding().clone(),
        invocation: invocation.id().clone(),
        subject: invocation.subject().clone(),
        scope: invocation.scope().clone(),
        result,
        grade,
        provenance,
        support,
        support_definition: support_definition.identity().clone(),
    }
}

#[cfg(test)]
pub(super) fn projection_support_id_for_test(
    invocation: &CheckInvocation,
    support_definition: &OperationLocalSupportDefinition,
) -> EvidenceSetId {
    projection_support_id(invocation, support_definition).unwrap()
}

#[cfg(test)]
pub(super) fn projection_provenance_id_for_test(
    invocation: &CheckInvocation,
    support_definition: &OperationLocalSupportDefinition,
) -> EvidenceProvenanceId {
    projection_provenance_id(invocation, support_definition).unwrap()
}
