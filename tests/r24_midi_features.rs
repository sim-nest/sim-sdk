#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn r24_midi_backend_features_are_hardware_opt_in() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(&features, "midi-rtmidi", &["midi-core", "stream-host"]);
    assert_feature_includes(&features, "midi-ble", &["midi-rtmidi", "stream-host"]);
    assert!(
        !features["midi"]
            .iter()
            .any(|feature| feature == "midi-rtmidi")
    );
    assert!(!features["midi"].iter().any(|feature| feature == "midi-ble"));
}
