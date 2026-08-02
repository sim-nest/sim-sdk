#[path = "support/features.rs"]
mod features;

use features::{assert_feature_includes, collect_feature_dependencies};

#[test]
fn interference_features_preserve_granular_dependency_layers() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));

    assert_feature_includes(
        &features,
        "interference-core",
        &["dep:sim-lib-interference-core"],
    );
    assert_feature_includes(
        &features,
        "interference-solve",
        &["interference-core", "dep:sim-lib-interference-solve"],
    );
    assert_feature_includes(
        &features,
        "interference-runtime",
        &["interference-solve", "dep:sim-lib-interference-runtime"],
    );
    assert_feature_includes(
        &features,
        "interference-compute",
        &["interference-runtime", "dep:sim-lib-interference-compute"],
    );
    assert_feature_includes(
        &features,
        "view-interference",
        &["interference-runtime", "dep:sim-lib-view-interference"],
    );
    assert_feature_includes(
        &features,
        "interference",
        &["interference-compute", "view-interference"],
    );
}

#[test]
fn default_and_interference_bundles_are_hardware_independent() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    let hardware = [
        "compute-auto",
        "compute-cuda",
        "compute-rocm",
        "compute-wgpu",
    ];

    for bundle in ["default", "interference"] {
        for excluded in hardware {
            assert!(
                !features[bundle].iter().any(|feature| feature == excluded),
                "{bundle} should not directly include {excluded}"
            );
        }
    }
}

#[test]
fn interference_dependencies_use_the_frozen_versions_without_paths() {
    let cargo_toml = include_str!("../Cargo.toml");
    for package in [
        "sim-lib-interference-core",
        "sim-lib-interference-solve",
        "sim-lib-interference-runtime",
        "sim-lib-interference-compute",
        "sim-lib-view-interference",
    ] {
        let prefix = format!("{package} = ");
        let declaration = cargo_toml
            .lines()
            .find(|line| line.starts_with(&prefix))
            .unwrap_or_else(|| panic!("missing {package} dependency"));
        assert!(
            declaration.contains("version = \"0.1.0\""),
            "{package} must use the frozen 0.1.0 candidate: {declaration}"
        );
        assert!(
            !declaration.contains("path ="),
            "{package} must remain registry-resolvable: {declaration}"
        );
    }
}

#[cfg(feature = "interference")]
#[test]
fn facade_reexports_the_canonical_types_without_wrappers() {
    fn accepts_problem(_: sim::interference_core::InterferenceProblem) {}
    fn accepts_field(_: sim::interference_solve::HostPhasorField) {}
    fn accepts_study(_: sim::interference_runtime::StudyDescriptor) {}
    fn accepts_solver(_: sim::interference_compute::TensorStudySolver) {}
    fn accepts_surface(_: sim::view_interference::InterferenceSurfaceCodec) {}

    let _: fn(sim_lib_interference_core::InterferenceProblem) = accepts_problem;
    let _: fn(sim_lib_interference_solve::HostPhasorField) = accepts_field;
    let _: fn(sim_lib_interference_runtime::StudyDescriptor) = accepts_study;
    let _: fn(sim_lib_interference_compute::TensorStudySolver) = accepts_solver;
    let _: fn(sim_lib_view_interference::InterferenceSurfaceCodec) = accepts_surface;
}

#[cfg(all(feature = "core", feature = "shape", feature = "interference-runtime"))]
#[test]
fn standard_install_loads_interference_runtime_before_optional_compute() {
    use std::sync::Arc;

    let mut cx = sim::kernel::Cx::new(
        Arc::new(sim::kernel::EagerPolicy),
        Arc::new(sim::kernel::DefaultFactory),
    );
    sim::runtime::install_core_runtime(&mut cx);

    assert!(
        cx.registry()
            .lib(&sim::interference_runtime::interference_lib_symbol())
            .is_some(),
        "standard install should load InterferenceLib"
    );

    #[cfg(feature = "interference-compute")]
    assert!(
        cx.registry()
            .lib(&sim::interference_compute::interference_compute_lib_symbol())
            .is_some(),
        "interference-compute should load after its InterferenceLib dependency"
    );

    #[cfg(not(feature = "interference-compute"))]
    assert!(
        cx.registry()
            .lib(&sim::kernel::Symbol::qualified(
                "sim",
                "interference-compute"
            ))
            .is_none(),
        "interference-runtime alone should not install the compute provider"
    );
}
