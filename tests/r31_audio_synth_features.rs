#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r31_audio_synth_feature_is_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "music-synth",
        &["audio-dsp", "pitch-core", "midi-core"],
    );
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "music-synth")
    );
}
