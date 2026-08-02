pub fn index_discovery() -> Result<(), Box<dyn std::error::Error>> {
    let features = include_str!("../../../features.toml");
    let fragment = include_str!("../../../docs/generated/sim-index-fragment.sx");

    assert!(features.contains("feature/sim-sdk/serial-music-composition"));
    assert!(features.contains("route/compose-serial-music-from-sdk"));
    assert!(fragment.contains("recipe/sim-sdk/serial-music/row-matrix-analysis"));
    Ok(())
}
