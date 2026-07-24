#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn genai_feature_bundles_are_layered_for_base_local_and_provider_use() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "genai",
        &["agent", "bridge", "codec-json", "cookbook"],
    );
    assert_feature_includes(
        &features,
        "genai-local",
        &[
            "genai",
            "agent-runner-process",
            "agent-runner-ollama",
            "agent-runner-http",
        ],
    );
    assert_feature_includes(
        &features,
        "genai-provider",
        &["genai", "agent-runner-http-tls"],
    );

    for excluded in [
        "agent-runner-process",
        "agent-runner-ollama",
        "agent-runner-http",
        "agent-runner-http-tls",
    ] {
        assert!(
            !features["genai"].iter().any(|feature| feature == excluded),
            "genai should not directly include {excluded}"
        );
    }
}
