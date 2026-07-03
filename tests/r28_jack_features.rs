#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r28_jack_feature_is_hardware_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "stream-jack",
        &[
            "audio-graph-core",
            "stream-audio",
            "stream-clock",
            "stream-host",
        ],
    );
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "stream-jack")
    );
}
