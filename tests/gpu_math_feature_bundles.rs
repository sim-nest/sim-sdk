#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn gpu_math_features_keep_modeled_default_separate_from_provider_runtimes() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "gpu-math",
        &[
            "compute-model",
            "compute-femm",
            "femm-prelude",
            "numbers-tensor-linalg",
            "cookbook",
        ],
    );
    assert_feature_includes(
        &features,
        "gpu-math-provider",
        &[
            "gpu-math",
            "compute-auto",
            "compute-wgpu",
            "compute-cuda",
            "compute-rocm",
        ],
    );

    for excluded in [
        "compute-auto",
        "compute-wgpu",
        "compute-cuda",
        "compute-rocm",
    ] {
        assert!(
            !features["gpu-math"]
                .iter()
                .any(|feature| feature == excluded),
            "gpu-math should not directly include {excluded}"
        );
        assert!(
            !features["default"]
                .iter()
                .any(|feature| feature == excluded),
            "default should not include {excluded}"
        );
    }
}
