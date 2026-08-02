/// Deterministic numeric signal contracts and their loadable runtime library.
///
/// The SDK intentionally exposes the reusable transform boundary, not the
/// crate's implementation modules or cookbook helpers.
#[cfg(feature = "signal")]
pub mod signal {
    pub use sim_lib_numbers_signal::{
        Direction, LengthPolicy, Normalization, PaddingPolicy, PlacementPolicy, SignConvention,
        SignalBuffer, SignalError, SignalNumbersLib, SignalView, SpectrumPacking, Stride,
        TransformKind, TransformPlan, TransformPrecision, TransformReport, TransformResources,
        call_signal_transform, reference_dft, signal_transform_symbol, transform,
        transform_in_place,
    };
}

/// Bounded deterministic search controls, extension traits, results, and
/// receipts.
#[cfg(any(feature = "discrete-search", feature = "music-algorithms"))]
pub mod discrete_search {
    pub use sim_lib_discrete_search::{
        NeverInterrupt, SearchControl, SearchError, SearchInterrupt, SearchOrder, SearchProblem,
        SearchReceipt, SearchRun, SearchStatus, SearchStep, WorkCosts, solve,
    };
}

/// Exact musical ratio values, policies, analysis, and bounded relation
/// contracts.
///
/// Compatibility-only mean dialects stay crate-local to direct users of
/// `sim-lib-pitch-ratio`; the SDK facade presents the stable exact-ratio path.
#[cfg(any(feature = "pitch-ratio", feature = "music-algorithms"))]
pub mod pitch_ratio {
    pub use sim_lib_pitch_ratio::{
        ApproximationStrategy, FactorVector, PitchRatio, PitchRatioError, RatioApproximation,
        RatioChordReport, RatioCoverage, RatioPolicy, RatioRelation, RatioRelationPath,
        analyze_ratio_chord, analyze_ratio_chord_with_root, approximate_ratio,
        approximate_ratio_with_strategy, expand_ratio_relation_tree, rank_ratio, ratio_coverage,
        ratio_interval_matrix, root_normalized_tones, unrank_ratio,
    };
}

/// Exact consonance reports and identity-preserving additive completion.
#[cfg(any(feature = "music-consonance", feature = "music-inference"))]
pub mod music_consonance {
    pub use sim_lib_music_consonance::{
        Addition, AdditionKind, ChordAddition, CompletionConstraints, CompletionError,
        CompletionProvenance, CompletionRequest, CompletionResult, ConsonanceError,
        ConsonancePatch, ConsonancePolicy, ConsonanceReport, ConstraintError, DoublingAddition,
        MetricBounds, MetricFamily, MetricReport, MetricThreshold, MusicConsonanceLib,
        NoteAddition, OrnamentAddition, PatchError, PedalAddition, PitchRangeConstraint,
        PreservationConstraints, Provenance, ProvenanceKind, SoundingNote, SoundingWindow,
        StyleConstraints, TimeSpan, VoiceAddition, WindowSonance, apply_patch, complete_staff,
        evaluate, evaluate_midi_timeline, evaluate_staff, install_music_consonance_lib,
        music_consonance_evaluate_symbol, remove_patch, slice_sounding_windows, sounding_windows,
        staff_content_key,
    };
}

/// Exact counterpoint analysis, bounded generation, and stretto graph
/// contracts.
///
/// The SDK omits the generator's compiled CSP planner records; hosts compose
/// through rule data, generation policy, bounded search controls, and receipts.
#[cfg(any(feature = "music-counterpoint", feature = "music-inference"))]
pub mod music_counterpoint {
    pub use sim_lib_music_counterpoint::{
        AlignmentWindow, AnalysisProvenance, CadencePolicy, ContrapuntalForm,
        CounterpointGeneration, CounterpointGenerationPolicy, CounterpointGenerationReceipt,
        CounterpointGenerationResult, CounterpointReport, DissonanceContext, DissonanceRules,
        DiversityPolicy, DurationRules, GenerationError, IntervalRules, MetricEvidence, Motion,
        MotionDirection, MotionRules, MusicCounterpointLib, NoteEvidence, OverlapEvidence,
        PitchRange, RuleError, RuleSet, Species, StrettoChain, StrettoCluster,
        StrettoCompatibility, StrettoCouple, StrettoEntry, StrettoError, StrettoFusion,
        StrettoGraph, StrettoPolicy, StrettoRejection, StrettoTransform, TimeSpan, Violation,
        VoiceEvidence, VoiceRules, analyze_counterpoint, cluster_overlap, fuse_stretto_entries,
        generate_counterpoint, install_music_counterpoint_lib, materialize_transform,
        music_counterpoint_analyze_symbol, music_counterpoint_generate_symbol,
        music_stretto_graph_symbol, stretto_graph,
    };
}
