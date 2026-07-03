#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r33_plugin_lv2_feature_is_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(&features, "plugin-lv2", &["plugin-core", "audio-dsp"]);
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "plugin-lv2")
    );
}
