#![cfg(feature = "platform")]

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
            .contains("platform")
    );
}
