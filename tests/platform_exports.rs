#![cfg(feature = "device")]

use sim::platform::{PlatformProviderAuthor, RequirementBuilder};

#[test]
fn exports_requirement_builder_and_provider_contract_without_capsules() {
    let requirements = RequirementBuilder::new()
        .require("platform/monotonic")
        .unwrap()
        .build();
    assert_eq!(requirements.len(), 1);
    let _ = std::any::type_name::<dyn PlatformProviderAuthor>();
    assert!(
        !include_str!("../Cargo.toml")
            .lines()
            .find(|line| line.starts_with("default ="))
            .unwrap()
            .contains("device")
    );
}

#[test]
fn exports_portable_loader_contract_without_a_concrete_capsule() {
    let _ = std::any::type_name::<dyn sim::loaders::LoaderPort>();
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("sim-platform-ubuntu-pc"));
    assert!(!manifest.contains("sim-platform-model"));
}
