//! Canonical child protocol for native projection admission.
//!
//! This crate deliberately owns only the authenticated producer/reader wire
//! statements, actual loaded-factory observation, and the child session.  The
//! neutral checker, build reviews, confinement issuer, and parent admission
//! authority remain in their respective owner crates.  Protocol statements are
//! descriptive: constructing one never creates admission authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::{
    collections::BTreeSet,
    fmt, thread,
    time::{Duration, Instant},
};

use sim_codec_binary::{decode_frame, encode_frame};
use sim_conformance_core::content_id_datum;
use sim_kernel::{CodecId, ContentId, Datum, Expr, Symbol};
use sim_lib_server::{FrameEnvelope, FrameKind, ServerFrame, decode_transport_frame};
use sim_platform_linux::{KernelObjectIdentity, NativeDescriptorMessage};
use sim_platform_ubuntu_projection::UbuntuProjectionAdmissionChannel;
use sim_run_core::{
    CliError, ProjectionConfigurationShapeEvidence, ProjectionFactoryEvidenceVisitor,
    ProjectionOwnedEvidence, RetainedProjectionFactoryCapture,
};

const POLL_INTERVAL: Duration = Duration::from_millis(2);

/// A canonical protocol statement was invalid or its live owner disappeared.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionAdmissionProtocolError {
    detail: String,
}

impl ProjectionAdmissionProtocolError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ProjectionAdmissionProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for ProjectionAdmissionProtocolError {}

/// Durable phase shared by the parent attempt and activated child session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionNativeAdmissionPhase {
    /// No transport effect has been attempted.
    Unentered,
    /// The unique parent offer transmission has been attempted.
    Offered,
    /// Exact current C has been received and checked.
    ChildCurrent,
    /// D is issued and retained; provisional delivery may be uncertain.
    ReceiptPending,
    /// The child acknowledged the exact provisional receipt.
    Acknowledged,
    /// The postchecked activation transmission has been attempted.
    Activating,
    /// Handshake is complete and per-operation service is permitted.
    Serving,
    /// Peer EOF ended service before platform terminal reconciliation.
    TerminalPendingReconciliation,
    /// No further protocol messages are allowed.
    Terminal,
    /// A definite protocol refusal ended the session.
    Refused,
    /// A transport or postcheck result is ambiguous and must not be retried.
    Uncertain,
}

/// Canonical descriptive message 6 sent by the retained parent owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParentOffer {
    administrative_owner: KernelObjectIdentity,
    attempt: ContentId,
    challenge: ContentId,
    implementation: ContentId,
    complete_input_closure: ContentId,
    expected_capture: ContentId,
    policies: BTreeSet<ContentId>,
}

impl ParentOffer {
    /// Creates a descriptive offer from values already obtained under parent custody.
    pub fn new(
        administrative_owner: KernelObjectIdentity,
        attempt: ContentId,
        challenge: ContentId,
        implementation: ContentId,
        complete_input_closure: ContentId,
        expected_capture: ContentId,
        policies: BTreeSet<ContentId>,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        require_nonempty_set(&policies, "projection admission policy set is empty")?;
        Ok(Self {
            administrative_owner,
            attempt,
            challenge,
            implementation,
            complete_input_closure,
            expected_capture,
            policies,
        })
    }

    /// Decodes and descriptor-binds the canonical message 6.
    pub fn from_message(
        message: NativeDescriptorMessage,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        let frame = strict_frame(&message, FrameKind::Response, "native-offer", 6, Some(5))?;
        let value = frame_datum(&frame)?;
        let fields = node(&value, "native-offer-v5", 7)?;
        let offer = Self::new(
            kernel_identity(field(fields, "administrative-owner")?)?,
            content_id(field(fields, "attempt")?)?,
            content_id(field(fields, "challenge")?)?,
            content_id(field(fields, "implementation")?)?,
            content_id(field(fields, "complete-input-closure")?)?,
            content_id(field(fields, "expected-capture")?)?,
            policy_set(field(fields, "policies")?)?,
        )?;
        message.bind_frame(&offer.frame()?, 0).map_err(protocol)?;
        Ok(offer)
    }

    /// Encodes the one canonical message-6 frame.
    pub fn frame(&self) -> Result<ServerFrame, ProjectionAdmissionProtocolError> {
        response_frame("native-offer", 6, Some(5), self.datum())
    }

    fn datum(&self) -> Datum {
        projection_node(
            "native-offer-v5",
            vec![
                (
                    "administrative-owner",
                    kernel_identity_datum(self.administrative_owner),
                ),
                ("attempt", content_id_datum(&self.attempt)),
                ("challenge", content_id_datum(&self.challenge)),
                ("implementation", content_id_datum(&self.implementation)),
                (
                    "complete-input-closure",
                    content_id_datum(&self.complete_input_closure),
                ),
                ("expected-capture", content_id_datum(&self.expected_capture)),
                ("policies", ids_datum(&self.policies)),
            ],
        )
    }

    /// Parent kernel-object identity bound to the authenticated channel.
    #[must_use]
    pub const fn administrative_owner(&self) -> KernelObjectIdentity {
        self.administrative_owner
    }

    /// Owner-derived one-shot attempt identity.
    #[must_use]
    pub const fn attempt(&self) -> &ContentId {
        &self.attempt
    }

    /// Challenge binding the exact producer and platform observations.
    #[must_use]
    pub const fn challenge(&self) -> &ContentId {
        &self.challenge
    }

    /// Exact executing implementation identity.
    #[must_use]
    pub const fn implementation(&self) -> &ContentId {
        &self.implementation
    }

    /// Exact complete consumed input closure.
    #[must_use]
    pub const fn complete_input_closure(&self) -> &ContentId {
        &self.complete_input_closure
    }

    /// Expected retained factory capture.
    #[must_use]
    pub const fn expected_capture(&self) -> &ContentId {
        &self.expected_capture
    }

    /// Exact selected policy-owner set.
    #[must_use]
    pub const fn policies(&self) -> &BTreeSet<ContentId> {
        &self.policies
    }
}

/// Canonical descriptive message 7 produced from actual retained child C.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildCurrent {
    attempt: ContentId,
    challenge: ContentId,
    implementation: ContentId,
    complete_input_closure: ContentId,
    capture: ContentId,
    policies: BTreeSet<ContentId>,
    composition: ContentId,
    configuration_shape: ContentId,
    providers: Vec<ContentId>,
    baseline: ContentId,
    graphs: Vec<ContentId>,
    closure: ContentId,
}

impl ChildCurrent {
    fn from_observation(
        offer: &ParentOffer,
        child: &ProjectionChildFactoryObservation,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        let current = Self {
            attempt: offer.attempt.clone(),
            challenge: offer.challenge.clone(),
            implementation: offer.implementation.clone(),
            complete_input_closure: offer.complete_input_closure.clone(),
            capture: child.factory.projection_capture().clone(),
            policies: child.policies.clone(),
            composition: child.composition.identity.clone(),
            configuration_shape: child.composition.configuration_shape.clone(),
            providers: child.composition.providers.clone(),
            baseline: child.composition.baseline.clone(),
            graphs: child.composition.graphs.clone(),
            closure: child.composition.closure.clone(),
        };
        current.verify_composition()?;
        Ok(current)
    }

    /// Decodes, binds, and internally verifies canonical message 7.
    pub fn from_message(
        message: NativeDescriptorMessage,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        let frame = strict_frame(&message, FrameKind::Response, "child-current", 7, Some(6))?;
        let value = frame_datum(&frame)?;
        let fields = node(&value, "child-current-v4", 12)?;
        let current = Self {
            attempt: content_id(field(fields, "attempt")?)?,
            challenge: content_id(field(fields, "challenge")?)?,
            implementation: content_id(field(fields, "implementation")?)?,
            complete_input_closure: content_id(field(fields, "complete-input-closure")?)?,
            capture: content_id(field(fields, "capture")?)?,
            policies: policy_set(field(fields, "policies")?)?,
            composition: content_id(field(fields, "composition")?)?,
            configuration_shape: content_id(field(fields, "configuration-shape")?)?,
            providers: ordered_ids(field(fields, "providers")?)?,
            baseline: content_id(field(fields, "baseline")?)?,
            graphs: ordered_ids(field(fields, "graphs")?)?,
            closure: content_id(field(fields, "closure")?)?,
        };
        current.verify_composition()?;
        message.bind_frame(&current.frame()?, 0).map_err(protocol)?;
        Ok(current)
    }

    /// Encodes the one canonical message-7 frame.
    pub fn frame(&self) -> Result<ServerFrame, ProjectionAdmissionProtocolError> {
        response_frame("child-current", 7, Some(6), self.datum())
    }

    /// Canonical identity of the complete message-7 subject.
    pub fn identity(&self) -> Result<ContentId, ProjectionAdmissionProtocolError> {
        self.datum().content_id().map_err(protocol)
    }

    /// Re-derives and checks the aggregate child composition identity.
    pub fn verify_composition(&self) -> Result<(), ProjectionAdmissionProtocolError> {
        if self.composition != child_composition_id(self)? {
            return Err(ProjectionAdmissionProtocolError::new(
                "projection child composition statement is internally inconsistent",
            ));
        }
        Ok(())
    }

    fn datum(&self) -> Datum {
        projection_node(
            "child-current-v4",
            vec![
                ("attempt", content_id_datum(&self.attempt)),
                ("challenge", content_id_datum(&self.challenge)),
                ("implementation", content_id_datum(&self.implementation)),
                (
                    "complete-input-closure",
                    content_id_datum(&self.complete_input_closure),
                ),
                ("capture", content_id_datum(&self.capture)),
                ("policies", ids_datum(&self.policies)),
                ("composition", content_id_datum(&self.composition)),
                (
                    "configuration-shape",
                    content_id_datum(&self.configuration_shape),
                ),
                ("providers", ordered_ids_datum(&self.providers)),
                ("baseline", content_id_datum(&self.baseline)),
                ("graphs", ordered_ids_datum(&self.graphs)),
                ("closure", content_id_datum(&self.closure)),
            ],
        )
    }

    /// Attempt identity repeated from message 6.
    #[must_use]
    pub const fn attempt(&self) -> &ContentId {
        &self.attempt
    }
    /// Challenge repeated from message 6.
    #[must_use]
    pub const fn challenge(&self) -> &ContentId {
        &self.challenge
    }
    /// Exact implementation repeated from message 6.
    #[must_use]
    pub const fn implementation(&self) -> &ContentId {
        &self.implementation
    }
    /// Complete input closure repeated from message 6.
    #[must_use]
    pub const fn complete_input_closure(&self) -> &ContentId {
        &self.complete_input_closure
    }
    /// Actual retained factory capture.
    #[must_use]
    pub const fn capture(&self) -> &ContentId {
        &self.capture
    }
    /// Actual selected policy-owner set.
    #[must_use]
    pub const fn policies(&self) -> &BTreeSet<ContentId> {
        &self.policies
    }
    /// Actual complete child composition identity.
    #[must_use]
    pub const fn composition(&self) -> &ContentId {
        &self.composition
    }
    /// Checked configuration and Shape selection.
    #[must_use]
    pub const fn configuration_shape(&self) -> &ContentId {
        &self.configuration_shape
    }
    /// Ordered provider-owner identities.
    #[must_use]
    pub fn providers(&self) -> &[ContentId] {
        &self.providers
    }
    /// Baseline-membership owner identity.
    #[must_use]
    pub const fn baseline(&self) -> &ContentId {
        &self.baseline
    }
    /// Ordered graph-owner identities.
    #[must_use]
    pub fn graphs(&self) -> &[ContentId] {
        &self.graphs
    }
    /// Federated closure owner identity.
    #[must_use]
    pub const fn closure(&self) -> &ContentId {
        &self.closure
    }
}

/// Canonical descriptive message 8 carrying provisional parent D.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Admitted {
    attempt: ContentId,
    challenge: ContentId,
    child_current: ContentId,
    receipt: ContentId,
    implementation: ContentId,
    complete_input_closure: ContentId,
}

impl Admitted {
    /// Creates message 8 from a parent-issued receipt and exact admitted values.
    pub fn new(
        attempt: ContentId,
        challenge: ContentId,
        child_current: ContentId,
        receipt: ContentId,
        implementation: ContentId,
        complete_input_closure: ContentId,
    ) -> Self {
        Self {
            attempt,
            challenge,
            child_current,
            receipt,
            implementation,
            complete_input_closure,
        }
    }

    /// Decodes and descriptor-binds canonical message 8.
    pub fn from_message(
        message: NativeDescriptorMessage,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        let frame = strict_frame(&message, FrameKind::Response, "admitted", 8, Some(7))?;
        let value = frame_datum(&frame)?;
        let fields = node(&value, "admitted-v3", 6)?;
        let admitted = Self::new(
            content_id(field(fields, "attempt")?)?,
            content_id(field(fields, "challenge")?)?,
            content_id(field(fields, "child-current")?)?,
            content_id(field(fields, "receipt")?)?,
            content_id(field(fields, "implementation")?)?,
            content_id(field(fields, "complete-input-closure")?)?,
        );
        message
            .bind_frame(&admitted.frame()?, 0)
            .map_err(protocol)?;
        Ok(admitted)
    }

    /// Encodes the one canonical message-8 frame.
    pub fn frame(&self) -> Result<ServerFrame, ProjectionAdmissionProtocolError> {
        response_frame(
            "admitted",
            8,
            Some(7),
            projection_node(
                "admitted-v3",
                vec![
                    ("attempt", content_id_datum(&self.attempt)),
                    ("challenge", content_id_datum(&self.challenge)),
                    ("child-current", content_id_datum(&self.child_current)),
                    ("receipt", content_id_datum(&self.receipt)),
                    ("implementation", content_id_datum(&self.implementation)),
                    (
                        "complete-input-closure",
                        content_id_datum(&self.complete_input_closure),
                    ),
                ],
            ),
        )
    }

    /// Attempt identity.
    #[must_use]
    pub const fn attempt(&self) -> &ContentId {
        &self.attempt
    }
    /// Challenge identity.
    #[must_use]
    pub const fn challenge(&self) -> &ContentId {
        &self.challenge
    }
    /// Current-C statement identity.
    #[must_use]
    pub const fn child_current(&self) -> &ContentId {
        &self.child_current
    }
    /// Parent-issued D receipt identity.
    #[must_use]
    pub const fn receipt(&self) -> &ContentId {
        &self.receipt
    }
    /// Admitted implementation identity.
    #[must_use]
    pub const fn implementation(&self) -> &ContentId {
        &self.implementation
    }
    /// Admitted complete input closure.
    #[must_use]
    pub const fn complete_input_closure(&self) -> &ContentId {
        &self.complete_input_closure
    }
}

/// Canonical descriptive message 9 acknowledging provisional D.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionAcknowledged {
    attempt: ContentId,
    challenge: ContentId,
    child_current: ContentId,
    receipt: ContentId,
}

impl AdmissionAcknowledged {
    /// Creates the exact child acknowledgement of message 8.
    #[must_use]
    pub fn new(admitted: &Admitted) -> Self {
        Self {
            attempt: admitted.attempt.clone(),
            challenge: admitted.challenge.clone(),
            child_current: admitted.child_current.clone(),
            receipt: admitted.receipt.clone(),
        }
    }

    /// Decodes and descriptor-binds canonical message 9.
    pub fn from_message(
        message: NativeDescriptorMessage,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        let frame = strict_frame(
            &message,
            FrameKind::Response,
            "admission-acknowledged",
            9,
            Some(8),
        )?;
        let value = frame_datum(&frame)?;
        let fields = node(&value, "admission-acknowledged-v2", 4)?;
        let acknowledged = Self {
            attempt: content_id(field(fields, "attempt")?)?,
            challenge: content_id(field(fields, "challenge")?)?,
            child_current: content_id(field(fields, "child-current")?)?,
            receipt: content_id(field(fields, "receipt")?)?,
        };
        message
            .bind_frame(&acknowledged.frame()?, 0)
            .map_err(protocol)?;
        Ok(acknowledged)
    }

    /// Encodes the one canonical message-9 frame.
    pub fn frame(&self) -> Result<ServerFrame, ProjectionAdmissionProtocolError> {
        response_frame("admission-acknowledged", 9, Some(8), self.datum())
    }

    /// Canonical acknowledgement identity.
    pub fn identity(&self) -> Result<ContentId, ProjectionAdmissionProtocolError> {
        self.datum().content_id().map_err(protocol)
    }

    fn datum(&self) -> Datum {
        projection_node(
            "admission-acknowledged-v2",
            vec![
                ("attempt", content_id_datum(&self.attempt)),
                ("challenge", content_id_datum(&self.challenge)),
                ("child-current", content_id_datum(&self.child_current)),
                ("receipt", content_id_datum(&self.receipt)),
            ],
        )
    }

    /// Acknowledged attempt.
    #[must_use]
    pub const fn attempt(&self) -> &ContentId {
        &self.attempt
    }
    /// Acknowledged challenge.
    #[must_use]
    pub const fn challenge(&self) -> &ContentId {
        &self.challenge
    }
    /// Acknowledged current-C statement.
    #[must_use]
    pub const fn child_current(&self) -> &ContentId {
        &self.child_current
    }
    /// Acknowledged D receipt.
    #[must_use]
    pub const fn receipt(&self) -> &ContentId {
        &self.receipt
    }
}

/// Canonical descriptive message 10 activating postchecked D.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Activated {
    attempt: ContentId,
    challenge: ContentId,
    child_current: ContentId,
    receipt: ContentId,
    acknowledgement: ContentId,
}

impl Activated {
    /// Creates parent activation after currentness postchecks.
    pub fn new(
        attempt: ContentId,
        challenge: ContentId,
        child_current: ContentId,
        receipt: ContentId,
        acknowledgement: ContentId,
    ) -> Self {
        Self {
            attempt,
            challenge,
            child_current,
            receipt,
            acknowledgement,
        }
    }

    /// Decodes and descriptor-binds canonical message 10.
    pub fn from_message(
        message: NativeDescriptorMessage,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        let frame = strict_frame(&message, FrameKind::Response, "activated", 10, Some(9))?;
        let value = frame_datum(&frame)?;
        let fields = node(&value, "activated-v2", 5)?;
        let activated = Self::new(
            content_id(field(fields, "attempt")?)?,
            content_id(field(fields, "challenge")?)?,
            content_id(field(fields, "child-current")?)?,
            content_id(field(fields, "receipt")?)?,
            content_id(field(fields, "acknowledgement")?)?,
        );
        message
            .bind_frame(&activated.frame()?, 0)
            .map_err(protocol)?;
        Ok(activated)
    }

    /// Encodes the one canonical message-10 frame.
    pub fn frame(&self) -> Result<ServerFrame, ProjectionAdmissionProtocolError> {
        response_frame("activated", 10, Some(9), self.datum())
    }

    /// Canonical activation identity.
    pub fn identity(&self) -> Result<ContentId, ProjectionAdmissionProtocolError> {
        self.datum().content_id().map_err(protocol)
    }

    fn datum(&self) -> Datum {
        projection_node(
            "activated-v2",
            vec![
                ("attempt", content_id_datum(&self.attempt)),
                ("challenge", content_id_datum(&self.challenge)),
                ("child-current", content_id_datum(&self.child_current)),
                ("receipt", content_id_datum(&self.receipt)),
                ("acknowledgement", content_id_datum(&self.acknowledgement)),
            ],
        )
    }

    /// Activated attempt.
    #[must_use]
    pub const fn attempt(&self) -> &ContentId {
        &self.attempt
    }
    /// Activated challenge.
    #[must_use]
    pub const fn challenge(&self) -> &ContentId {
        &self.challenge
    }
    /// Activated current-C statement.
    #[must_use]
    pub const fn child_current(&self) -> &ContentId {
        &self.child_current
    }
    /// Activated D receipt.
    #[must_use]
    pub const fn receipt(&self) -> &ContentId {
        &self.receipt
    }
    /// Exact child acknowledgement identity.
    #[must_use]
    pub const fn acknowledgement(&self) -> &ContentId {
        &self.acknowledgement
    }
}

/// Canonical descriptive message 11 completing activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationComplete {
    attempt: ContentId,
    activation: ContentId,
}

impl ActivationComplete {
    /// Creates the exact child completion of message 10.
    #[must_use]
    pub fn new(activated: &Activated) -> Result<Self, ProjectionAdmissionProtocolError> {
        Ok(Self {
            attempt: activated.attempt.clone(),
            activation: activated.identity()?,
        })
    }

    /// Decodes and descriptor-binds canonical message 11.
    pub fn from_message(
        message: NativeDescriptorMessage,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        let frame = strict_frame(
            &message,
            FrameKind::Response,
            "activation-complete",
            11,
            Some(10),
        )?;
        let value = frame_datum(&frame)?;
        let fields = node(&value, "activation-complete-v2", 2)?;
        let complete = Self {
            attempt: content_id(field(fields, "attempt")?)?,
            activation: content_id(field(fields, "activation")?)?,
        };
        message
            .bind_frame(&complete.frame()?, 0)
            .map_err(protocol)?;
        Ok(complete)
    }

    /// Encodes the one canonical message-11 frame.
    pub fn frame(&self) -> Result<ServerFrame, ProjectionAdmissionProtocolError> {
        response_frame(
            "activation-complete",
            11,
            Some(10),
            projection_node(
                "activation-complete-v2",
                vec![
                    ("attempt", content_id_datum(&self.attempt)),
                    ("activation", content_id_datum(&self.activation)),
                ],
            ),
        )
    }

    /// Completed attempt.
    #[must_use]
    pub const fn attempt(&self) -> &ContentId {
        &self.attempt
    }
    /// Exact activation identity.
    #[must_use]
    pub const fn activation(&self) -> &ContentId {
        &self.activation
    }
}

/// Canonical four-frame statement for one admitted World operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationStatement {
    attempt: ContentId,
    sequence: u64,
    policy: ContentId,
    implementation: ContentId,
    complete_input_closure: ContentId,
    receipt: ContentId,
}

impl OperationStatement {
    /// Creates a statement from one activated session and selected policy.
    #[must_use]
    pub fn new(
        attempt: ContentId,
        sequence: u64,
        policy: ContentId,
        implementation: ContentId,
        complete_input_closure: ContentId,
        receipt: ContentId,
    ) -> Self {
        Self {
            attempt,
            sequence,
            policy,
            implementation,
            complete_input_closure,
            receipt,
        }
    }

    /// Decodes and binds the next expected begin frame.
    pub fn from_begin(
        message: NativeDescriptorMessage,
        expected_sequence: u64,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        let (msg_id, correlate) = operation_ids(expected_sequence, 0)?;
        let frame = strict_frame(&message, FrameKind::Request, "begin", msg_id, correlate)?;
        let value = frame_datum(&frame)?;
        let fields = node(&value, "native-operation-v1", 6)?;
        let sequence = decimal_u64(
            field(fields, "sequence")?,
            "projection operation sequence differs",
        )?;
        if sequence != expected_sequence {
            return Err(ProjectionAdmissionProtocolError::new(
                "projection operation sequence was replayed",
            ));
        }
        let statement = Self::new(
            content_id(field(fields, "attempt")?)?,
            sequence,
            content_id(field(fields, "policy")?)?,
            content_id(field(fields, "implementation")?)?,
            content_id(field(fields, "complete-input-closure")?)?,
            content_id(field(fields, "receipt")?)?,
        );
        message
            .bind_frame(&statement.frame(0, FrameKind::Request, "begin")?, 0)
            .map_err(protocol)?;
        Ok(statement)
    }

    /// Encodes one of the canonical begin/granted/finish/accepted frames.
    pub fn frame(
        &self,
        offset: u64,
        kind: FrameKind,
        role: &str,
    ) -> Result<ServerFrame, ProjectionAdmissionProtocolError> {
        let (msg_id, correlate) = operation_ids(self.sequence, offset)?;
        let mut frame = ServerFrame::new(
            Symbol::qualified("codec", "binary"),
            kind,
            FrameEnvelope {
                role: Some(Symbol::qualified("projection-admission", role)),
                ..FrameEnvelope::default()
            },
            encode_frame(&Expr::from(self.datum())).map_err(protocol)?.0,
        );
        frame.msg_id = Some(msg_id);
        frame.correlate = correlate;
        Ok(frame)
    }

    fn datum(&self) -> Datum {
        projection_node(
            "native-operation-v1",
            vec![
                ("attempt", content_id_datum(&self.attempt)),
                ("sequence", Datum::String(self.sequence.to_string())),
                ("policy", content_id_datum(&self.policy)),
                ("implementation", content_id_datum(&self.implementation)),
                (
                    "complete-input-closure",
                    content_id_datum(&self.complete_input_closure),
                ),
                ("receipt", content_id_datum(&self.receipt)),
            ],
        )
    }

    /// Attempt identity.
    #[must_use]
    pub const fn attempt(&self) -> &ContentId {
        &self.attempt
    }
    /// Operation sequence.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Selected policy owner.
    #[must_use]
    pub const fn policy(&self) -> &ContentId {
        &self.policy
    }
    /// Admitted implementation.
    #[must_use]
    pub const fn implementation(&self) -> &ContentId {
        &self.implementation
    }
    /// Admitted complete input closure.
    #[must_use]
    pub const fn complete_input_closure(&self) -> &ContentId {
        &self.complete_input_closure
    }
    /// Admitted D receipt.
    #[must_use]
    pub const fn receipt(&self) -> &ContentId {
        &self.receipt
    }
}

/// Child-side observation of the actual loaded C factory and composition.
pub struct ProjectionChildFactoryObservation {
    factory: RetainedProjectionFactoryCapture,
    policies: BTreeSet<ContentId>,
    composition: ProjectionChildComposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectionChildComposition {
    identity: ContentId,
    configuration_shape: ContentId,
    providers: Vec<ContentId>,
    policies: BTreeSet<ContentId>,
    baseline: ContentId,
    graphs: Vec<ContentId>,
    closure: ContentId,
}

impl ProjectionChildFactoryObservation {
    /// Observes and retains the actual loaded child factory.
    pub fn observe(
        factory: RetainedProjectionFactoryCapture,
        policies: BTreeSet<ContentId>,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        if policies.is_empty() {
            return Err(ProjectionAdmissionProtocolError::new(
                "projection child policy selection is empty",
            ));
        }
        let composition = observe_child_factory(&factory, &policies).map_err(protocol)?;
        Ok(Self {
            factory,
            policies,
            composition,
        })
    }

    fn while_current<T>(
        &self,
        expected_capture: &ContentId,
        expected_policies: &BTreeSet<ContentId>,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionAdmissionProtocolError> {
        if self.factory.projection_capture() != expected_capture
            || &self.policies != expected_policies
        {
            return Err(ProjectionAdmissionProtocolError::new(
                "actual child factory differs from the parent boot selection",
            ));
        }
        let mut visitor = ConcreteProjectionVisitor::matching(&self.policies, &self.composition);
        self.factory
            .with_projection_evidence_current(&mut visitor, action)
            .map_err(protocol)
    }
}

/// Descriptive child result of an admitted D receipt.
pub struct ProjectionNativeAdmission {
    attempt: ContentId,
    challenge: ContentId,
    child_current: ContentId,
    receipt: ContentId,
    implementation: ContentId,
    complete_input_closure: ContentId,
}

impl ProjectionNativeAdmission {
    /// Owner-derived attempt identity.
    #[must_use]
    pub const fn attempt(&self) -> &ContentId {
        &self.attempt
    }
    /// Parent-issued D receipt identity.
    #[must_use]
    pub const fn receipt(&self) -> &ContentId {
        &self.receipt
    }
    /// Exact handshake challenge.
    #[must_use]
    pub const fn challenge(&self) -> &ContentId {
        &self.challenge
    }
    /// Exact current-C statement identity.
    #[must_use]
    pub const fn child_current(&self) -> &ContentId {
        &self.child_current
    }
    /// Descriptive admitted implementation identity.
    #[must_use]
    pub const fn implementation(&self) -> &ContentId {
        &self.implementation
    }
    /// Descriptive admitted complete input closure.
    #[must_use]
    pub const fn complete_input_closure(&self) -> &ContentId {
        &self.complete_input_closure
    }
}

/// Descriptive parent offer received through the authenticated child channel.
pub struct ProjectionNativeAdmissionOffer {
    offer: ParentOffer,
}

impl ProjectionNativeAdmissionOffer {
    fn receive_once(
        channel: &UbuntuProjectionAdmissionChannel,
        deadline: Instant,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        channel.revalidate().map_err(protocol)?;
        let offer = ParentOffer::from_message(receive_child_until(channel, deadline)?)?;
        if offer.administrative_owner != channel.creator_identity() {
            return Err(ProjectionAdmissionProtocolError::new(
                "projection admission offer came from a substituted administrative owner",
            ));
        }
        channel.revalidate().map_err(protocol)?;
        Ok(Self { offer })
    }

    /// Exact expected loaded-factory capture.
    #[must_use]
    pub const fn expected_capture(&self) -> &ContentId {
        &self.offer.expected_capture
    }
    /// Exact selected policy owners.
    #[must_use]
    pub const fn policies(&self) -> &BTreeSet<ContentId> {
        &self.offer.policies
    }
    /// Exact executing implementation.
    #[must_use]
    pub const fn implementation(&self) -> &ContentId {
        &self.offer.implementation
    }
    /// Exact complete consumed input closure.
    #[must_use]
    pub const fn complete_input_closure(&self) -> &ContentId {
        &self.offer.complete_input_closure
    }
}

/// Inert activated child session retaining actual C and its authenticated channel.
pub struct ActivatedProjectionAdmissionSession {
    channel: UbuntuProjectionAdmissionChannel,
    child: ProjectionChildFactoryObservation,
    admission: ProjectionNativeAdmission,
    expected_capture: ContentId,
    policies: BTreeSet<ContentId>,
    next_sequence: u64,
    phase: ProjectionNativeAdmissionPhase,
}

impl ActivatedProjectionAdmissionSession {
    /// Consumes actual C and the authenticated channel to complete messages 6-11.
    pub fn activate(
        child: ProjectionChildFactoryObservation,
        channel: UbuntuProjectionAdmissionChannel,
        deadline: Instant,
    ) -> Result<Self, ProjectionAdmissionProtocolError> {
        let received = ProjectionNativeAdmissionOffer::receive_once(&channel, deadline)?;
        let offer = received.offer;
        if offer.administrative_owner != channel.creator_identity() {
            return Err(ProjectionAdmissionProtocolError::new(
                "projection admission channel owner changed after offer",
            ));
        }
        let admission = child.while_current(
            &offer.expected_capture,
            &offer.policies,
            || -> Result<ProjectionNativeAdmission, ProjectionAdmissionProtocolError> {
                channel.revalidate().map_err(protocol)?;
                let current = ChildCurrent::from_observation(&offer, &child)?;
                let child_current = current.identity()?;
                channel.send_frame(&current.frame()?).map_err(protocol)?;
                let admitted = Admitted::from_message(receive_child_until(&channel, deadline)?)?;
                if admitted.attempt != offer.attempt
                    || admitted.challenge != offer.challenge
                    || admitted.child_current != child_current
                    || admitted.implementation != offer.implementation
                    || admitted.complete_input_closure != offer.complete_input_closure
                {
                    return Err(ProjectionAdmissionProtocolError::new(
                        "projection admitted statement does not acknowledge current C",
                    ));
                }
                channel.revalidate().map_err(protocol)?;
                let acknowledgement = AdmissionAcknowledged::new(&admitted);
                let acknowledgement_id = acknowledgement.identity()?;
                let admission = ProjectionNativeAdmission {
                    attempt: admitted.attempt,
                    challenge: admitted.challenge,
                    child_current: admitted.child_current,
                    receipt: admitted.receipt,
                    implementation: admitted.implementation,
                    complete_input_closure: admitted.complete_input_closure,
                };
                channel
                    .send_frame(&acknowledgement.frame()?)
                    .map_err(protocol)?;
                let activated = Activated::from_message(receive_child_until(&channel, deadline)?)?;
                if activated.attempt != admission.attempt
                    || activated.challenge != admission.challenge
                    || activated.child_current != admission.child_current
                    || activated.receipt != admission.receipt
                    || activated.acknowledgement != acknowledgement_id
                {
                    return Err(ProjectionAdmissionProtocolError::new(
                        "projection activation differs from acknowledged provisional D",
                    ));
                }
                channel
                    .send_frame(&ActivationComplete::new(&activated)?.frame()?)
                    .map_err(protocol)?;
                channel.revalidate().map_err(protocol)?;
                Ok(admission)
            },
        )??;
        Ok(Self {
            channel,
            child,
            admission,
            expected_capture: offer.expected_capture,
            policies: offer.policies,
            next_sequence: 0,
            phase: ProjectionNativeAdmissionPhase::Serving,
        })
    }

    /// Exact admitted implementation.
    #[must_use]
    pub const fn implementation(&self) -> &ContentId {
        self.admission.implementation()
    }
    /// Exact admitted input closure.
    #[must_use]
    pub const fn complete_input_closure(&self) -> &ContentId {
        self.admission.complete_input_closure()
    }
    /// Actual retained C capture.
    #[must_use]
    pub const fn loaded_capture(&self) -> &ContentId {
        &self.expected_capture
    }
    /// Exact selected policy owners.
    #[must_use]
    pub const fn policies(&self) -> &BTreeSet<ContentId> {
        &self.policies
    }
    /// Parent-retained D receipt.
    #[must_use]
    pub const fn checker_receipt(&self) -> &ContentId {
        self.admission.receipt()
    }
    /// Owner-derived attempt identity.
    #[must_use]
    pub const fn attempt(&self) -> &ContentId {
        self.admission.attempt()
    }
    /// Current terminal-safe session phase.
    #[must_use]
    pub const fn phase(&self) -> ProjectionNativeAdmissionPhase {
        self.phase
    }

    /// Executes one World operation between exact grant and acceptance frames.
    pub fn with_world_operation<T>(
        &mut self,
        policy: &ContentId,
        deadline: Instant,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionAdmissionProtocolError> {
        if self.phase != ProjectionNativeAdmissionPhase::Serving {
            return Err(ProjectionAdmissionProtocolError::new(
                "projection child session is not serving",
            ));
        }
        if !self.policies.contains(policy) {
            self.phase = ProjectionNativeAdmissionPhase::Refused;
            return Err(ProjectionAdmissionProtocolError::new(
                "projection operation policy differs",
            ));
        }
        let statement = OperationStatement::new(
            self.admission.attempt.clone(),
            self.next_sequence,
            policy.clone(),
            self.admission.implementation.clone(),
            self.admission.complete_input_closure.clone(),
            self.admission.receipt.clone(),
        );
        if let Err(error) =
            self.channel
                .send_frame(&statement.frame(0, FrameKind::Request, "begin")?)
        {
            self.phase = ProjectionNativeAdmissionPhase::Uncertain;
            return Err(protocol(error));
        }
        let granted = statement.frame(1, FrameKind::Response, "granted")?;
        if let Err(error) = receive_child_bound_until(&self.channel, &granted, deadline) {
            self.phase = ProjectionNativeAdmissionPhase::Uncertain;
            return Err(error);
        }
        let output = self
            .child
            .while_current(&self.expected_capture, &self.policies, || {
                let output = action();
                self.channel
                    .send_frame(&statement.frame(2, FrameKind::Request, "finish")?)
                    .map_err(protocol)?;
                receive_child_bound_until(
                    &self.channel,
                    &statement.frame(3, FrameKind::Response, "accepted")?,
                    deadline,
                )?;
                self.channel.revalidate().map_err(protocol)?;
                Ok::<T, ProjectionAdmissionProtocolError>(output)
            });
        let output = match output {
            Ok(Ok(output)) => output,
            Ok(Err(error)) | Err(error) => {
                self.phase = ProjectionNativeAdmissionPhase::Uncertain;
                return Err(error);
            }
        };
        self.next_sequence = self.next_sequence.checked_add(1).ok_or_else(|| {
            ProjectionAdmissionProtocolError::new("projection operation sequence exhausted")
        })?;
        Ok(output)
    }
}

struct ConcreteProjectionVisitor<'a> {
    expected_policies: &'a BTreeSet<ContentId>,
    expected_composition: Option<&'a ProjectionChildComposition>,
    configuration_shape: Option<ContentId>,
    providers: Vec<ContentId>,
    policies: BTreeSet<ContentId>,
    baseline: Option<ContentId>,
    graphs: Vec<ContentId>,
    closure: Option<ContentId>,
}

impl<'a> ConcreteProjectionVisitor<'a> {
    fn new(expected_policies: &'a BTreeSet<ContentId>) -> Self {
        Self {
            expected_policies,
            expected_composition: None,
            configuration_shape: None,
            providers: Vec::new(),
            policies: BTreeSet::new(),
            baseline: None,
            graphs: Vec::new(),
            closure: None,
        }
    }

    fn matching(
        expected_policies: &'a BTreeSet<ContentId>,
        expected_composition: &'a ProjectionChildComposition,
    ) -> Self {
        let mut visitor = Self::new(expected_policies);
        visitor.expected_composition = Some(expected_composition);
        visitor
    }

    fn snapshot(&self) -> Result<ProjectionChildComposition, CliError> {
        let configuration_shape = self
            .configuration_shape
            .as_ref()
            .ok_or_else(|| CliError::new("projection configuration Shape is absent"))?;
        let baseline = self
            .baseline
            .as_ref()
            .ok_or_else(|| CliError::new("projection baseline owner is absent"))?;
        let closure = self
            .closure
            .as_ref()
            .ok_or_else(|| CliError::new("projection federated closure is absent"))?;
        if self.providers.is_empty() {
            return Err(CliError::new("projection provider set is empty"));
        }
        if self.graphs.is_empty() {
            return Err(CliError::new("projection owner graph set is empty"));
        }
        if &self.policies != self.expected_policies {
            return Err(CliError::new(
                "loaded projection policy objects differ from the selected policy set",
            ));
        }
        let mut providers = self.providers.clone();
        providers.sort();
        let mut graphs = self.graphs.clone();
        graphs.sort();
        let identity = canonical_projection_id(
            "retained-factory-composition-v1",
            &[
                configuration_shape,
                &ordered_projection_id("provider-owners-v1", &providers)?,
                &ordered_projection_id(
                    "selected-policy-owners-v1",
                    &self.policies.iter().cloned().collect::<Vec<_>>(),
                )?,
                baseline,
                &ordered_projection_id("projection-graph-owners-v1", &graphs)?,
                closure,
            ],
        )?;
        Ok(ProjectionChildComposition {
            identity,
            configuration_shape: configuration_shape.clone(),
            providers,
            policies: self.policies.clone(),
            baseline: baseline.clone(),
            graphs,
            closure: closure.clone(),
        })
    }

    fn observe(owner: &dyn ProjectionOwnedEvidence) -> Result<ContentId, CliError> {
        owner.reobserve_current()?;
        let canonical = owner.canonical()?;
        owner.reobserve_current()?;
        canonical
            .content_id()
            .map_err(|error| CliError::new(error.to_string()))
    }
}

impl ProjectionFactoryEvidenceVisitor for ConcreteProjectionVisitor<'_> {
    fn configuration_shape(
        &mut self,
        configuration: &Datum,
        shape: &dyn ProjectionConfigurationShapeEvidence,
    ) -> Result<(), CliError> {
        if self.configuration_shape.is_some() {
            return Err(CliError::new(
                "projection configuration Shape is duplicated",
            ));
        }
        shape.reobserve_current()?;
        shape.verify_configuration(configuration)?;
        let shape_id = Self::observe(shape)?;
        shape.verify_configuration(configuration)?;
        shape.reobserve_current()?;
        self.configuration_shape = Some(canonical_projection_id(
            "configuration-shape-selection-v1",
            &[
                &configuration
                    .content_id()
                    .map_err(|error| CliError::new(error.to_string()))?,
                &shape_id,
            ],
        )?);
        Ok(())
    }

    fn provider_registration(
        &mut self,
        provider: &dyn ProjectionOwnedEvidence,
    ) -> Result<(), CliError> {
        let identity = Self::observe(provider)?;
        if self.providers.contains(&identity) {
            return Err(CliError::new("projection provider owner is duplicated"));
        }
        self.providers.push(identity);
        Ok(())
    }

    fn selected_policy(&mut self, policy: &dyn ProjectionOwnedEvidence) -> Result<(), CliError> {
        let identity = Self::observe(policy)?;
        if !self.policies.insert(identity) {
            return Err(CliError::new("projection policy owner is duplicated"));
        }
        Ok(())
    }

    fn baseline_membership(
        &mut self,
        membership: &dyn ProjectionOwnedEvidence,
    ) -> Result<(), CliError> {
        if self.baseline.replace(Self::observe(membership)?).is_some() {
            return Err(CliError::new("projection baseline owner is duplicated"));
        }
        Ok(())
    }

    fn graph_owner(&mut self, owner: &dyn ProjectionOwnedEvidence) -> Result<(), CliError> {
        let identity = Self::observe(owner)?;
        if self.graphs.contains(&identity) {
            return Err(CliError::new("projection graph owner is duplicated"));
        }
        self.graphs.push(identity);
        Ok(())
    }

    fn federated_closure(&mut self, closure: &dyn ProjectionOwnedEvidence) -> Result<(), CliError> {
        if self.closure.replace(Self::observe(closure)?).is_some() {
            return Err(CliError::new("projection federated closure is duplicated"));
        }
        Ok(())
    }

    fn complete(&mut self) -> Result<(), CliError> {
        let composition = self.snapshot()?;
        if self
            .expected_composition
            .is_some_and(|expected| expected != &composition)
        {
            return Err(CliError::new("actual child projection composition changed"));
        }
        Ok(())
    }
}

fn observe_child_factory(
    factory: &RetainedProjectionFactoryCapture,
    policies: &BTreeSet<ContentId>,
) -> Result<ProjectionChildComposition, CliError> {
    let mut visitor = ConcreteProjectionVisitor::new(policies);
    factory.visit_projection_evidence(&mut visitor)?;
    visitor.snapshot()
}

fn child_composition_id(
    current: &ChildCurrent,
) -> Result<ContentId, ProjectionAdmissionProtocolError> {
    canonical_protocol_id(
        "retained-factory-composition-v1",
        &[
            current.configuration_shape(),
            &ordered_protocol_id("provider-owners-v1", current.providers())?,
            &ordered_protocol_id(
                "selected-policy-owners-v1",
                &current.policies().iter().cloned().collect::<Vec<_>>(),
            )?,
            current.baseline(),
            &ordered_protocol_id("projection-graph-owners-v1", current.graphs())?,
            current.closure(),
        ],
    )
}

fn ordered_projection_id(tag: &str, identities: &[ContentId]) -> Result<ContentId, CliError> {
    canonical_projection_id(tag, &identities.iter().collect::<Vec<_>>())
}

fn canonical_projection_id(tag: &str, identities: &[&ContentId]) -> Result<ContentId, CliError> {
    canonical_node("projection", tag, identities)
        .content_id()
        .map_err(|error| CliError::new(error.to_string()))
}

fn ordered_protocol_id(
    tag: &str,
    ids: &[ContentId],
) -> Result<ContentId, ProjectionAdmissionProtocolError> {
    if ids.is_empty() || ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection child owner membership is empty, duplicated, or unordered",
        ));
    }
    canonical_protocol_id(tag, &ids.iter().collect::<Vec<_>>())
}

fn canonical_protocol_id(
    tag: &str,
    ids: &[&ContentId],
) -> Result<ContentId, ProjectionAdmissionProtocolError> {
    canonical_node("projection", tag, ids)
        .content_id()
        .map_err(protocol)
}

fn canonical_node(namespace: &str, tag: &str, ids: &[&ContentId]) -> Datum {
    Datum::Node {
        tag: Symbol::qualified(namespace, tag),
        fields: vec![(
            Symbol::new("members"),
            Datum::Vector(
                ids.iter()
                    .map(|identity| content_id_datum(identity))
                    .collect(),
            ),
        )],
    }
}

/// Canonical identity of one platform kernel object.
pub fn kernel_identity_id(
    identity: KernelObjectIdentity,
) -> Result<ContentId, ProjectionAdmissionProtocolError> {
    kernel_identity_datum(identity)
        .content_id()
        .map_err(protocol)
}

fn kernel_identity_datum(identity: KernelObjectIdentity) -> Datum {
    projection_node(
        "kernel-object-identity-v1",
        vec![
            ("device", Datum::String(identity.device.to_string())),
            ("inode", Datum::String(identity.inode.to_string())),
        ],
    )
}

fn kernel_identity(
    value: &Datum,
) -> Result<KernelObjectIdentity, ProjectionAdmissionProtocolError> {
    let fields = node(value, "kernel-object-identity-v1", 2)?;
    Ok(KernelObjectIdentity {
        device: decimal_u64(
            field(fields, "device")?,
            "projection owner identity differs",
        )?,
        inode: decimal_u64(field(fields, "inode")?, "projection owner identity differs")?,
    })
}

fn response_frame(
    role: &str,
    msg_id: u64,
    correlate: Option<u64>,
    payload: Datum,
) -> Result<ServerFrame, ProjectionAdmissionProtocolError> {
    let mut frame = ServerFrame::new(
        Symbol::qualified("codec", "binary"),
        FrameKind::Response,
        FrameEnvelope {
            role: Some(Symbol::qualified("projection-admission", role)),
            ..FrameEnvelope::default()
        },
        encode_frame(&Expr::from(payload)).map_err(protocol)?.0,
    );
    frame.msg_id = Some(msg_id);
    frame.correlate = correlate;
    Ok(frame)
}

fn strict_frame(
    message: &NativeDescriptorMessage,
    kind: FrameKind,
    role: &str,
    msg_id: u64,
    correlate: Option<u64>,
) -> Result<ServerFrame, ProjectionAdmissionProtocolError> {
    if !message.descriptors.is_empty() {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission statement carried descriptors",
        ));
    }
    let frame = decode_transport_frame(&message.bytes).map_err(protocol)?;
    if frame.codec != Symbol::qualified("codec", "binary")
        || frame.kind != kind
        || frame.envelope
            != (FrameEnvelope {
                role: Some(Symbol::qualified("projection-admission", role)),
                ..FrameEnvelope::default()
            })
        || frame.msg_id != Some(msg_id)
        || frame.correlate != correlate
    {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission statement envelope differs",
        ));
    }
    Ok(frame)
}

fn frame_datum(frame: &ServerFrame) -> Result<Datum, ProjectionAdmissionProtocolError> {
    let (_, expression) = decode_frame(CodecId(0), &frame.payload).map_err(protocol)?;
    Datum::try_from(expression).map_err(protocol)
}

fn receive_child_until(
    channel: &UbuntuProjectionAdmissionChannel,
    deadline: Instant,
) -> Result<NativeDescriptorMessage, ProjectionAdmissionProtocolError> {
    loop {
        if Instant::now() >= deadline {
            return Err(ProjectionAdmissionProtocolError::new(
                "projection admission response deadline expired",
            ));
        }
        match channel.receive().map_err(protocol)? {
            Some(message) => return Ok(message),
            None => thread::sleep(POLL_INTERVAL),
        }
    }
}

fn receive_child_bound_until(
    channel: &UbuntuProjectionAdmissionChannel,
    expected: &ServerFrame,
    deadline: Instant,
) -> Result<(), ProjectionAdmissionProtocolError> {
    receive_child_until(channel, deadline)?
        .bind_frame(expected, 0)
        .map(|_| ())
        .map_err(protocol)
}

fn operation_ids(
    sequence: u64,
    offset: u64,
) -> Result<(u64, Option<u64>), ProjectionAdmissionProtocolError> {
    if offset > 3 {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection operation frame offset differs",
        ));
    }
    let base = sequence
        .checked_mul(4)
        .and_then(|value| value.checked_add(12))
        .ok_or_else(|| {
            ProjectionAdmissionProtocolError::new("projection operation sequence exhausted")
        })?;
    let msg_id = base.checked_add(offset).ok_or_else(|| {
        ProjectionAdmissionProtocolError::new("projection operation sequence exhausted")
    })?;
    let correlate = if offset == 0 && sequence == 0 {
        Some(10)
    } else if offset == 0 {
        Some(base - 1)
    } else {
        Some(msg_id - 1)
    };
    Ok((msg_id, correlate))
}

fn projection_node(tag: &str, fields: Vec<(&str, Datum)>) -> Datum {
    Datum::Node {
        tag: Symbol::qualified("projection-admission", tag),
        fields: fields
            .into_iter()
            .map(|(name, value)| (Symbol::new(name), value))
            .collect(),
    }
}

fn node<'a>(
    value: &'a Datum,
    tag: &str,
    count: usize,
) -> Result<&'a [(Symbol, Datum)], ProjectionAdmissionProtocolError> {
    match value {
        Datum::Node {
            tag: actual,
            fields,
        } if actual == &Symbol::qualified("projection-admission", tag) && fields.len() == count => {
            Ok(fields)
        }
        _ => Err(ProjectionAdmissionProtocolError::new(
            "projection admission statement schema differs",
        )),
    }
}

fn field<'a>(
    fields: &'a [(Symbol, Datum)],
    name: &str,
) -> Result<&'a Datum, ProjectionAdmissionProtocolError> {
    let key = Symbol::new(name);
    let mut values = fields.iter().filter(|(candidate, _)| candidate == &key);
    let value = values.next().map(|(_, value)| value).ok_or_else(|| {
        ProjectionAdmissionProtocolError::new("projection admission statement field is absent")
    })?;
    if values.next().is_some() {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission statement field is duplicated",
        ));
    }
    Ok(value)
}

fn content_id(value: &Datum) -> Result<ContentId, ProjectionAdmissionProtocolError> {
    let Datum::Node { tag, fields } = value else {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission content identity differs",
        ));
    };
    if tag != &Symbol::qualified("conformance", "content-id-v1") || fields.len() != 2 {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission content identity differs",
        ));
    }
    let algorithm = match qualified_field(fields, "conformance", "algorithm")? {
        Datum::Symbol(value) => value.clone(),
        _ => {
            return Err(ProjectionAdmissionProtocolError::new(
                "projection admission content algorithm differs",
            ));
        }
    };
    let bytes = match qualified_field(fields, "conformance", "digest")? {
        Datum::Bytes(value) if value.len() == 32 => <[u8; 32]>::try_from(value.as_slice())
            .map_err(|_| {
                ProjectionAdmissionProtocolError::new("projection admission content digest differs")
            })?,
        _ => {
            return Err(ProjectionAdmissionProtocolError::new(
                "projection admission content digest differs",
            ));
        }
    };
    Ok(ContentId::from_bytes(algorithm, bytes))
}

fn qualified_field<'a>(
    fields: &'a [(Symbol, Datum)],
    namespace: &str,
    name: &str,
) -> Result<&'a Datum, ProjectionAdmissionProtocolError> {
    let key = Symbol::qualified(namespace, name);
    let mut values = fields.iter().filter(|(candidate, _)| candidate == &key);
    let value = values.next().map(|(_, value)| value).ok_or_else(|| {
        ProjectionAdmissionProtocolError::new("projection admission content field is absent")
    })?;
    if values.next().is_some() {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission content field is duplicated",
        ));
    }
    Ok(value)
}

fn policy_set(value: &Datum) -> Result<BTreeSet<ContentId>, ProjectionAdmissionProtocolError> {
    let Datum::Vector(values) = value else {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission policy set differs",
        ));
    };
    let policies = values
        .iter()
        .map(content_id)
        .collect::<Result<BTreeSet<_>, _>>()?;
    require_nonempty_set(&policies, "projection admission policy set is empty")?;
    if policies.len() != values.len() {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission policy set contains duplicates",
        ));
    }
    Ok(policies)
}

fn ordered_ids(value: &Datum) -> Result<Vec<ContentId>, ProjectionAdmissionProtocolError> {
    let Datum::Vector(values) = value else {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission owner sequence differs",
        ));
    };
    let ids = values
        .iter()
        .map(content_id)
        .collect::<Result<Vec<_>, _>>()?;
    if ids.is_empty() || ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ProjectionAdmissionProtocolError::new(
            "projection admission owner sequence is empty, duplicated, or unordered",
        ));
    }
    Ok(ids)
}

fn require_nonempty_set<T>(
    set: &BTreeSet<T>,
    detail: &str,
) -> Result<(), ProjectionAdmissionProtocolError> {
    if set.is_empty() {
        Err(ProjectionAdmissionProtocolError::new(detail))
    } else {
        Ok(())
    }
}

fn ids_datum(ids: &BTreeSet<ContentId>) -> Datum {
    Datum::Vector(ids.iter().map(content_id_datum).collect())
}

fn ordered_ids_datum(ids: &[ContentId]) -> Datum {
    Datum::Vector(ids.iter().map(content_id_datum).collect())
}

fn decimal_u64(value: &Datum, detail: &str) -> Result<u64, ProjectionAdmissionProtocolError> {
    match value {
        Datum::String(value)
            if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            value
                .parse::<u64>()
                .map_err(|_| ProjectionAdmissionProtocolError::new(detail))
        }
        _ => Err(ProjectionAdmissionProtocolError::new(detail)),
    }
}

fn protocol(error: impl fmt::Display) -> ProjectionAdmissionProtocolError {
    ProjectionAdmissionProtocolError::new(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_lib_server::encode_transport_frame;
    use std::sync::{Arc, RwLock};

    fn id(value: &str) -> ContentId {
        Datum::String(value.to_owned())
            .content_id()
            .expect("content identity")
    }

    fn message(frame: &ServerFrame) -> NativeDescriptorMessage {
        NativeDescriptorMessage {
            bytes: encode_transport_frame(frame).expect("transport frame"),
            descriptors: Vec::new(),
        }
    }

    fn current(offer: &ParentOffer) -> ChildCurrent {
        let mut providers = vec![id("provider-a"), id("provider-b")];
        providers.sort();
        let mut graphs = vec![id("graph-a"), id("graph-b")];
        graphs.sort();
        let mut current = ChildCurrent {
            attempt: offer.attempt.clone(),
            challenge: offer.challenge.clone(),
            implementation: offer.implementation.clone(),
            complete_input_closure: offer.complete_input_closure.clone(),
            capture: offer.expected_capture.clone(),
            policies: offer.policies.clone(),
            composition: id("pending"),
            configuration_shape: id("shape"),
            providers,
            baseline: id("baseline"),
            graphs,
            closure: id("closure"),
        };
        current.composition = child_composition_id(&current).expect("composition");
        current
    }

    fn offer() -> ParentOffer {
        ParentOffer::new(
            KernelObjectIdentity {
                device: 3,
                inode: 7,
            },
            id("attempt"),
            id("challenge"),
            id("implementation"),
            id("inputs"),
            id("capture"),
            BTreeSet::from([id("policy")]),
        )
        .expect("offer")
    }

    #[test]
    fn every_statement_round_trips_through_one_canonical_codec() {
        let offer = offer();
        assert_eq!(
            ParentOffer::from_message(message(&offer.frame().unwrap())).unwrap(),
            offer
        );
        let current = current(&offer);
        assert_eq!(
            ChildCurrent::from_message(message(&current.frame().unwrap())).unwrap(),
            current
        );
        let admitted = Admitted::new(
            offer.attempt.clone(),
            offer.challenge.clone(),
            current.identity().unwrap(),
            id("receipt"),
            offer.implementation.clone(),
            offer.complete_input_closure.clone(),
        );
        assert_eq!(
            Admitted::from_message(message(&admitted.frame().unwrap())).unwrap(),
            admitted
        );
        let ack = AdmissionAcknowledged::new(&admitted);
        assert_eq!(
            AdmissionAcknowledged::from_message(message(&ack.frame().unwrap())).unwrap(),
            ack
        );
        let activated = Activated::new(
            admitted.attempt.clone(),
            admitted.challenge.clone(),
            admitted.child_current.clone(),
            admitted.receipt.clone(),
            ack.identity().unwrap(),
        );
        assert_eq!(
            Activated::from_message(message(&activated.frame().unwrap())).unwrap(),
            activated
        );
        let complete = ActivationComplete::new(&activated).unwrap();
        assert_eq!(
            ActivationComplete::from_message(message(&complete.frame().unwrap())).unwrap(),
            complete
        );
    }

    #[test]
    fn envelope_payload_descriptor_and_replay_substitution_refuse() {
        let current = current(&offer());
        let exact = current.frame().unwrap();
        let mut wrong_correlation = exact.clone();
        wrong_correlation.correlate = Some(5);
        assert!(ChildCurrent::from_message(message(&wrong_correlation)).is_err());
        let mut wrong_payload = exact.clone();
        wrong_payload.payload.push(0);
        assert!(ChildCurrent::from_message(message(&wrong_payload)).is_err());
        let with_descriptor = NativeDescriptorMessage {
            bytes: encode_transport_frame(&exact).unwrap(),
            descriptors: vec![std::fs::File::open("/dev/null").unwrap()],
        };
        assert!(ChildCurrent::from_message(with_descriptor).is_err());

        let operation = OperationStatement::new(
            id("attempt"),
            4,
            id("policy"),
            id("implementation"),
            id("inputs"),
            id("receipt"),
        );
        let begin = operation.frame(0, FrameKind::Request, "begin").unwrap();
        assert_eq!(
            OperationStatement::from_begin(message(&begin), 4).unwrap(),
            operation
        );
        assert!(OperationStatement::from_begin(message(&begin), 3).is_err());
        let substituted = OperationStatement::new(
            id("attempt"),
            4,
            id("policy"),
            id("implementation"),
            id("inputs"),
            id("other-receipt"),
        );
        assert_ne!(
            substituted
                .frame(0, FrameKind::Request, "begin")
                .unwrap()
                .payload,
            begin.payload
        );
    }

    struct TestOwnerEvidence {
        value: Datum,
        current: Arc<RwLock<bool>>,
    }

    impl TestOwnerEvidence {
        fn new(role: &str, current: Arc<RwLock<bool>>) -> Self {
            Self {
                value: Datum::Node {
                    tag: Symbol::qualified("projection-test", role),
                    fields: vec![],
                },
                current,
            }
        }

        fn identity(&self) -> ContentId {
            self.value.content_id().expect("owner identity")
        }
    }

    impl ProjectionOwnedEvidence for TestOwnerEvidence {
        fn canonical(&self) -> Result<Datum, CliError> {
            self.reobserve_current()?;
            Ok(self.value.clone())
        }

        fn reobserve_current(&self) -> Result<(), CliError> {
            if *self.current.read().expect("current read") {
                Ok(())
            } else {
                Err(CliError::new("test projection owner is not current"))
            }
        }

        fn while_current(&self, action: &mut dyn FnMut()) -> Result<(), CliError> {
            let current = self.current.read().expect("current read");
            if !*current {
                return Err(CliError::new("test projection owner is not current"));
            }
            action();
            Ok(())
        }
    }

    struct TestShape(TestOwnerEvidence);

    impl ProjectionOwnedEvidence for TestShape {
        fn canonical(&self) -> Result<Datum, CliError> {
            self.0.canonical()
        }
        fn reobserve_current(&self) -> Result<(), CliError> {
            self.0.reobserve_current()
        }
        fn while_current(&self, action: &mut dyn FnMut()) -> Result<(), CliError> {
            self.0.while_current(action)
        }
    }

    impl ProjectionConfigurationShapeEvidence for TestShape {
        fn verify_configuration(&self, configuration: &Datum) -> Result<(), CliError> {
            self.reobserve_current()?;
            if configuration == &Datum::String("selected".to_owned()) {
                Ok(())
            } else {
                Err(CliError::new("test configuration was substituted"))
            }
        }
    }

    fn complete_composition(
        visitor: &mut ConcreteProjectionVisitor<'_>,
        current: &Arc<RwLock<bool>>,
        policy: &TestOwnerEvidence,
    ) -> Result<(), CliError> {
        visitor.configuration_shape(
            &Datum::String("selected".to_owned()),
            &TestShape(TestOwnerEvidence::new("shape", Arc::clone(current))),
        )?;
        visitor.provider_registration(&TestOwnerEvidence::new("provider", Arc::clone(current)))?;
        visitor.selected_policy(policy)?;
        visitor.baseline_membership(&TestOwnerEvidence::new("baseline", Arc::clone(current)))?;
        visitor.graph_owner(&TestOwnerEvidence::new("graph", Arc::clone(current)))?;
        visitor.federated_closure(&TestOwnerEvidence::new("closure", Arc::clone(current)))?;
        Ok(())
    }

    #[test]
    fn actual_c_observation_binds_full_composition_and_owner_currentness() {
        let current = Arc::new(RwLock::new(true));
        let policy = TestOwnerEvidence::new("policy", Arc::clone(&current));
        let policies = BTreeSet::from([policy.identity()]);
        let mut visitor = ConcreteProjectionVisitor::new(&policies);
        complete_composition(&mut visitor, &current, &policy).unwrap();
        assert!(visitor.snapshot().is_ok());

        let substituted = TestOwnerEvidence::new("substituted", Arc::clone(&current));
        let substituted_policies = BTreeSet::from([substituted.identity()]);
        let mut visitor = ConcreteProjectionVisitor::new(&substituted_policies);
        complete_composition(&mut visitor, &current, &policy).unwrap();
        assert!(visitor.snapshot().is_err());

        let mut visitor = ConcreteProjectionVisitor::new(&policies);
        *current.write().expect("current write") = false;
        assert!(visitor.selected_policy(&policy).is_err());
    }

    #[test]
    fn actual_c_composition_canonicalizes_provider_and_graph_order() {
        let current = Arc::new(RwLock::new(true));
        let policy = TestOwnerEvidence::new("policy", Arc::clone(&current));
        let policies = BTreeSet::from([policy.identity()]);
        let provider_a = TestOwnerEvidence::new("provider-a", Arc::clone(&current));
        let provider_b = TestOwnerEvidence::new("provider-b", Arc::clone(&current));
        let graph_a = TestOwnerEvidence::new("graph-a", Arc::clone(&current));
        let graph_b = TestOwnerEvidence::new("graph-b", Arc::clone(&current));

        let mut first = ConcreteProjectionVisitor::new(&policies);
        complete_composition(&mut first, &current, &policy).unwrap();
        first.provider_registration(&provider_a).unwrap();
        first.provider_registration(&provider_b).unwrap();
        first.graph_owner(&graph_a).unwrap();
        first.graph_owner(&graph_b).unwrap();

        let mut reversed = ConcreteProjectionVisitor::new(&policies);
        complete_composition(&mut reversed, &current, &policy).unwrap();
        reversed.provider_registration(&provider_b).unwrap();
        reversed.provider_registration(&provider_a).unwrap();
        reversed.graph_owner(&graph_b).unwrap();
        reversed.graph_owner(&graph_a).unwrap();
        assert_eq!(first.snapshot().unwrap(), reversed.snapshot().unwrap());
    }
}
