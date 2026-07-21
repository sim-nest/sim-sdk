use super::support::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn watch_feature_installs_modeled_stack_and_keeps_hardware_opt_in() {
    let features = collect_feature_dependencies(include_str!("../../Cargo.toml"));
    assert_feature_includes(&features, "watch", &["watch-modeled"]);
    assert_feature_includes(
        &features,
        "watch-modeled",
        &[
            "device-reference",
            "dep:sim-lib-stream-wrist",
            "dep:sim-lib-view-wrist",
            "cookbook",
        ],
    );
    assert_feature_includes(
        &features,
        "watch-hardware",
        &["watch-modeled", "dep:sim-lib-stream-wristbridge"],
    );
}
