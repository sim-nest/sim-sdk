//! Stable SDK surface for capturing behavior before a refactor and comparing it afterward.
//!
//! Enable the `standard-core` feature, declare a bounded [`ScenarioSpec`], and
//! record only canonical observations. Publish captures when a content-addressed
//! evidence identity is required; use [`compare_characterization_captures`] for
//! a strict comparison whose differences retain stable field paths and both
//! canonical values.

pub use sim_lib_standard_core::{
    BoundedLane, CanonicalFailure, CanonicalObservation, CanonicalOutcome, CaptureComparison,
    CaptureComparisonProjection, CaptureDifference, CharacterizationCapture, FailureLocation,
    ScenarioInput, ScenarioLimits, ScenarioObservationLane, ScenarioSpec,
    characterization_capture_kind, characterization_capture_predicate,
    compare_characterization_captures, publish_characterization_capture,
};
