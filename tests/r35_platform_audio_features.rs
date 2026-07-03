#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r35_platform_audio_features_are_hardware_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "stream-asio",
        &["audio-graph-core", "stream-audio", "stream-host"],
    );
    assert_feature_includes(
        &features,
        "stream-coreaudio",
        &["audio-graph-core", "stream-audio", "stream-host"],
    );
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "stream-asio" || feature == "stream-coreaudio")
    );
}
