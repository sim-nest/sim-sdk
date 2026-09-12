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
mod revocation;

/// The 21 statically bound conformance pack entrypoints.
pub mod packs;

pub use authority::{
    OperationLocalAuthority, OperationLocalAuthorityError, OperationLocalOwnerControl,
    QualifiedOperationLocalReceipt, bind_operation_local_owner,
};
pub use catalog::{PackSpec, all_packs, find_pack};
pub use harness::{
    CheckObservation, MemorySubject, PackFailure, PackRequest, PackSubject, PackVerdict,
};
pub use revocation::{operation_local_policy_id, owner_revocation_source};
