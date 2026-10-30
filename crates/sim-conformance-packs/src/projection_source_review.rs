//! Owner-issued review of the exact retained inputs to a native projection build.
//!
//! This owner is deliberately narrower than projection admission. It reviews one
//! held build while [`ProjectionBuildReviewInput::with_current`] keeps the real
//! source, toolchain, output, and artifact custody live. A successful review is
//! an input to admission; it is never admission by itself.
//!
//! Rust inputs are parsed and checked against a deliberately closed AST policy.
//! Unsupported dispatch, expansion, or attributes fail closed; hashes establish
//! identity and current custody, never source semantics.

use std::collections::BTreeSet;
use std::fmt;

use sim_conformance_core::{
    CheckArgument, CheckInputClosureId, CheckInvocation, CheckScopeId, CheckTemplate,
    CheckedSubjectId, CheckerBinding, CheckerReceipt, CheckerResultId,
    CheckerRevocationObservation, CommandId, ConformanceError, ConformancePackId,
    EnvironmentPolicyId, EvidenceGrade, EvidenceProvenanceId, EvidenceSetId, LiveCheckerAuthority,
    LiveCheckerError, LiveCheckerIssuer, LiveCheckerOwner, LiveCheckerReceipt, OutputShapeId,
    OwnerBindingId,
    PolicyId, ProofCodeId, WorkingDirectoryPolicyId, content_id_datum,
};
use sim_kernel::{ContentId, Datum, Symbol};
use sim_lib_hotload::{
    ConsumedBuildInputOrigin, ConsumedBuildInputsPrerequisiteKind, CurrentConsumedBuildInput,
    CurrentProjectionBuildReview, NativeArtifactTarget, NativeBuildRequest,
    ProjectionBuildReviewInput,
};
use syn::visit::Visit;

use crate::owner_revocation_source;

const REVIEW_SCOPE: &str = "boundary/projection-native-source-review";
const REVIEW_POLICY: &str = "sim-sdk/projection-native-source-review-v2";
const REVIEW_CHECKER: &str = "checker/projection-native-source-review";
const REVIEW_PACK: &str = "pack/projection-native-source-review-v2";

/// One source property that the narrow review policy refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionSourceForbiddenClass {
    /// Ambient filesystem, network, process, environment, or device access.
    AmbientIo,
    /// A native ABI or foreign-language input.
    ForeignCode,
    /// An unsafe Rust construct.
    UnsafeRust,
    /// Process-global state or a mutable global cache.
    GlobalState,
    /// A time, randomness, hash-order, or other nondeterministic input.
    Nondeterminism,
    /// Build-time execution without complete reads and selected semantics.
    BuildTimeCode,
}

/// Failure while reviewing or observing exact projection source support.
#[derive(Debug)]
pub enum ProjectionSourceReviewError {
    /// A canonical conformance identity could not be constructed.
    Conformance(ConformanceError),
    /// The neutral live checker owner refused or became unavailable.
    Authority(LiveCheckerError),
    /// The retained build or one of its owner-held resources is no longer current.
    BuildUnavailable(String),
    /// A bounded source or toolchain observation could not be completed.
    ObservationUnavailable(String),
    /// The build owner could not establish its complete consumed-input ledger.
    ConsumedInputsPrerequisite {
        /// Stable prerequisite category from the owning build service.
        kind: ConsumedBuildInputsPrerequisiteKind,
        /// Bounded owner diagnostic.
        detail: String,
    },
    /// A consumed input could not be assigned a sound role under this policy.
    InputClassificationUnavailable {
        /// Retained input owner.
        origin: ConsumedBuildInputOrigin,
        /// Path relative to that owner.
        path: String,
    },
    /// Valid Rust syntax used behavior outside the deliberately closed policy.
    SemanticAnalysisUnavailable {
        /// Exact retained input path.
        path: String,
        /// Stable reason this policy cannot establish semantics.
        detail: &'static str,
    },
    /// A relevant source construct is outside the selected deterministic policy.
    Forbidden {
        /// Refused property class.
        class: ProjectionSourceForbiddenClass,
        /// Exact source-relative specimen path.
        path: String,
        /// Stable diagnostic identifying the rejected construct.
        detail: &'static str,
    },
    /// A reviewed subject, invocation, receipt, or retained input was substituted.
    Substituted(&'static str),
}

impl fmt::Display for ProjectionSourceReviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conformance(error) => error.fmt(formatter),
            Self::Authority(error) => error.fmt(formatter),
            Self::BuildUnavailable(detail) | Self::ObservationUnavailable(detail) => {
                detail.fmt(formatter)
            }
            Self::ConsumedInputsPrerequisite { kind, detail } => write!(
                formatter,
                "consumed-build-input prerequisite {kind:?}: {detail}"
            ),
            Self::InputClassificationUnavailable { origin, path } => write!(
                formatter,
                "consumed {origin:?} input has no selected-policy role: {path}"
            ),
            Self::SemanticAnalysisUnavailable { path, detail } => write!(
                formatter,
                "projection source semantics are not established for {path}: {detail}"
            ),
            Self::Forbidden {
                class,
                path,
                detail,
            } => write!(
                formatter,
                "projection source review refused {class:?} in {path}: {detail}"
            ),
            Self::Substituted(role) => {
                write!(formatter, "projection source review substituted {role}")
            }
        }
    }
}

impl std::error::Error for ProjectionSourceReviewError {}

impl From<ConformanceError> for ProjectionSourceReviewError {
    fn from(error: ConformanceError) -> Self {
        Self::Conformance(error)
    }
}

impl From<LiveCheckerError> for ProjectionSourceReviewError {
    fn from(error: LiveCheckerError) -> Self {
        Self::Authority(error)
    }
}

/// Sole strong owner for one installed projection source-review generation.
#[derive(Debug)]
pub(crate) struct ProjectionSourceReviewOwner {
    live: LiveCheckerOwner,
    issuer: LiveCheckerIssuer,
}

/// Weak handle for observing receipts from one exact review-owner generation.
#[derive(Clone, Debug)]
pub struct ProjectionSourceReviewAuthority {
    live: LiveCheckerAuthority,
}

/// Non-serializable support for one exact, still-held projection build.
///
/// The neutral receipt is necessary but insufficient. Verification also enters
/// the retained build callback again and recomputes the exact reviewed closure.
pub struct QualifiedProjectionSourceReview {
    live: LiveCheckerReceipt,
    build: ProjectionBuildReviewInput,
    snapshot: ReviewedProjectionSource,
}

impl fmt::Debug for QualifiedProjectionSourceReview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QualifiedProjectionSourceReview")
            .field("invocation", self.live.invocation().id())
            .field("build_receipt", &self.snapshot.build_receipt)
            .finish_non_exhaustive()
    }
}

impl ProjectionSourceReviewOwner {
    /// Boots the exact installed review binding and policy generation.
    pub(crate) fn boot(generation: ContentId) -> Result<Self, ProjectionSourceReviewError> {
        let (live, issuer) =
            LiveCheckerOwner::boot(generation, source_review_binding()?, policy_id()?);
        Ok(Self { live, issuer })
    }

    /// Returns a weak observation handle which cannot keep this owner alive.
    #[must_use]
    pub(crate) fn authority(&self) -> ProjectionSourceReviewAuthority {
        ProjectionSourceReviewAuthority {
            live: self.live.authority(),
        }
    }

    /// Reviews and issues support only while the actual build inputs remain held.
    ///
    /// The review consumes the build owner's canonical input ledger through the
    /// same guarded callback. Incomplete Cargo records, untraced build scripts,
    /// native inputs, or lost retained resources remain typed prerequisites;
    /// readable mount identities never substitute for exact consumed bytes.
    pub(crate) fn review_and_issue(
        &self,
        build: &ProjectionBuildReviewInput,
    ) -> Result<QualifiedProjectionSourceReview, ProjectionSourceReviewError> {
        let snapshot = observe_and_review(build)?;
        let binding = source_review_binding()?;
        let invocation = source_review_invocation(&binding, &snapshot)?;
        self.live.mark_current(&invocation)?;
        let live = self.issuer.issue(
            invocation,
            result_id(&snapshot)?,
            EvidenceGrade::Reproducible,
            provenance_id(&snapshot)?,
            support_id(&snapshot)?,
        )?;
        let qualified = QualifiedProjectionSourceReview {
            live,
            build: build.clone(),
            snapshot,
        };
        qualified.verify_current(&ProjectionSourceReviewAuthority {
            live: self.live.authority(),
        })?;
        Ok(qualified)
    }

    /// Revokes only this exact reviewed build invocation.
    pub(crate) fn revoke(
        &self,
        reviewed: &QualifiedProjectionSourceReview,
    ) -> Result<(), ProjectionSourceReviewError> {
        self.live.revoke(reviewed.live.invocation())?;
        Ok(())
    }
}

impl ProjectionSourceReviewAuthority {
    /// Confirms that the exact selected review owner remains alive.
    pub fn ensure_live(&self) -> Result<(), ProjectionSourceReviewError> {
        self.live.ensure_live()?;
        Ok(())
    }
}

impl QualifiedProjectionSourceReview {
    /// Returns whether this support retains the exact supplied build owner.
    ///
    /// This is deliberately crate-private: downstream callers use the
    /// parent-issued session, not a descriptive build identity, to join A to
    /// native admission.
    #[must_use]
    pub(crate) fn shares_build_owner(&self, expected: &ProjectionBuildReviewInput) -> bool {
        self.build.shares_owner_with(expected)
    }

    /// Returns the exact native source-review invocation.
    pub fn invocation(&self) -> &CheckInvocation {
        self.live.invocation()
    }

    /// Returns the exact reviewed build receipt identity.
    pub const fn build_receipt(&self) -> &ContentId {
        &self.snapshot.build_receipt
    }

    /// Returns the reviewed subject identity for an exact admission join.
    pub fn subject(&self) -> &CheckedSubjectId {
        self.live.invocation().subject()
    }

    /// Returns the immutable owner-issued source-review receipt.
    #[must_use]
    pub const fn receipt(&self) -> &CheckerReceipt {
        self.live.receipt()
    }

    /// Returns the receipt-bound issue-time revocation observation.
    #[must_use]
    pub const fn issue_observation(&self) -> &CheckerRevocationObservation {
        self.live.issue_observation()
    }

    /// Recomputes input support and obtains current revocation from its owner.
    pub fn verify_current(
        &self,
        authority: &ProjectionSourceReviewAuthority,
    ) -> Result<(), ProjectionSourceReviewError> {
        self.live.verify_current(&authority.live)?;
        let current = observe_and_review(&self.build)?;
        if current != self.snapshot {
            return Err(ProjectionSourceReviewError::Substituted("build inputs"));
        }
        if self.live.invocation() != &source_review_invocation(self.live.binding(), &current)? {
            return Err(ProjectionSourceReviewError::Substituted("invocation"));
        }
        Ok(())
    }

    /// Runs an admission join while both receipt and build custody stay current.
    pub fn while_current<T>(
        &self,
        authority: &ProjectionSourceReviewAuthority,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionSourceReviewError> {
        let mut action = Some(action);
        let nested = self.live.while_current(&authority.live, || {
            self.build.with_current(|current| {
                let observed = review_current(current)?;
                if observed != self.snapshot {
                    return Err(ProjectionSourceReviewError::Substituted("build inputs"));
                }
                Ok(action.take().expect("review action runs once")())
            })
        })?;
        nested.map_err(|error| ProjectionSourceReviewError::BuildUnavailable(error.to_string()))?
    }

    /// Runs an admission join only for the exact build owner reviewed here.
    ///
    /// Equal receipts and input hashes cannot substitute another retained owner.
    pub fn while_current_for_build<T>(
        &self,
        authority: &ProjectionSourceReviewAuthority,
        expected_build: &ProjectionBuildReviewInput,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionSourceReviewError> {
        if !self.build.shares_owner_with(expected_build) {
            return Err(ProjectionSourceReviewError::Substituted("build owner"));
        }
        self.while_current(authority, action)
    }

    /// Runs an action while checker liveness and the ALREADY-BORROWED current
    /// build review stay current, without itself entering the retained build's
    /// currentness guard.
    ///
    /// Callers that also need [`QualifiedProjectionCompilerReview`]'s own
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
        authority: &ProjectionSourceReviewAuthority,
        current: &CurrentProjectionBuildReview<'_>,
        action: impl FnOnce() -> T,
    ) -> Result<T, ProjectionSourceReviewError> {
        let mut action = Some(action);
        self.live.while_current(&authority.live, || {
            let observed = review_current(current)?;
            if observed != self.snapshot {
                return Err(ProjectionSourceReviewError::Substituted("build inputs"));
            }
            Ok(action.take().expect("review action runs once")())
        })?
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReviewedProjectionSource {
    request: ContentId,
    invocation: ContentId,
    readable_inputs: ContentId,
    consumed_inputs: ContentId,
    inputs: Vec<ReviewedConsumedInput>,
    artifact: ContentId,
    build_receipt: ContentId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReviewedInputRole {
    RustSource,
    CargoManifest,
    ResolverLock,
    CargoConfiguration,
    CompilerDependencyRecord,
    CompilerUnitOutput,
    ToolchainTrustRoot,
}

impl ReviewedInputRole {
    const fn label(self) -> &'static str {
        match self {
            Self::RustSource => "rust-source",
            Self::CargoManifest => "cargo-manifest",
            Self::ResolverLock => "resolver-lock",
            Self::CargoConfiguration => "cargo-configuration",
            Self::CompilerDependencyRecord => "compiler-dependency-record",
            Self::CompilerUnitOutput => "compiler-unit-output",
            Self::ToolchainTrustRoot => "toolchain-trust-root",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReviewedConsumedInput {
    origin: ConsumedBuildInputOrigin,
    immutable_resource: Option<(String, String)>,
    path: String,
    content: ContentId,
    role: ReviewedInputRole,
}

fn observe_and_review(
    build: &ProjectionBuildReviewInput,
) -> Result<ReviewedProjectionSource, ProjectionSourceReviewError> {
    build
        .with_current(review_current)
        .map_err(|error| ProjectionSourceReviewError::BuildUnavailable(error.to_string()))?
}

fn review_current(
    current: &CurrentProjectionBuildReview<'_>,
) -> Result<ReviewedProjectionSource, ProjectionSourceReviewError> {
    let ledger = current
        .consumed_build_inputs()
        .map_err(consumed_inputs_prerequisite)?;
    if ledger.build_receipt() != &current.candidate().build_receipt {
        return Err(ProjectionSourceReviewError::Substituted(
            "consumed-input build receipt",
        ));
    }
    let mut inputs = Vec::with_capacity(ledger.input_count());
    let mut review_failure = None;
    current
        .visit_consumed_build_inputs(|input| {
            if review_failure.is_some() {
                return;
            }
            match review_consumed_input(&input) {
                Ok(reviewed) => inputs.push(reviewed),
                Err(error) => review_failure = Some(error),
            }
        })
        .map_err(build_unavailable)?;
    if let Some(error) = review_failure {
        return Err(error);
    }
    if inputs.len() != ledger.input_count() {
        return Err(ProjectionSourceReviewError::Substituted(
            "consumed-input count",
        ));
    }
    ensure_complete_selected_inputs(current.request(), &inputs)?;
    let request = current.request();
    Ok(ReviewedProjectionSource {
        request: request_datum(request)?
            .content_id()
            .map_err(|_| ConformanceError::NoncanonicalDatum)?,
        invocation: current
            .invocation()
            .binding_datum()
            .content_id()
            .map_err(|_| ConformanceError::NoncanonicalDatum)?,
        readable_inputs: current
            .readable_input_closure()
            .content_id()
            .map_err(|_| ConformanceError::NoncanonicalDatum)?,
        consumed_inputs: ledger.semantic_identity().clone(),
        inputs,
        artifact: current.candidate().content.content_id().clone(),
        build_receipt: current.candidate().build_receipt.content_id().clone(),
    })
}

fn review_consumed_input(
    input: &CurrentConsumedBuildInput<'_>,
) -> Result<ReviewedConsumedInput, ProjectionSourceReviewError> {
    review_retained_input(
        input.origin(),
        input.immutable_resource_binding(),
        input.path(),
        input.content(),
        input.bytes(),
        input.is_compiler_output(),
    )
}

fn review_retained_input(
    origin: ConsumedBuildInputOrigin,
    immutable_resource: Option<(&str, &str)>,
    path: &[String],
    content: &ContentId,
    bytes: &[u8],
    compiler_output: bool,
) -> Result<ReviewedConsumedInput, ProjectionSourceReviewError> {
    if (origin == ConsumedBuildInputOrigin::ImmutableResource) != immutable_resource.is_some() {
        return Err(ProjectionSourceReviewError::Substituted(
            "immutable consumed-input owner binding",
        ));
    }
    if path.is_empty()
        || path
            .iter()
            .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('/'))
    {
        return Err(
            ProjectionSourceReviewError::InputClassificationUnavailable {
                origin,
                path: path.join("/"),
            },
        );
    }
    let path = path.join("/");
    let role = classify_and_review_input(origin, &path, bytes, compiler_output)?;
    Ok(ReviewedConsumedInput {
        origin,
        immutable_resource: immutable_resource
            .map(|(guest_path, source)| (guest_path.to_owned(), source.to_owned())),
        path,
        content: content.clone(),
        role,
    })
}

fn classify_and_review_input(
    origin: ConsumedBuildInputOrigin,
    path: &str,
    bytes: &[u8],
    compiler_output: bool,
) -> Result<ReviewedInputRole, ProjectionSourceReviewError> {
    if compiler_output {
        if origin != ConsumedBuildInputOrigin::Workspace || bytes.is_empty() {
            return Err(
                ProjectionSourceReviewError::InputClassificationUnavailable {
                    origin,
                    path: path.into(),
                },
            );
        }
        return Ok(ReviewedInputRole::CompilerUnitOutput);
    }
    if path.ends_with(".rs") {
        if path.split('/').next_back() == Some("build.rs") {
            return Err(forbidden(
                ProjectionSourceForbiddenClass::BuildTimeCode,
                path.into(),
                "build-script source requires an owner-issued read trace",
            ));
        }
        review_rust_source(path, bytes)?;
        return Ok(ReviewedInputRole::RustSource);
    }
    if path.split('/').next_back() == Some("Cargo.toml") {
        require_utf8_configuration(path, bytes)?;
        return Ok(ReviewedInputRole::CargoManifest);
    }
    if path.split('/').next_back() == Some("Cargo.lock") {
        require_utf8_configuration(path, bytes)?;
        return Ok(ReviewedInputRole::ResolverLock);
    }
    if path.ends_with(".cargo/config") || path.ends_with(".cargo/config.toml") {
        require_utf8_configuration(path, bytes)?;
        return Ok(ReviewedInputRole::CargoConfiguration);
    }
    if origin == ConsumedBuildInputOrigin::Workspace && path.ends_with(".d") {
        require_utf8_configuration(path, bytes)?;
        return Ok(ReviewedInputRole::CompilerDependencyRecord);
    }
    if matches!(
        origin,
        ConsumedBuildInputOrigin::Toolchain | ConsumedBuildInputOrigin::ImmutableResource
    ) {
        return Ok(ReviewedInputRole::ToolchainTrustRoot);
    }
    if origin == ConsumedBuildInputOrigin::Source {
        return Err(forbidden(
            ProjectionSourceForbiddenClass::AmbientIo,
            path.into(),
            "consumed non-Rust source input is outside the selected policy",
        ));
    }
    Err(
        ProjectionSourceReviewError::InputClassificationUnavailable {
            origin,
            path: path.into(),
        },
    )
}

fn require_utf8_configuration(path: &str, bytes: &[u8]) -> Result<(), ProjectionSourceReviewError> {
    std::str::from_utf8(bytes).map_err(|_| {
        ProjectionSourceReviewError::ObservationUnavailable(format!(
            "consumed configuration is not UTF-8: {path}"
        ))
    })?;
    Ok(())
}

fn ensure_complete_selected_inputs(
    request: &NativeBuildRequest,
    inputs: &[ReviewedConsumedInput],
) -> Result<(), ProjectionSourceReviewError> {
    let mut identities = BTreeSet::new();
    for input in inputs {
        if !identities.insert((
            origin_label(input.origin),
            input.immutable_resource.as_ref(),
            input.path.as_str(),
        )) {
            return Err(ProjectionSourceReviewError::Substituted(
                "duplicate consumed input",
            ));
        }
    }
    let selected_manifest = inputs.iter().any(|input| {
        input.origin == ConsumedBuildInputOrigin::Source
            && input.path == request.manifest
            && input.role == ReviewedInputRole::CargoManifest
    });
    let selected_lock = inputs.iter().any(|input| {
        input.origin == ConsumedBuildInputOrigin::Source
            && input.path == "Cargo.lock"
            && input.role == ReviewedInputRole::ResolverLock
    });
    let rust_source = inputs
        .iter()
        .any(|input| input.role == ReviewedInputRole::RustSource);
    if !selected_manifest || !selected_lock {
        return Err(ProjectionSourceReviewError::ConsumedInputsPrerequisite {
            kind: ConsumedBuildInputsPrerequisiteKind::ResolverConfiguration,
            detail: "ledger omits the selected source manifest or resolver lock".into(),
        });
    }
    if !rust_source {
        return Err(ProjectionSourceReviewError::ConsumedInputsPrerequisite {
            kind: ConsumedBuildInputsPrerequisiteKind::CompilerDependencyRecord,
            detail: "ledger identifies no consumed Rust implementation input".into(),
        });
    }
    Ok(())
}

fn consumed_inputs_prerequisite(
    prerequisite: &sim_lib_hotload::ConsumedBuildInputsPrerequisite,
) -> ProjectionSourceReviewError {
    ProjectionSourceReviewError::ConsumedInputsPrerequisite {
        kind: prerequisite.kind(),
        detail: prerequisite.detail().into(),
    }
}

fn origin_label(origin: ConsumedBuildInputOrigin) -> &'static str {
    match origin {
        ConsumedBuildInputOrigin::Source => "source",
        ConsumedBuildInputOrigin::Toolchain => "toolchain",
        ConsumedBuildInputOrigin::ImmutableResource => "immutable-resource",
        ConsumedBuildInputOrigin::Workspace => "workspace",
    }
}

fn review_rust_source(path: &str, bytes: &[u8]) -> Result<(), ProjectionSourceReviewError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ProjectionSourceReviewError::ObservationUnavailable(format!(
            "consumed Rust source is not UTF-8: {path}"
        ))
    })?;
    let syntax = syn::parse_file(text).map_err(|_| {
        ProjectionSourceReviewError::SemanticAnalysisUnavailable {
            path: path.into(),
            detail: "consumed Rust input does not parse under the installed review grammar",
        }
    })?;
    let mut reviewer = RustPolicyReview { path, error: None };
    reviewer.visit_file(&syntax);
    reviewer.error.map_or(Ok(()), Err)
}

struct RustPolicyReview<'a> {
    path: &'a str,
    error: Option<ProjectionSourceReviewError>,
}

impl RustPolicyReview<'_> {
    fn unavailable(&mut self, detail: &'static str) {
        if self.error.is_none() {
            self.error = Some(ProjectionSourceReviewError::SemanticAnalysisUnavailable {
                path: self.path.into(),
                detail,
            });
        }
    }

    fn forbid(&mut self, class: ProjectionSourceForbiddenClass, detail: &'static str) {
        if self.error.is_none() {
            self.error = Some(forbidden(class, self.path.into(), detail));
        }
    }

    fn review_path(&mut self, path: &syn::Path) {
        let segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        if let Some((class, detail)) = forbidden_path(&segments) {
            self.forbid(class, detail);
        } else if path.leading_colon.is_some() || segments.len() > 1 {
            self.unavailable("qualified paths are outside the closed value/type namespace");
        }
    }

    fn review_macro(&mut self, node: &syn::Macro) {
        let name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        match name.as_deref() {
            Some("include" | "include_bytes" | "include_str" | "env" | "option_env") => self
                .forbid(
                    ProjectionSourceForbiddenClass::AmbientIo,
                    "compile-time ambient input macro",
                ),
            Some("print" | "println" | "eprint" | "eprintln" | "dbg") => self.forbid(
                ProjectionSourceForbiddenClass::AmbientIo,
                "ambient output macro",
            ),
            Some("asm" | "global_asm") => self.forbid(
                ProjectionSourceForbiddenClass::ForeignCode,
                "inline native assembly macro",
            ),
            Some("thread_local") => self.forbid(
                ProjectionSourceForbiddenClass::GlobalState,
                "thread-global state macro",
            ),
            _ => self.unavailable("macro expansion has no selected-policy semantic record"),
        }
    }
}

impl<'ast> Visit<'ast> for RustPolicyReview<'_> {
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if self.error.is_some() {
            return;
        }
        let safe_inert = [
            "doc",
            "must_use",
            "repr",
            "inline",
            "cold",
            "non_exhaustive",
        ];
        if safe_inert.iter().any(|name| node.path().is_ident(name)) {
            return;
        }
        if node.path().is_ident("proc_macro")
            || node.path().is_ident("proc_macro_attribute")
            || node.path().is_ident("proc_macro_derive")
            || node.path().is_ident("derive")
        {
            self.forbid(
                ProjectionSourceForbiddenClass::BuildTimeCode,
                "procedural or derived expansion lacks selected semantic evidence",
            );
        } else if node.path().is_ident("no_mangle") || node.path().is_ident("export_name") {
            self.forbid(
                ProjectionSourceForbiddenClass::ForeignCode,
                "externally selected symbol export",
            );
        } else {
            self.unavailable("attribute semantics are outside the selected policy");
        }
    }

    fn visit_item(&mut self, node: &'ast syn::Item) {
        if self.error.is_some() {
            return;
        }
        match node {
            syn::Item::Static(_) => self.forbid(
                ProjectionSourceForbiddenClass::GlobalState,
                "process-global static state",
            ),
            syn::Item::ForeignMod(_) => self.forbid(
                ProjectionSourceForbiddenClass::ForeignCode,
                "foreign module declaration",
            ),
            syn::Item::Union(_) => self.forbid(
                ProjectionSourceForbiddenClass::UnsafeRust,
                "union representation requires unsafe interpretation",
            ),
            syn::Item::ExternCrate(_) => {
                self.unavailable("extern-crate aliasing is outside the closed namespace")
            }
            syn::Item::Impl(_) | syn::Item::Trait(_) | syn::Item::TraitAlias(_) => {
                self.unavailable("trait dispatch and operator overloading are outside the policy")
            }
            syn::Item::Use(item) => {
                let segments = use_tree_prefix(&item.tree);
                if let Some((class, detail)) = forbidden_path(&segments) {
                    self.forbid(class, detail);
                } else {
                    self.unavailable("imports can alias behavior outside the closed namespace");
                }
            }
            syn::Item::Macro(item) => self.review_macro(&item.mac),
            syn::Item::Fn(item) if item.sig.unsafety.is_some() => self.forbid(
                ProjectionSourceForbiddenClass::UnsafeRust,
                "unsafe function declaration",
            ),
            syn::Item::Fn(item) if item.sig.abi.is_some() => self.forbid(
                ProjectionSourceForbiddenClass::ForeignCode,
                "foreign ABI function declaration",
            ),
            syn::Item::Verbatim(_) => {
                self.unavailable("unparsed item syntax is outside the selected policy")
            }
            _ => syn::visit::visit_item(self, node),
        }
    }

    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if self.error.is_some() {
            return;
        }
        match node {
            syn::Expr::Unsafe(_) => self.forbid(
                ProjectionSourceForbiddenClass::UnsafeRust,
                "unsafe expression",
            ),
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(path) = call.func.as_ref() {
                    self.review_path(&path.path);
                }
                if self.error.is_none() {
                    self.unavailable("function-call effects lack a selected call-graph proof");
                }
            }
            syn::Expr::MethodCall(_) => {
                self.unavailable("method dispatch lacks a selected call-graph proof")
            }
            syn::Expr::Macro(node) => self.review_macro(&node.mac),
            syn::Expr::Async(_) | syn::Expr::Await(_) | syn::Expr::Yield(_) => self.forbid(
                ProjectionSourceForbiddenClass::Nondeterminism,
                "unmediated asynchronous scheduling",
            ),
            syn::Expr::ForLoop(_) | syn::Expr::Index(_) | syn::Expr::Try(_) => {
                self.unavailable("implicit trait dispatch lacks a selected call-graph proof")
            }
            syn::Expr::Verbatim(_) => {
                self.unavailable("unparsed expression syntax is outside the selected policy")
            }
            _ => syn::visit::visit_expr(self, node),
        }
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        if self.error.is_some() {
            return;
        }
        self.review_path(node);
        if self.error.is_none() {
            syn::visit::visit_path(self, node);
        }
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if self.error.is_none() {
            self.review_macro(node);
        }
    }
}

fn use_tree_prefix(tree: &syn::UseTree) -> Vec<String> {
    let mut segments = Vec::new();
    let mut cursor = tree;
    while let syn::UseTree::Path(path) = cursor {
        segments.push(path.ident.to_string());
        cursor = path.tree.as_ref();
    }
    if let syn::UseTree::Name(name) = cursor {
        segments.push(name.ident.to_string());
    }
    segments
}

fn forbidden_path(segments: &[String]) -> Option<(ProjectionSourceForbiddenClass, &'static str)> {
    let starts = |prefix: &[&str]| {
        segments.len() >= prefix.len()
            && segments
                .iter()
                .map(String::as_str)
                .zip(prefix.iter().copied())
                .all(|(actual, expected)| actual == expected)
    };
    if ["fs", "net", "process", "io", "env"]
        .iter()
        .any(|area| starts(&["std", area]))
    {
        Some((
            ProjectionSourceForbiddenClass::AmbientIo,
            "ambient standard-library service",
        ))
    } else if starts(&["std", "thread"])
        || starts(&["std", "time"])
        || starts(&["rand"])
        || starts(&["getrandom"])
        || segments
            .iter()
            .any(|part| part == "HashMap" || part == "HashSet")
    {
        Some((
            ProjectionSourceForbiddenClass::Nondeterminism,
            "unmediated scheduling, time, randomness, or hash order",
        ))
    } else {
        None
    }
}

fn forbidden(
    class: ProjectionSourceForbiddenClass,
    path: String,
    detail: &'static str,
) -> ProjectionSourceReviewError {
    ProjectionSourceReviewError::Forbidden {
        class,
        path,
        detail,
    }
}
fn build_unavailable(error: sim_lib_hotload::BuildFailure) -> ProjectionSourceReviewError {
    ProjectionSourceReviewError::BuildUnavailable(error.to_string())
}

fn source_review_binding() -> Result<CheckerBinding, ConformanceError> {
    let scope = CheckScopeId::from_text(REVIEW_SCOPE)?;
    let template = CheckTemplate::new(
        "sim_conformance_packs::ProjectionParentPreparation::issue_reviews".into(),
        vec![
            CheckArgument::Literal(REVIEW_CHECKER.into()),
            CheckArgument::BindingSlot,
            CheckArgument::SubjectSlot,
            CheckArgument::ScopeSlot,
        ],
        WorkingDirectoryPolicyId::from_text("call/no-working-directory")?,
        EnvironmentPolicyId::from_text("call/retained-build-custody-v1")?,
        OutputShapeId::from_text("check/result-v1")?,
    )?;
    CheckerBinding::new(
        REVIEW_CHECKER.into(),
        OwnerBindingId::from_fields(vec![(
            Symbol::qualified("projection", "owner"),
            Datum::String("sim-sdk/sim-conformance-packs".into()),
        )])?,
        "sim_conformance_packs::ProjectionParentReviewSession/source".into(),
        vec![pack_id()?],
        OutputShapeId::from_text("check/receipt-v1")?,
        owner_revocation_source()?,
        CommandId::from_text(
            "cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo doc --workspace --no-deps && cargo clippy --workspace --all-features --all-targets -- -D warnings && cargo test --workspace --all-features",
        )?,
        CommandId::from_text("cargo run -p xtask -- simdoc --check")?,
        BTreeSet::from([scope]),
        template,
    )
}

fn source_review_invocation(
    binding: &CheckerBinding,
    snapshot: &ReviewedProjectionSource,
) -> Result<CheckInvocation, ConformanceError> {
    binding.instantiate(
        checker_code_id()?,
        pack_id()?,
        subject_id(snapshot)?,
        CheckScopeId::from_text(REVIEW_SCOPE)?,
        input_closure_id(snapshot)?,
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
            Datum::String(REVIEW_POLICY.into()),
        ),
        (
            Symbol::qualified("projection", "checker-code"),
            checker_code_id()?.to_datum(),
        ),
    ])
}

fn subject_id(snapshot: &ReviewedProjectionSource) -> Result<CheckedSubjectId, ConformanceError> {
    CheckedSubjectId::from_fields(vec![
        (
            Symbol::qualified("projection", "request"),
            content_id_datum(&snapshot.request),
        ),
        (
            Symbol::qualified("projection", "source"),
            implementation_datum(snapshot),
        ),
        (
            Symbol::qualified("projection", "artifact"),
            content_id_datum(&snapshot.artifact),
        ),
        (
            Symbol::qualified("projection", "build-receipt"),
            content_id_datum(&snapshot.build_receipt),
        ),
    ])
}

fn input_closure_id(
    snapshot: &ReviewedProjectionSource,
) -> Result<CheckInputClosureId, ConformanceError> {
    CheckInputClosureId::from_fields(vec![
        (
            Symbol::qualified("projection", "invocation"),
            content_id_datum(&snapshot.invocation),
        ),
        (
            Symbol::qualified("projection", "readable-inputs"),
            content_id_datum(&snapshot.readable_inputs),
        ),
        (
            Symbol::qualified("projection", "consumed-input-ledger"),
            content_id_datum(&snapshot.consumed_inputs),
        ),
        (
            Symbol::qualified("projection", "consumed-inputs"),
            consumed_inputs_datum(snapshot),
        ),
        (
            Symbol::qualified("projection", "artifact"),
            content_id_datum(&snapshot.artifact),
        ),
        (
            Symbol::qualified("projection", "build-receipt"),
            content_id_datum(&snapshot.build_receipt),
        ),
    ])
}

fn result_id(snapshot: &ReviewedProjectionSource) -> Result<CheckerResultId, ConformanceError> {
    CheckerResultId::from_fields(vec![
        (
            Symbol::qualified("projection", "verdict"),
            Datum::String("accepted".into()),
        ),
        (
            Symbol::qualified("projection", "policy"),
            policy_id()?.to_datum(),
        ),
        (
            Symbol::qualified("projection", "subject"),
            subject_id(snapshot)?.to_datum(),
        ),
    ])
}

fn provenance_id(
    snapshot: &ReviewedProjectionSource,
) -> Result<EvidenceProvenanceId, ConformanceError> {
    EvidenceProvenanceId::from_fields(vec![
        (
            Symbol::qualified("projection", "build-receipt"),
            content_id_datum(&snapshot.build_receipt),
        ),
        (
            Symbol::qualified("projection", "review-code"),
            checker_code_id()?.to_datum(),
        ),
    ])
}

fn support_id(snapshot: &ReviewedProjectionSource) -> Result<EvidenceSetId, ConformanceError> {
    EvidenceSetId::from_fields(vec![
        (
            Symbol::qualified("projection", "consumed-input-ledger"),
            content_id_datum(&snapshot.consumed_inputs),
        ),
        (
            Symbol::qualified("projection", "consumed-inputs"),
            consumed_inputs_datum(snapshot),
        ),
        (
            Symbol::qualified("projection", "implementation"),
            implementation_datum(snapshot),
        ),
    ])
}

fn implementation_datum(snapshot: &ReviewedProjectionSource) -> Datum {
    Datum::Vector(
        snapshot
            .inputs
            .iter()
            .filter(|input| input.role == ReviewedInputRole::RustSource)
            .map(|input| Datum::Node {
                tag: Symbol::qualified("projection", "reviewed-rust-input-v2"),
                fields: vec![
                    (Symbol::new("owner"), reviewed_owner_datum(input)),
                    (Symbol::new("path"), Datum::String(input.path.clone())),
                    (Symbol::new("content"), content_id_datum(&input.content)),
                ],
            })
            .collect(),
    )
}

fn consumed_inputs_datum(snapshot: &ReviewedProjectionSource) -> Datum {
    Datum::Vector(
        snapshot
            .inputs
            .iter()
            .map(|input| Datum::Node {
                tag: Symbol::qualified("projection", "reviewed-consumed-input-v2"),
                fields: vec![
                    (Symbol::new("owner"), reviewed_owner_datum(input)),
                    (Symbol::new("path"), Datum::String(input.path.clone())),
                    (Symbol::new("content"), content_id_datum(&input.content)),
                    (
                        Symbol::new("role"),
                        Datum::String(input.role.label().into()),
                    ),
                ],
            })
            .collect(),
    )
}

fn reviewed_owner_datum(input: &ReviewedConsumedInput) -> Datum {
    match &input.immutable_resource {
        Some((guest_path, source)) => Datum::Node {
            tag: Symbol::qualified("projection", "reviewed-immutable-owner-v1"),
            fields: vec![
                (Symbol::new("guest-path"), Datum::String(guest_path.clone())),
                (Symbol::new("source"), Datum::String(source.clone())),
            ],
        },
        None => Datum::Symbol(Symbol::qualified(
            "projection-input-owner",
            origin_label(input.origin),
        )),
    }
}

fn request_datum(request: &NativeBuildRequest) -> Result<Datum, ConformanceError> {
    let artifact = match &request.artifact {
        NativeArtifactTarget::DynamicLibrary { expected_library } => Datum::Node {
            tag: Symbol::qualified("projection", "dynamic-library-target-v1"),
            fields: vec![(
                Symbol::new("library"),
                Datum::Symbol(expected_library.clone()),
            )],
        },
        NativeArtifactTarget::Executable { binary } => Datum::Node {
            tag: Symbol::qualified("projection", "executable-target-v1"),
            fields: vec![(Symbol::new("binary"), Datum::String(binary.clone()))],
        },
    };
    Ok(Datum::Node {
        tag: Symbol::qualified("projection", "native-build-request-v1"),
        fields: vec![
            (
                Symbol::new("source"),
                Datum::String(request.source_mount.clone()),
            ),
            (
                Symbol::new("manifest"),
                Datum::String(request.manifest.clone()),
            ),
            (
                Symbol::new("package"),
                Datum::String(request.package.clone()),
            ),
            (
                Symbol::new("features"),
                Datum::Vector(
                    request
                        .features
                        .iter()
                        .cloned()
                        .map(Datum::String)
                        .collect(),
                ),
            ),
            (Symbol::new("artifact"), artifact),
            (
                Symbol::new("toolchain"),
                Datum::String(request.toolchain.content.clone()),
            ),
            (
                Symbol::new("cargo"),
                Datum::String(request.toolchain.cargo_program.clone()),
            ),
            (
                Symbol::new("environment"),
                Datum::Vector(
                    request
                        .toolchain
                        .environment
                        .iter()
                        .map(|(name, value)| Datum::Node {
                            tag: Symbol::qualified("projection", "environment-binding-v1"),
                            fields: vec![
                                (Symbol::new("name"), Datum::String(name.clone())),
                                (Symbol::new("value"), Datum::String(value.clone())),
                            ],
                        })
                        .collect(),
                ),
            ),
        ],
    })
}

#[cfg(test)]
#[path = "projection_source_review_tests.rs"]
mod tests;
