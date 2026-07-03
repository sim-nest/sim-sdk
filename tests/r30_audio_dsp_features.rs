#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r30_audio_dsp_feature_is_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(&features, "audio-dsp", &["audio-graph-core", "sound-core"]);
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "audio-dsp")
    );
}
