#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r29_audio_graph_live_feature_is_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "audio-graph-live",
        &["audio-graph-core", "stream-clock", "stream-host"],
    );
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "audio-graph-live")
    );
}
