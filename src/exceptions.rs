//! Shared raised-exception contract for guest runtimes.
//!
//! A guest obtains a class from its declared class descriptor, constructs
//! [`Raised`], selects handlers through [`match_raised_class`], and stores
//! recursive guest relations as stable edges in [`ManagedException`].
//!
//! Conformance requires this facade to export the canonical raised envelope,
//! matcher, and managed relation adapter without defining a second exception
//! carrier.

pub use sim_lib_control::{
    BoundedSubclassOutcome, ClassMatchBudget, ClassMatchEvidence, ClassMatchOutcome,
    ExceptionGraphBudget, ExceptionGraphEdge, ExceptionGraphView, ManagedException, Raised,
    RaisedBrowseBudget, RaisedBrowseProjection, RaisedShape, match_raised_class,
};
