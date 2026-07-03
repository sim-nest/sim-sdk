#[test]
fn workspace_does_not_ship_baked_sim_server_cli() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        !manifest.contains("\"crates/sim-server\""),
        "sim-server must not be a sim-sdk workspace member"
    );
    assert!(
        !std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("crates/sim-server/Cargo.toml")
            .exists(),
        "sim-sdk must not own a baked sim-server CLI crate"
    );
}

#[test]
fn server_surface_remains_a_feature_gated_library() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains("server = [\"dep:sim-lib-server\""),
        "the reusable server surface stays behind the sim feature map"
    );
}
