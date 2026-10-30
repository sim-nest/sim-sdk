//! Public, project-neutral conformance packs.
//!
//! Every checker entrypoint is present from the first release. A scope becomes
//! usable only when its scenario implementation is funded and shipped; all
//! other declared scopes return [`PackVerdict::UnimplementedPack`]. Packs are
//! pure over an injected [`PackSubject`] and never read files, spawn processes,
//! inspect environment variables, or store their own receipts.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod authority;
mod catalog;
mod harness;
#[cfg(target_os = "linux")]
mod projection_authority;
#[cfg(all(test, target_os = "linux"))]
mod projection_authority_tests;
#[cfg(target_os = "linux")]
mod projection_compiler_review;
#[cfg(target_os = "linux")]
mod projection_source_review;
mod revocation;

/// The 21 statically bound conformance pack entrypoints.
pub mod packs;

pub use authority::{
    MAX_OPERATION_LOCAL_EVIDENCE_BYTES, MAX_OPERATION_LOCAL_EVIDENCE_FACTS,
    MAX_OPERATION_LOCAL_SUPPORT_MEMBERS, OperationLocalAuthority, OperationLocalAuthorityError,
    OperationLocalComparisonIdentities, OperationLocalEvidence, OperationLocalEvidenceError,
    OperationLocalInvoker, OperationLocalOwner, OperationLocalSupportDefinition,
    OperationLocalSupportError, QualifiedOperationLocalCheck,
};
pub use catalog::{PackSpec, all_packs, find_pack};
pub use harness::{
    CheckObservation, MemorySubject, PackFailure, PackRequest, PackSubject, PackVerdict,
};
#[cfg(target_os = "linux")]
pub use projection_authority::{
    ActivatedProjectionAdmissionSession, ProjectionAdmissionAuthority,
    ProjectionAdmissionAuthorityError, ProjectionAdmissionOwner, ProjectionChildFactoryObservation,
    ProjectionNativeAdmission, ProjectionNativeAdmissionAttempt, ProjectionNativeAdmissionOffer,
    ProjectionNativeAdmissionPhase, ProjectionNativeOperationService, ProjectionParentPreparation,
    ProjectionParentReviewInputs, ProjectionParentReviewSession,
    ProjectionParentReviewSessionPhase, ProjectionSourceReviewPrerequisite,
    QualifiedProjectionAdmissionCheck, VerifiedProjectionProducerRecords,
};
#[cfg(target_os = "linux")]
pub use projection_compiler_review::{
    ProjectionCompilerReviewAuthority, ProjectionCompilerReviewError,
    QualifiedProjectionCompilerReview,
};
#[cfg(target_os = "linux")]
pub use projection_source_review::{
    ProjectionSourceForbiddenClass, ProjectionSourceReviewAuthority, ProjectionSourceReviewError,
    QualifiedProjectionSourceReview,
};
pub use revocation::{
    operation_local_policy_id, owner_revocation_source, projection_admission_policy_id,
};
