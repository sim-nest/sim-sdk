//! Curated serial-music facade over the frozen `0.1.0` candidate crates.
//!
//! The SDK keeps this surface deliberately thin. The owner crates retain the
//! implementation, cookbook content, and domain-specific policy details.

/// Pitch-independent series calculus plus strict twelve-tone row analysis.
pub mod theory {
    pub use sim_lib_pitch_serial::{
        ROW_MATRIX_SIZE, RowFamily, RowFamilySet, RowLabelConvention, RowMatrix, RowOperation,
        ToneRow,
    };
    pub use sim_lib_serial_core::{
        AggregateRule, AggregateRuleKind, AlphabetId, FiniteAlphabet, SerialAlphabet, Series,
        SeriesTransform,
    };
}

/// Immutable serial plans, deployers, and strict realization inputs.
pub mod practice {
    pub use sim_lib_music_serial::{
        BuiltInPracticeRule, DeclaredWaivers, EventPlacement, OrdinalRef, PlannedSerialEvent,
        PracticeId, PracticeRuleId, RowInstanceId, SerialEventId, SerialOrigin, SerialPlan,
        SerialPractice, SerialRole, StrictEventSpec, StrictRealizationContext, StructuralLicense,
        StructuralReadingId, build_canon, realize_strict,
    };
}

/// Open adaptation and registry-backed realization selection.
pub mod adaptation {
    pub use sim_lib_music_serial::{
        MarkedChromaticInflectionRealizer, ModalDegreeCycleRealizer, NearestScaleToneRealizer,
        SerialRealizerRegistry, default_realizer_registry,
    };
}

/// Reversible additive completion over immutable serial plans.
pub mod completion {
    pub use sim_lib_music_serial::{
        AcceptedSerialAddition, AdditionKind, ChordAddition, CompletionCandidate,
        CompletionRequest, DoublingAddition, NoteAddition, OrnamentAddition, PedalAddition,
        PitchRangeConstraint, SerialCompletionAllowances, SerialCompletionError,
        SerialCompletionRequest, SerialCompletionResult, VoiceAddition, complete_serial,
    };
}

/// Focused rendering and export helpers for end-to-end serial workflows.
pub mod runtime {
    pub use sim_lib_music_serial::{
        SerialRenderOptions, lower_serial_score, render_serial_audition_score, write_serial_smf,
    };
}
