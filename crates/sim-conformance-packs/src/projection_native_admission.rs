//! Authenticated split-process production of native projection admission D.
//!
//! Wire transport remains platform-owned. This module owns the admission
//! statements carried over that transport and is the only production path that
//! converts the retained A/B/source/run/confinement records plus the remote
//! current C observation into checker evidence and an owner-issued receipt.

use std::{
    collections::BTreeSet,
    thread,
    time::{Duration, Instant},
};

use sim_conformance_core::EvidenceGrade;
use sim_kernel::ContentId;
use sim_lib_server::{FrameKind, ServerFrame};
use sim_platform_linux::{KernelObjectIdentity, NativeDescriptorMessage};
use sim_platform_ubuntu_pc::{
    ProjectionConfinementObservation, ProjectionTerminalObservation,
    UbuntuProjectionGuardedExchange, UbuntuProjectionParentExchange,
};
use sim_projection_admission_protocol::{
    Activated, ActivationComplete, AdmissionAcknowledged, Admitted, ChildCurrent,
    OperationStatement, ParentOffer, ProjectionNativeAdmissionPhase, kernel_identity_id,
};

use super::{
    ProjectionAdmissionAuthorityError, ProjectionAdmissionOwner, ProjectionParentCustody,
    ProjectionParentPreparation, QualifiedProjectionAdmissionCheck,
    VerifiedProjectionProducerRecords, canonical_id as canonical_content_id,
    execute_projection_check, projection_provenance_id, projection_support_id, unavailable,
};
use crate::{
    OperationLocalSupportDefinition, ProjectionCompilerReviewAuthority,
    ProjectionSourceReviewAuthority, QualifiedProjectionCompilerReview,
    QualifiedProjectionSourceReview,
};

/// Result of serving the next parent-side World operation boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionNativeOperationService {
    /// One exact operation completed its accepted frame.
    Served,
    /// Authenticated peer EOF occurred before a new begin frame.
    PeerClosed,
}

/// Parent-owned one-shot production admission attempt.
///
/// This value exists before message 6 and retains the strongest D state reached
/// by the attempt. It is non-cloneable, has no decoder, and never accepts a
/// checker owner, generation, attempt identity, or semantic identity from its
/// caller. An error never resets it to an earlier phase.
pub struct ProjectionNativeAdmissionAttempt {
    phase: ProjectionNativeAdmissionPhase,
    attempt: Option<ContentId>,
    admission_owner: Option<ProjectionAdmissionOwner>,
    qualified: Option<QualifiedProjectionAdmissionCheck>,
    challenge: Option<ContentId>,
    child_current: Option<ContentId>,
    receipt: Option<ContentId>,
    acknowledgement: Option<ContentId>,
    implementation: Option<ContentId>,
    complete_input_closure: Option<ContentId>,
    terminal_binding: Option<ContentId>,
    expected_capture: Option<ContentId>,
    policies: BTreeSet<ContentId>,
    next_sequence: u64,
}

const POLL_INTERVAL: Duration = Duration::from_millis(2);

enum ParentBegin {
    Message(NativeDescriptorMessage),
    PeerClosed,
}

trait ParentProtocolExchange {
    fn administrative_owner_identity(&self) -> std::io::Result<KernelObjectIdentity>;
    fn send_frame(&self, frame: &ServerFrame) -> std::io::Result<()>;
    fn receive(&self) -> std::io::Result<Option<NativeDescriptorMessage>>;
    fn revalidate(&self) -> std::io::Result<()>;
}

impl ParentProtocolExchange for UbuntuProjectionParentExchange {
    fn administrative_owner_identity(&self) -> std::io::Result<KernelObjectIdentity> {
        self.administrative_owner_identity()
    }

    fn send_frame(&self, frame: &ServerFrame) -> std::io::Result<()> {
        self.send_frame(frame)
    }

    fn receive(&self) -> std::io::Result<Option<NativeDescriptorMessage>> {
        self.receive()
    }

    fn revalidate(&self) -> std::io::Result<()> {
        self.revalidate()
    }
}

impl ParentProtocolExchange for UbuntuProjectionGuardedExchange<'_> {
    fn administrative_owner_identity(&self) -> std::io::Result<KernelObjectIdentity> {
        self.administrative_owner_identity()
    }

    fn send_frame(&self, frame: &ServerFrame) -> std::io::Result<()> {
        self.send_frame(frame)
    }

    fn receive(&self) -> std::io::Result<Option<NativeDescriptorMessage>> {
        self.receive()
    }

    fn revalidate(&self) -> std::io::Result<()> {
        self.revalidate()
    }
}

impl ProjectionNativeAdmissionAttempt {
    /// Allocates an unentered parent attempt before any transport effect.
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase: ProjectionNativeAdmissionPhase::Unentered,
            attempt: None,
            admission_owner: None,
            qualified: None,
            challenge: None,
            child_current: None,
            receipt: None,
            acknowledgement: None,
            implementation: None,
            complete_input_closure: None,
            terminal_binding: None,
            expected_capture: None,
            policies: BTreeSet::new(),
            next_sequence: 0,
        }
    }

    /// Current durable in-memory attempt phase.
    #[must_use]
    pub const fn phase(&self) -> ProjectionNativeAdmissionPhase {
        self.phase
    }

    /// Exact platform-derived attempt identity, once observation began.
    #[must_use]
    pub const fn attempt(&self) -> Option<&ContentId> {
        self.attempt.as_ref()
    }

    /// Exact retained D receipt identity, once issuance succeeded.
    #[must_use]
    pub const fn receipt(&self) -> Option<&ContentId> {
        self.receipt.as_ref()
    }

    /// Retained qualified D, never cloned or projected from an error.
    #[must_use]
    pub const fn qualified(&self) -> Option<&QualifiedProjectionAdmissionCheck> {
        self.qualified.as_ref()
    }

    /// Produces D through this sole one-shot attempt and enters serving state.
    ///
    /// The checker generation and attempt identity are derived internally from
    /// exact platform and producer owners. Any error after entry leaves this
    /// value in `Refused` or `Uncertain` with its strongest receipt retained.
    pub fn issue_native_admission(
        &mut self,
        parent: &ProjectionParentPreparation,
        source_review: &QualifiedProjectionSourceReview,
        source_authority: &ProjectionSourceReviewAuthority,
        compiler_review: &QualifiedProjectionCompilerReview,
        compiler_authority: &ProjectionCompilerReviewAuthority,
        exchange: &UbuntuProjectionParentExchange,
        deadline: Instant,
    ) -> Result<(), ProjectionAdmissionAuthorityError> {
        self.require_unentered()?;
        let prepared = parent.while_parent_reviews_exchange_then(
            source_review,
            source_authority,
            compiler_review,
            compiler_authority,
            exchange,
            |platform, guarded| {
                verify_platform_selection(parent, source_review, compiler_review, platform)?;
                self.issue_inside_platform_observation(
                    parent,
                    source_review,
                    compiler_review,
                    guarded,
                    deadline,
                    platform,
                )
            },
            |_| Ok(()),
        );
        match prepared {
            Ok(()) => {}
            Err(error) => {
                self.phase = if self.phase == ProjectionNativeAdmissionPhase::Unentered {
                    ProjectionNativeAdmissionPhase::Refused
                } else {
                    ProjectionNativeAdmissionPhase::Uncertain
                };
                return Err(error.into());
            }
        }

        self.phase = ProjectionNativeAdmissionPhase::Activating;
        let owner = self
            .admission_owner
            .as_ref()
            .ok_or_else(|| unavailable("projection D owner was not retained"))?;
        let authority = owner.authority();
        let qualified = self
            .qualified
            .as_ref()
            .ok_or_else(|| unavailable("projection D receipt was not retained"))?;
        let activated = match self.activation_statement() {
            Ok(activated) => activated,
            Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Uncertain;
                return Err(error.into());
            }
        };
        let activation = qualified.while_parent_exchange_then(
            &authority,
            ProjectionParentCustody::new(
                parent,
                source_review,
                source_authority,
                compiler_review,
                compiler_authority,
            ),
            exchange,
            |_, guarded| activate_inside_current_guard(guarded, deadline, &activated),
            |_| Ok(()),
        );
        match activation {
            Ok(()) => {
                self.phase = ProjectionNativeAdmissionPhase::Serving;
                Ok(())
            }
            Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Uncertain;
                Err(error)
            }
        }
    }

    fn require_unentered(&self) -> Result<(), ProjectionAdmissionAuthorityError> {
        if self.phase != ProjectionNativeAdmissionPhase::Unentered {
            return Err(unavailable(
                "projection admission attempt was already entered",
            ));
        }
        Ok(())
    }

    fn activation_statement(&self) -> Result<Activated, ProjectionAdmissionAuthorityError> {
        Ok(Activated::new(
            self.attempt
                .clone()
                .ok_or_else(|| unavailable("projection attempt identity is absent"))?,
            self.challenge
                .clone()
                .ok_or_else(|| unavailable("projection challenge is absent"))?,
            self.child_current
                .clone()
                .ok_or_else(|| unavailable("projection child-current identity is absent"))?,
            self.receipt
                .clone()
                .ok_or_else(|| unavailable("projection receipt identity is absent"))?,
            self.acknowledgement
                .clone()
                .ok_or_else(|| unavailable("projection acknowledgement is absent"))?,
        ))
    }

    fn issue_inside_platform_observation(
        &mut self,
        parent: &ProjectionParentPreparation,
        source_review: &QualifiedProjectionSourceReview,
        compiler_review: &QualifiedProjectionCompilerReview,
        exchange: &impl ParentProtocolExchange,
        deadline: Instant,
        platform: &ProjectionConfinementObservation<'_>,
    ) -> Result<(), ProjectionAdmissionAuthorityError> {
        let administrative_owner = exchange
            .administrative_owner_identity()
            .map_err(native_unavailable)?;
        let attempt = production_attempt_id(
            parent,
            source_review,
            compiler_review,
            administrative_owner,
            platform,
        )?;
        let offer = parent_offer(
            parent,
            source_review,
            compiler_review,
            administrative_owner,
            attempt.clone(),
            platform,
        )?;
        self.attempt = Some(attempt);
        self.challenge = Some(offer.challenge().clone());
        self.implementation = Some(offer.implementation().clone());
        self.complete_input_closure = Some(offer.complete_input_closure().clone());
        self.terminal_binding = Some(platform.terminal_binding().clone());
        self.expected_capture = Some(offer.expected_capture().clone());
        self.policies = offer.policies().clone();
        self.phase = ProjectionNativeAdmissionPhase::Offered;
        if let Err(error) = exchange.send_frame(&offer.frame()?) {
            self.phase = ProjectionNativeAdmissionPhase::Uncertain;
            return Err(native_unavailable(error));
        }

        let message = match receive_parent_until(exchange, deadline) {
            Ok(message) => message,
            Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Uncertain;
                return Err(error.into());
            }
        };
        let child = match ChildCurrent::from_message(message) {
            Ok(child) => child,
            Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Refused;
                return Err(error.into());
            }
        };
        if let Err(error) = verify_child_current(&offer, &child) {
            self.phase = ProjectionNativeAdmissionPhase::Refused;
            return Err(error);
        }
        self.phase = ProjectionNativeAdmissionPhase::ChildCurrent;
        let child_current = child.identity()?;
        let generation = production_d_generation(
            offer.attempt(),
            parent,
            source_review,
            compiler_review,
            platform,
            &child_current,
        )?;
        let owner = ProjectionAdmissionOwner::boot(generation)?;
        let qualified = issue_from_current_records(
            &owner,
            parent,
            source_review,
            compiler_review,
            &offer,
            &child,
            platform,
        )?;
        let receipt = qualified.receipt().id().content_id().clone();
        self.admission_owner = Some(owner);
        self.qualified = Some(qualified);
        self.child_current = Some(child_current.clone());
        self.receipt = Some(receipt.clone());
        self.phase = ProjectionNativeAdmissionPhase::ReceiptPending;
        let admitted = Admitted::new(
            offer.attempt().clone(),
            offer.challenge().clone(),
            child_current,
            receipt,
            offer.implementation().clone(),
            offer.complete_input_closure().clone(),
        );
        if let Err(error) = exchange.send_frame(&admitted.frame()?) {
            self.phase = ProjectionNativeAdmissionPhase::Uncertain;
            return Err(native_unavailable(error));
        }
        let message = match receive_parent_until(exchange, deadline) {
            Ok(message) => message,
            Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Uncertain;
                return Err(error);
            }
        };
        let acknowledged = match AdmissionAcknowledged::from_message(message) {
            Ok(acknowledged) => acknowledged,
            Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Refused;
                return Err(error.into());
            }
        };
        if acknowledged.attempt() != admitted.attempt()
            || acknowledged.challenge() != admitted.challenge()
            || acknowledged.child_current() != admitted.child_current()
            || acknowledged.receipt() != admitted.receipt()
        {
            self.phase = ProjectionNativeAdmissionPhase::Refused;
            return Err(unavailable(
                "projection acknowledgement differs from provisional D",
            ));
        }
        if let Err(error) = exchange.revalidate() {
            self.phase = ProjectionNativeAdmissionPhase::Uncertain;
            return Err(native_unavailable(error));
        }
        self.acknowledgement = Some(acknowledged.identity()?);
        self.phase = ProjectionNativeAdmissionPhase::Acknowledged;
        Ok(())
    }

    /// Serves exactly one sequenced World operation while all parent owners are current.
    pub fn serve_next_world_operation(
        &mut self,
        parent: &ProjectionParentPreparation,
        source_review: &QualifiedProjectionSourceReview,
        source_authority: &ProjectionSourceReviewAuthority,
        compiler_review: &QualifiedProjectionCompilerReview,
        compiler_authority: &ProjectionCompilerReviewAuthority,
        exchange: &UbuntuProjectionParentExchange,
        deadline: Instant,
    ) -> Result<ProjectionNativeOperationService, ProjectionAdmissionAuthorityError> {
        if self.phase != ProjectionNativeAdmissionPhase::Serving {
            return Err(unavailable("projection parent session is not serving"));
        }
        let message = match receive_parent_begin_until(exchange, deadline) {
            Ok(ParentBegin::Message(message)) => message,
            Ok(ParentBegin::PeerClosed) => {
                self.phase = ProjectionNativeAdmissionPhase::TerminalPendingReconciliation;
                return Ok(ProjectionNativeOperationService::PeerClosed);
            }
            Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Uncertain;
                return Err(error);
            }
        };
        let statement = match OperationStatement::from_begin(message, self.next_sequence) {
            Ok(statement) => statement,
            Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Refused;
                return Err(error.into());
            }
        };
        if self.attempt.as_ref() != Some(statement.attempt())
            || self.implementation.as_ref() != Some(statement.implementation())
            || self.complete_input_closure.as_ref() != Some(statement.complete_input_closure())
            || self.receipt.as_ref() != Some(statement.receipt())
            || !self.policies.contains(statement.policy())
        {
            self.phase = ProjectionNativeAdmissionPhase::Refused;
            return Err(unavailable("projection operation differs from admitted D"));
        }
        let owner = self
            .admission_owner
            .as_ref()
            .ok_or_else(|| unavailable("projection D owner was not retained"))?;
        let authority = owner.authority();
        let qualified = self
            .qualified
            .as_ref()
            .ok_or_else(|| unavailable("projection D receipt was not retained"))?;
        let accepted = statement.frame(3, FrameKind::Response, "accepted")?;
        let guarded = qualified.while_parent_exchange_then(
            &authority,
            ProjectionParentCustody::new(
                parent,
                source_review,
                source_authority,
                compiler_review,
                compiler_authority,
            ),
            exchange,
            |_, guarded| -> Result<(), ProjectionAdmissionAuthorityError> {
                guarded
                    .send_frame(&statement.frame(1, FrameKind::Response, "granted")?)
                    .map_err(native_unavailable)?;
                receive_parent_bound_until(
                    guarded,
                    &statement.frame(2, FrameKind::Request, "finish")?,
                    deadline,
                )
            },
            |guarded| guarded.send_frame(&accepted).map_err(native_unavailable),
        );
        match guarded {
            Ok(()) => {}
            Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Uncertain;
                return Err(error);
            }
        }
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| unavailable("projection operation sequence exhausted"))?;
        Ok(ProjectionNativeOperationService::Served)
    }

    /// Revokes retained D only after the caller has reconciled the native terminal.
    pub fn revoke_after_terminal_reconciliation(
        &mut self,
        terminal: &ProjectionTerminalObservation,
    ) -> Result<(), ProjectionAdmissionAuthorityError> {
        if let Some(expected) = &self.terminal_binding
            && terminal.execution_binding() != expected
        {
            return Err(unavailable(
                "projection terminal observation belongs to another admission",
            ));
        }
        reconcile_terminal_phase(&mut self.phase, || {
            if let (Some(owner), Some(qualified)) = (&self.admission_owner, &self.qualified) {
                owner.revoke(qualified.invocation())?;
            }
            Ok(())
        })
    }
}

fn reconcile_terminal_phase(
    phase: &mut ProjectionNativeAdmissionPhase,
    revoke: impl FnOnce() -> Result<(), ProjectionAdmissionAuthorityError>,
) -> Result<(), ProjectionAdmissionAuthorityError> {
    match *phase {
        ProjectionNativeAdmissionPhase::Terminal => Ok(()),
        ProjectionNativeAdmissionPhase::Unentered => {
            *phase = ProjectionNativeAdmissionPhase::Terminal;
            Ok(())
        }
        _ => {
            revoke()?;
            *phase = ProjectionNativeAdmissionPhase::Terminal;
            Ok(())
        }
    }
}

impl Default for ProjectionNativeAdmissionAttempt {
    fn default() -> Self {
        Self::new()
    }
}

fn activate_inside_current_guard(
    exchange: &impl ParentProtocolExchange,
    deadline: Instant,
    activated: &Activated,
) -> Result<(), ProjectionAdmissionAuthorityError> {
    exchange.revalidate().map_err(native_unavailable)?;
    let activation = activated.identity()?;
    exchange
        .send_frame(&activated.frame()?)
        .map_err(native_unavailable)?;
    let complete = ActivationComplete::from_message(receive_parent_until(exchange, deadline)?)?;
    if complete.attempt() != activated.attempt() || complete.activation() != &activation {
        return Err(unavailable(
            "projection activation completion differs from current activation",
        ));
    }
    exchange.revalidate().map_err(native_unavailable)
}

fn verify_child_current(
    offer: &ParentOffer,
    child: &ChildCurrent,
) -> Result<(), ProjectionAdmissionAuthorityError> {
    if child.attempt() != offer.attempt()
        || child.challenge() != offer.challenge()
        || child.implementation() != offer.implementation()
        || child.complete_input_closure() != offer.complete_input_closure()
        || child.capture() != offer.expected_capture()
        || child.policies() != offer.policies()
        || child.verify_composition().is_err()
    {
        return Err(unavailable(
            "authenticated child-current statement differs from parent preparation",
        ));
    }
    Ok(())
}

fn ordered_native_id(
    tag: &str,
    ids: &[ContentId],
) -> Result<ContentId, ProjectionAdmissionAuthorityError> {
    if ids.is_empty() || ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(unavailable(
            "projection child owner membership is empty, duplicated, or unordered",
        ));
    }
    canonical_id(tag, &ids.iter().collect::<Vec<_>>())
}

fn verify_platform_selection(
    parent: &ProjectionParentPreparation,
    source_review: &QualifiedProjectionSourceReview,
    compiler_review: &QualifiedProjectionCompilerReview,
    platform: &ProjectionConfinementObservation<'_>,
) -> Result<(), ProjectionAdmissionAuthorityError> {
    if platform.implementation() != parent.image.implementation()
        || platform.complete_build_inputs() != parent.image.complete_input_closure()
        || platform.build_receipt() != parent.image.build_receipt()
        || platform.build_receipt() != source_review.build_receipt()
        || platform.build_receipt() != compiler_review.build_receipt()
        || platform.implementation() != compiler_review.artifact()
        || platform.runtime_closure() != parent.image.runtime_closure()
        || platform.selected_policies() != &parent.policies
        || platform.bound_policy() != &parent.confinement_policy
        || parent.runs.native_program() != platform.implementation()
    {
        return Err(unavailable(
            "platform confinement observation differs from retained projection owners",
        ));
    }
    Ok(())
}

fn issue_from_current_records(
    owner: &ProjectionAdmissionOwner,
    parent: &ProjectionParentPreparation,
    source_review: &QualifiedProjectionSourceReview,
    compiler_review: &QualifiedProjectionCompilerReview,
    offer: &ParentOffer,
    child: &ChildCurrent,
    platform: &ProjectionConfinementObservation<'_>,
) -> Result<QualifiedProjectionAdmissionCheck, ProjectionAdmissionAuthorityError> {
    let witnesses = producer_witnesses(
        parent,
        source_review,
        compiler_review,
        offer,
        child,
        platform,
    )?;
    let support_definition = OperationLocalSupportDefinition::new(witnesses.iter().cloned())
        .map_err(|error| unavailable(error.to_string()))?;
    let evidence = support_definition
        .identity_evidence()
        .map_err(|error| unavailable(error.to_string()))?;
    support_definition
        .verify_identity_evidence(&evidence)
        .map_err(|error| unavailable(error.to_string()))?;

    let authority = owner.authority();
    let invocation = authority.invocation(&evidence, &support_definition)?;
    let binding = super::projection_spec()?.native_projection_binding()?;
    let (result, _) = execute_projection_check(&binding, &invocation, &evidence)?;
    let provenance = projection_provenance_id(&invocation, &support_definition)?;
    let producer = VerifiedProjectionProducerRecords {
        binding: invocation.binding().clone(),
        invocation: invocation.id().clone(),
        subject: invocation.subject().clone(),
        scope: invocation.scope().clone(),
        result,
        grade: EvidenceGrade::Reproducible,
        support: projection_support_id(&invocation, &support_definition)?,
        provenance,
        support_definition: support_definition.identity().clone(),
    };
    owner.mark_current(&invocation)?;
    owner.check_and_issue(invocation, &evidence, &support_definition, producer)
}

fn parent_offer(
    parent: &ProjectionParentPreparation,
    source_review: &QualifiedProjectionSourceReview,
    compiler_review: &QualifiedProjectionCompilerReview,
    administrative_owner: KernelObjectIdentity,
    attempt: ContentId,
    platform: &ProjectionConfinementObservation<'_>,
) -> Result<ParentOffer, ProjectionAdmissionAuthorityError> {
    let administrative_owner_id = kernel_identity_id(administrative_owner)?;
    let challenge = canonical_id(
        "native-admission-challenge-v1",
        &[
            &attempt,
            &administrative_owner_id,
            platform.implementation(),
            platform.complete_build_inputs(),
            platform.build_receipt(),
            platform.runtime_closure(),
            parent.runs.identity(),
            parent.runs.execution_environment(),
            platform.native_confinement(),
            platform.generation(),
            platform.bound_policy(),
            parent.expected_capture(),
            platform.selected_policy_set(),
            source_review.receipt().id().content_id(),
            compiler_review.receipt().id().content_id(),
        ],
    )?;
    Ok(ParentOffer::new(
        administrative_owner,
        attempt,
        challenge,
        platform.implementation().clone(),
        platform.complete_build_inputs().clone(),
        parent.expected_capture.clone(),
        platform.selected_policies().clone(),
    )?)
}

fn production_attempt_id(
    parent: &ProjectionParentPreparation,
    source_review: &QualifiedProjectionSourceReview,
    compiler_review: &QualifiedProjectionCompilerReview,
    administrative_owner: KernelObjectIdentity,
    platform: &ProjectionConfinementObservation<'_>,
) -> Result<ContentId, ProjectionAdmissionAuthorityError> {
    canonical_id(
        "native-admission-attempt-v1",
        &[
            platform.generation(),
            platform.native_confinement(),
            &kernel_identity_id(administrative_owner)?,
            platform.implementation(),
            platform.complete_build_inputs(),
            platform.build_receipt(),
            platform.runtime_closure(),
            parent.runs.identity(),
            parent.runs.execution_environment(),
            source_review.receipt().id().content_id(),
            compiler_review.receipt().id().content_id(),
        ],
    )
}

fn production_d_generation(
    attempt: &ContentId,
    parent: &ProjectionParentPreparation,
    source_review: &QualifiedProjectionSourceReview,
    compiler_review: &QualifiedProjectionCompilerReview,
    platform: &ProjectionConfinementObservation<'_>,
    child_current: &ContentId,
) -> Result<ContentId, ProjectionAdmissionAuthorityError> {
    canonical_id(
        "native-admission-owner-generation-v1",
        &[
            attempt,
            platform.generation(),
            platform.implementation(),
            platform.complete_build_inputs(),
            platform.build_receipt(),
            platform.runtime_closure(),
            parent.runs.identity(),
            parent.runs.execution_environment(),
            platform.native_confinement(),
            platform.bound_policy(),
            platform.selected_policy_set(),
            source_review.receipt().id().content_id(),
            compiler_review.receipt().id().content_id(),
            child_current,
        ],
    )
}

fn producer_witnesses(
    parent: &ProjectionParentPreparation,
    source_review: &QualifiedProjectionSourceReview,
    compiler_review: &QualifiedProjectionCompilerReview,
    offer: &ParentOffer,
    child: &ChildCurrent,
    platform: &ProjectionConfinementObservation<'_>,
) -> Result<Vec<(String, ContentId)>, ProjectionAdmissionAuthorityError> {
    let source_receipt = source_review.receipt().id().content_id();
    let source_subject = source_review.subject().content_id();
    let compiler_receipt = compiler_review.receipt().id().content_id();
    let compiler_subject = compiler_review.subject().content_id();
    let qualification_policy = super::projection_admission_policy_id()?;
    let compiler_report = compiler_review.classification();
    let facts = [
        (
            "boundary.kernel-unchanged",
            canonical_id(
                "reviewed-kernel-closure-v2",
                &[
                    source_receipt,
                    source_subject,
                    compiler_receipt,
                    compiler_subject,
                    compiler_report,
                ],
            )?,
        ),
        (
            "boundary.projection-registry-open",
            canonical_id(
                "observed-open-registry-v2",
                &[child.capture(), child.composition()],
            )?,
        ),
        (
            "boundary.config-shape-checked",
            canonical_id(
                "observed-configuration-shape-v2",
                &[child.configuration_shape(), child.composition()],
            )?,
        ),
        (
            "boundary.inputs-immutable-and-selected",
            canonical_id(
                "reviewed-selected-inputs-v2",
                &[
                    source_receipt,
                    compiler_receipt,
                    platform.complete_build_inputs(),
                    platform.build_receipt(),
                    compiler_review.final_unit(),
                ],
            )?,
        ),
        (
            "boundary.mediated-reads-exact",
            canonical_id(
                "reviewed-mediated-reads-v2",
                &[
                    source_receipt,
                    compiler_receipt,
                    compiler_report,
                    parent.runs.execution_environment(),
                    platform.native_confinement(),
                    platform.generation(),
                    platform.bound_policy(),
                ],
            )?,
        ),
        (
            "boundary.semantic-envelope-separated",
            canonical_id(
                "reviewed-semantic-envelope-v2",
                &[
                    source_receipt,
                    compiler_receipt,
                    child.configuration_shape(),
                    compiler_report,
                ],
            )?,
        ),
        (
            "boundary.qualification-separate-from-confinement",
            canonical_id(
                "separate-qualification-confinement-v2",
                &[
                    qualification_policy.content_id(),
                    platform.selected_policy_set(),
                    platform.bound_policy(),
                    offer.challenge(),
                ],
            )?,
        ),
        (
            "boundary.native-source-and-dependencies-reviewed",
            canonical_id(
                "native-source-review-v2",
                &[
                    source_receipt,
                    source_review.build_receipt(),
                    source_subject,
                    compiler_receipt,
                    compiler_subject,
                    compiler_review.build_receipt(),
                    compiler_report,
                ],
            )?,
        ),
        (
            "boundary.native-loaded-code-identity-exact",
            canonical_id(
                "native-loaded-code-v2",
                &[
                    platform.implementation(),
                    compiler_review.artifact(),
                    compiler_review.final_unit(),
                    compiler_report,
                    platform.runtime_closure(),
                    parent.runs.identity(),
                ],
            )?,
        ),
        (
            "boundary.baseline-provider-kinds-complete",
            canonical_id(
                "observed-baseline-provider-set-v2",
                &[
                    child.baseline(),
                    &ordered_native_id("providers", child.providers())?,
                ],
            )?,
        ),
        (
            "boundary.federated-owner-closure-sealed",
            canonical_id(
                "observed-federated-owner-closure-v2",
                &[
                    child.closure(),
                    &ordered_native_id("graphs", child.graphs())?,
                    child.composition(),
                    source_receipt,
                ],
            )?,
        ),
    ];
    let mut witnesses = facts
        .into_iter()
        .map(|(name, support)| (name.to_owned(), support))
        .collect::<Vec<_>>();
    witnesses.extend(
        parent
            .runs
            .witnesses()
            .iter()
            .map(|witness| (witness.name().to_owned(), witness.support().clone())),
    );
    witnesses.sort_by(|left, right| left.0.cmp(&right.0));
    if witnesses.len() != 22 || witnesses.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(unavailable(
            "production projection owner did not derive the exact 22-fact support set",
        ));
    }
    Ok(witnesses)
}

fn receive_parent_until(
    exchange: &impl ParentProtocolExchange,
    deadline: Instant,
) -> Result<NativeDescriptorMessage, ProjectionAdmissionAuthorityError> {
    loop {
        if Instant::now() >= deadline {
            return Err(unavailable("projection child-current deadline expired"));
        }
        match exchange.receive().map_err(native_unavailable)? {
            Some(message) => return Ok(message),
            None => thread::sleep(POLL_INTERVAL),
        }
    }
}

fn receive_parent_begin_until(
    exchange: &impl ParentProtocolExchange,
    deadline: Instant,
) -> Result<ParentBegin, ProjectionAdmissionAuthorityError> {
    loop {
        if Instant::now() >= deadline {
            return Err(unavailable("projection operation begin deadline expired"));
        }
        match exchange.receive() {
            Ok(Some(message)) => return Ok(ParentBegin::Message(message)),
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(ParentBegin::PeerClosed);
            }
            Err(error) => return Err(native_unavailable(error)),
        }
    }
}

fn receive_parent_bound_until(
    exchange: &impl ParentProtocolExchange,
    expected: &ServerFrame,
    deadline: Instant,
) -> Result<(), ProjectionAdmissionAuthorityError> {
    receive_parent_until(exchange, deadline)?
        .bind_frame(expected, 0)
        .map(|_| ())
        .map_err(native_unavailable)
}

fn canonical_id(
    tag: &str,
    ids: &[&ContentId],
) -> Result<ContentId, ProjectionAdmissionAuthorityError> {
    canonical_content_id(tag, ids).map_err(|error| unavailable(error.to_string()))
}

fn native_unavailable(error: impl std::fmt::Display) -> ProjectionAdmissionAuthorityError {
    unavailable(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::boundary::PROJECTION_ADMISSION_FACTS;
    use sim_kernel::{Datum, Symbol};

    fn id(value: &str) -> ContentId {
        Datum::String(value.to_owned()).content_id().unwrap()
    }

    fn issued_test_d(
        generation: u8,
    ) -> (
        ProjectionAdmissionOwner,
        super::super::ProjectionAdmissionAuthority,
        QualifiedProjectionAdmissionCheck,
    ) {
        let support = OperationLocalSupportDefinition::new(
            PROJECTION_ADMISSION_FACTS
                .into_iter()
                .map(|name| (name.to_owned(), id(&format!("support/{name}")))),
        )
        .unwrap();
        let evidence = support.identity_evidence().unwrap();
        let owner = ProjectionAdmissionOwner::boot(ContentId::from_bytes(
            Symbol::qualified("projection", "native-terminal-test-generation"),
            [generation; 32],
        ))
        .unwrap();
        let authority = owner.authority();
        let invocation = authority.invocation(&evidence, &support).unwrap();
        let binding = super::super::projection_spec()
            .unwrap()
            .native_projection_binding()
            .unwrap();
        let (result, _) = execute_projection_check(&binding, &invocation, &evidence).unwrap();
        let producer = VerifiedProjectionProducerRecords {
            binding: invocation.binding().clone(),
            invocation: invocation.id().clone(),
            subject: invocation.subject().clone(),
            scope: invocation.scope().clone(),
            result,
            grade: EvidenceGrade::Reproducible,
            provenance: projection_provenance_id(&invocation, &support).unwrap(),
            support: projection_support_id(&invocation, &support).unwrap(),
            support_definition: support.identity().clone(),
        };
        owner.mark_current(&invocation).unwrap();
        let qualified = owner
            .check_and_issue(invocation, &evidence, &support, producer)
            .unwrap();
        (owner, authority, qualified)
    }

    #[test]
    fn new_parent_attempt_is_unentered_and_nonreusable_phase_is_terminal() {
        let mut attempt = ProjectionNativeAdmissionAttempt::new();
        assert_eq!(attempt.phase(), ProjectionNativeAdmissionPhase::Unentered);
        assert!(attempt.require_unentered().is_ok());
        attempt.phase = ProjectionNativeAdmissionPhase::Uncertain;
        assert_ne!(attempt.phase(), ProjectionNativeAdmissionPhase::Unentered);
        assert!(attempt.require_unentered().is_err());
    }

    #[test]
    fn peer_eof_phase_requires_successful_d_revocation_before_terminal() {
        let mut phase = ProjectionNativeAdmissionPhase::TerminalPendingReconciliation;
        let mut calls = 0;
        reconcile_terminal_phase(&mut phase, || {
            calls += 1;
            Err(unavailable("injected D revocation failure"))
        })
        .unwrap_err();
        assert_eq!(calls, 1);
        assert_eq!(
            phase,
            ProjectionNativeAdmissionPhase::TerminalPendingReconciliation
        );
        reconcile_terminal_phase(&mut phase, || {
            calls += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(calls, 2);
        assert_eq!(phase, ProjectionNativeAdmissionPhase::Terminal);
    }

    #[test]
    fn peer_eof_reconciliation_revokes_the_actual_retained_d_owner() {
        let (owner, authority, qualified) = issued_test_d(29);
        qualified.verify_current(&authority).unwrap();
        let mut phase = ProjectionNativeAdmissionPhase::TerminalPendingReconciliation;

        reconcile_terminal_phase(&mut phase, || owner.revoke(qualified.invocation())).unwrap();

        assert_eq!(phase, ProjectionNativeAdmissionPhase::Terminal);
        assert!(qualified.verify_current(&authority).is_err());
    }
}
