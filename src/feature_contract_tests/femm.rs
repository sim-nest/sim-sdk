use super::*;

#[test]
fn r10_femm_feature_implications_stay_wired() {
    let features = collect_feature_dependencies(include_str!("../../Cargo.toml"));
    assert_feature_includes(&features, "femm-geometry", &["femm-core"]);
    assert_feature_includes(&features, "femm-material", &["femm-core", "numbers-ad"]);
    assert_feature_includes(&features, "femm-mesh", &["femm-geometry", "femm-material"]);
    assert_feature_includes(&features, "femm-assembly", &["femm-space", "numbers-ad"]);
    assert_feature_includes(&features, "femm-solve", &["femm-core", "numbers-complex"]);
    assert_feature_includes(
        &features,
        "femm-flow",
        &[
            "femm-core",
            "femm-assembly",
            "femm-solve",
            "numbers-numeric",
        ],
    );
    assert_feature_includes(
        &features,
        "femm-physics",
        &["femm-core", "femm-assembly", "numbers-complex"],
    );
    assert_feature_includes(&features, "femm-post", &["femm-core", "femm-physics"]);
    assert_feature_includes(
        &features,
        "femm-field",
        &["femm-core", "femm-post", "numbers-func", "numbers-tensor"],
    );
    assert_feature_includes(
        &features,
        "femm-function",
        &["femm-core", "femm-field", "numbers-func"],
    );
    assert_feature_includes(
        &features,
        "femm-sensitiv",
        &["femm-core", "femm-function", "femm-solve", "numbers-ad"],
    );
    assert_feature_includes(
        &features,
        "femm-tape",
        &["femm-core", "femm-function", "femm-solve"],
    );
    assert_feature_includes(
        &features,
        "femm-ode",
        &["femm-core", "femm-tape", "numbers-rk", "numbers-tensor"],
    );
    assert_feature_includes(
        &features,
        "femm-codec",
        &[
            "femm-core",
            "femm-geometry",
            "femm-material",
            "femm-mesh",
            "femm-space",
            "femm-assembly",
            "femm-solve",
            "femm-flow",
            "femm-physics",
            "femm-post",
            "femm-field",
            "femm-function",
            "femm-sensitiv",
            "femm-tape",
            "femm-ode",
            "numbers-codec",
        ],
    );
    assert_feature_includes(&features, "femm-fixtures", &["femm-prelude"]);
    assert_feature_includes(
        &features,
        "femm-prelude",
        &[
            "femm-core",
            "femm-geometry",
            "femm-material",
            "femm-mesh",
            "femm-space",
            "femm-assembly",
            "femm-solve",
            "femm-flow",
            "femm-physics",
            "femm-post",
            "femm-field",
            "femm-function",
            "femm-sensitiv",
            "femm-tape",
            "femm-ode",
            "femm-codec",
            "numbers-prelude",
        ],
    );
}
