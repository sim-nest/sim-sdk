#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r36_daw_session_feature_is_opt_in_and_layered() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "daw-session",
        &["audio-graph-live", "plugin-core", "topology-core"],
    );
    assert!(
        !features["default"]
            .iter()
            .any(|feature| feature == "daw-session")
    );
}
