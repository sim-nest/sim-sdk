use super::support::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn glasses_features_share_the_device_base_and_keep_provider_modes_explicit() {
    let features = collect_feature_dependencies(include_str!("../../Cargo.toml"));
    assert_feature_includes(&features, "glasses", &["glasses-modeled"]);
    assert_feature_includes(
        &features,
        "glasses-modeled",
        &[
            "device",
            "dep:sim-lib-stream-halo",
            "dep:sim-lib-stream-xr",
            "dep:sim-lib-view-spatial",
            "view-bridge",
            "web-bridge",
        ],
    );
    assert_feature_includes(
        &features,
        "glasses-viture",
        &["glasses-modeled", "dep:sim-lib-stream-viture"],
    );
    assert_feature_includes(&features, "glasses-halo", &["glasses-modeled"]);
}
