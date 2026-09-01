/// The executable specimen lives in `tests/hotload_generation.rs` so the same
/// public loop is compiled and run by both the SDK test gate and recipe verifier.
pub fn public_hotload_generation_loop() {
    assert_eq!(include_str!("generation-a/src/lib.rs").trim(), "pub const VALUE: &str = \"alpha\";");
    assert_eq!(include_str!("generation-b/src/lib.rs").trim(), "pub const VALUE: &str = \"beta\";");
}
