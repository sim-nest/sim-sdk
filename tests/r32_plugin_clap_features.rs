#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r32_plugin_clap_features_are_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(&features, "plugin-core", &["audio-graph-core"]);
    assert_feature_includes(&features, "plugin-clap", &["plugin-core", "audio-dsp"]);
    assert!(
        !features["plugin-clap"]
            .iter()
            .any(|feature| feature == "music-synth"),
        "plugin-clap should not pull the music-facing synth surface"
    );
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "plugin-core" || feature == "plugin-clap")
    );
}
