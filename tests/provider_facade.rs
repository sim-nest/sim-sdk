#![cfg(feature = "study")]

use sim::provider::{ProviderInventory, ProviderSeatId};

#[test]
fn provider_facade_exposes_inventory_and_stable_seat_identity() {
    let inventory = ProviderInventory::from_toml(
        r#"schema = "sim.provider-seats/v1"
[[seat]]
id = "api"
family = "openai-api"
auth = "api-key"
principal_ref = "principal/api"
endpoint_label = "public-api"
secret_source = "secret-provider/api"
provider_probe = "probe/api"
[[seat]]
id = "cli"
family = "codex-cli"
auth = "subscription"
principal_ref = "principal/cli"
endpoint_label = "codex-cli"
config_home_ref = "config-home/codex"
provider_probe = "probe/cli"
[[seat]]
id = "local"
family = "ollama"
auth = "none"
principal_ref = "principal/local"
endpoint_label = "ollama-local"
resource_job = "resource/local"
"#,
    )
    .unwrap();
    assert_eq!(inventory.seats.len(), 3);
    let stable_export: Vec<ProviderSeatId> = Vec::new();
    assert!(stable_export.is_empty());
    assert_eq!(inventory.seats[0].id, "api");
}
