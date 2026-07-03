#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r34_plugin_vst3_feature_is_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(&features, "plugin-vst3", &["plugin-core", "audio-dsp"]);
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "plugin-vst3")
    );
}
