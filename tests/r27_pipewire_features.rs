#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r27_pipewire_feature_is_hardware_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "stream-pipewire",
        &["audio-graph-core", "stream-audio", "stream-host"],
    );
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "stream-pipewire")
    );
}
