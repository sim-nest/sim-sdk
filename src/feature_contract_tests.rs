mod cfg_sentinels;
mod support;

use support::{
    assert_all_feature_metadata_has_no_invalid_dependency_warnings,
    assert_crate_cargo_tomls_do_not_contain, assert_dep_edges_reference_optional_dependencies,
    assert_feature_includes, collect_cfg_features, collect_declared_features,
    collect_feature_dependencies, collect_optional_dependencies, repo_root,
};

const PUBLIC_FACADE_ALIASES: &[(&str, &str)] = &[
    ("agent-runner-core", "lib_agent_runner_core"),
    ("agent-runner-http", "lib_agent_runner_http"),
    ("agent-runner-process", "lib_agent_runner_process"),
    ("discrete", "lib_discrete"),
    ("view", "lib_view"),
    ("view-agent", "lib_view_agent"),
    ("view-bridge", "lib_view_bridge"),
    ("view-codec", "lib_view_codec"),
    ("view-daw", "lib_view_daw"),
    ("view-doc", "lib_view_doc"),
    ("view-math", "lib_view_math"),
    ("web-layout", "lib_web_layout"),
    ("web-wasm-frame", "lib_view_wasm_frame"),
];

#[test]
fn declared_features_match_cfg_usage() {
    let root = repo_root();
    let cargo_toml = include_str!("../Cargo.toml");
    let declared = collect_declared_features(cargo_toml);
    let used = collect_cfg_features(&root);
    assert_eq!(
        declared, used,
        "declared features must match cfg(feature = ...) usage in src/ and tests/"
    );
}

#[test]
fn default_features_support_readme_quickstart() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(
        &features,
        "default",
        &["core", "shape", "codec-lisp", "numbers-f64"],
    );
}

#[test]
fn public_facade_alias_table_mentions_declared_features() {
    let declared = collect_declared_features(include_str!("../Cargo.toml"));
    let missing = PUBLIC_FACADE_ALIASES
        .iter()
        .filter(|(feature, _)| !declared.contains(*feature))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "public facade aliases must reference declared features: {missing:?}"
    );
}

#[test]
fn feature_dep_edges_reference_optional_dependencies() {
    let cargo_toml = include_str!("../Cargo.toml");
    let features = collect_feature_dependencies(cargo_toml);
    let optional_dependencies = collect_optional_dependencies(cargo_toml);
    assert_dep_edges_reference_optional_dependencies(&features, &optional_dependencies);
}

#[test]
fn all_feature_metadata_has_no_ignored_optional_dependencies() {
    assert_all_feature_metadata_has_no_invalid_dependency_warnings(&repo_root());
}

#[test]
fn r10_numeric_feature_implications_stay_wired() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(
        &features,
        "numbers-rk",
        &["numbers-numeric", "numbers-tensor"],
    );
    assert_feature_includes(
        &features,
        "numbers-rational",
        &["numbers-arith", "numbers-bigint", "numbers-core"],
    );
    assert_feature_includes(
        &features,
        "numbers-tensor-linalg",
        &["numbers-tensor", "numbers-cas"],
    );
    assert_feature_includes(
        &features,
        "numbers-tensor-cmplxf",
        &["numbers-tensor", "numbers-complex", "numbers-f64"],
    );
    assert_feature_includes(
        &features,
        "numbers-codec",
        &[
            "numbers-core",
            "numbers-f64",
            "numbers-i64",
            "numbers-bool",
            "numbers-fixed",
            "numbers-float",
            "numbers-bigint",
            "numbers-rational",
            "numbers-complex",
            "numbers-exotic",
            "numbers-cas",
            "numbers-func",
            "numbers-numeric",
            "numbers-rk",
            "numbers-quad",
            "numbers-tensor",
            "numbers-tensor-bcast",
            "numbers-tensor-linalg",
            "numbers-tensor-bit",
            "numbers-tensor-f64",
            "numbers-tensor-i64",
            "numbers-tensor-rat64",
            "numbers-tensor-cmplxf",
        ],
    );
    assert_feature_includes(
        &features,
        "numbers-prelude",
        &[
            "numbers-ad",
            "numbers-arith",
            "numbers-core",
            "numbers-f64",
            "numbers-i64",
            "numbers-rational",
            "numbers-complex",
            "numbers-bool",
            "numbers-fixed",
            "numbers-float",
            "numbers-bigint",
            "numbers-exotic",
            "numbers-cas",
            "numbers-cas-diff",
            "numbers-cas-eval",
            "numbers-func",
            "numbers-numeric",
            "numbers-rk",
            "numbers-quad",
            "numbers-tensor",
            "numbers-tensor-bcast",
            "numbers-tensor-linalg",
            "numbers-tensor-bit",
            "numbers-tensor-f64",
            "numbers-tensor-i64",
            "numbers-tensor-rat64",
            "numbers-tensor-cmplxf",
            "numbers-codec",
        ],
    );
}

#[test]
fn r12_logic_feature_implications_stay_wired() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(&features, "logic", &["logic-core", "logic-numbers"]);
    assert_feature_includes(&features, "logic-agent", &["logic-core", "agent"]);
    assert_feature_includes(&features, "logic-server", &["logic-core", "server"]);
    assert_feature_includes(&features, "logic-wasm", &["logic-core", "wasm"]);
    assert_feature_includes(
        &features,
        "logic-numbers",
        &[
            "logic-core",
            "numbers-arith",
            "numbers-f64",
            "numbers-i64",
            "numbers-rational",
        ],
    );
}

#[rustfmt::skip] const MCP_STREAM_DEPS: &[&str] = &["mcp", "stream-core", "stream-fabric", "stream-combinators", "sim-lib-mcp/stream", "sim-lib-mcp/progress"];
#[rustfmt::skip] const MCP_HTTP_DEPS: &[&str] = &["mcp-stream", "server", "server-net-http", "sim-lib-mcp/http"];
const MCP_SAMPLING_DEPS: &[&str] = &["mcp", "agent-runner-core", "sim-lib-mcp/sampling"];

#[test]
fn g6_mcp_feature_implications_stay_wired() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    let cases: &[(&str, &[&str])] = &[
        ("mcp", &["dep:sim-lib-mcp", "codec-mcp", "core", "shape"]),
        ("mcp-skill", &["mcp", "skill", "sim-lib-mcp/skill"]),
        ("mcp-stdio", &["mcp", "sim-lib-mcp/stdio"]),
        ("mcp-stream", MCP_STREAM_DEPS),
        ("mcp-http", MCP_HTTP_DEPS),
        ("mcp-client", &["mcp-skill", "sim-lib-mcp/client"]),
        ("mcp-sampling", MCP_SAMPLING_DEPS),
        ("mcp-cassette", &["mcp", "sim-lib-mcp/cassette"]),
        ("mcp-binary", &["mcp-stdio"]),
        (
            "skill-serve",
            &["skill-mcp", "mcp-skill", "server", "sim-lib-skill/serve"],
        ),
    ];
    for (feature, expected) in cases {
        assert_feature_includes(&features, feature, expected);
    }
}

#[test]
fn r11_music_stack_feature_implications_stay_wired() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(
        &features,
        "pitch",
        &[
            "pitch-core",
            "pitch-set",
            "pitch-scale",
            "pitch-chord",
            "pitch-namer",
            "pitch-dissonance",
            "pitch-shapes",
        ],
    );
    assert_feature_includes(
        &features,
        "pitch-namer",
        &[
            "pitch-namer-forte",
            "pitch-namer-jazz",
            "pitch-namer-roman",
            "pitch-namer-riemann",
            "pitch-set",
            "pitch-scale",
            "pitch-chord",
        ],
    );
    assert_feature_includes(
        &features,
        "midi",
        &[
            "midi-core",
            "midi-smf",
            "midi-live",
            "midi-sysex",
            "midi-shapes",
        ],
    );
    assert_feature_includes(&features, "midi-sysex", &["midi-core"]);
    assert_feature_includes(
        &features,
        "music",
        &[
            "music-core",
            "music-combinators",
            "music-analysis",
            "music-transform",
            "music-lower",
            "music-lift",
            "music-notation",
            "music-shapes",
            "pitch",
            "midi",
        ],
    );
    assert_feature_includes(
        &features,
        "sound",
        &[
            "sound-core",
            "sound-spectrum",
            "sound-timbre",
            "sound-tuning",
            "sound-dissonance",
            "sound-bridge",
            "sound-render",
            "sound-shapes",
            "pitch",
            "midi",
        ],
    );
    assert_feature_includes(
        &features,
        "music-stack",
        &[
            "pitch",
            "midi",
            "music",
            "sound",
            "sound-gm",
            "sound-audio-lift",
            "sound-music",
        ],
    );
    assert_feature_includes(&features, "sound-music", &["sound", "music"]);
    assert_feature_includes(
        &features,
        "sound-audio-lift",
        &["sound-spectrum", "sound-tuning", "pitch"],
    );
    assert_feature_includes(&features, "sound-gm", &["sound-timbre"]);
    assert_feature_includes(&features, "pitch-wasm-frame", &["pitch", "wasm"]);
    assert_feature_includes(&features, "midi-wasm-frame", &["midi", "wasm"]);
    assert_feature_includes(&features, "stream-host", &["stream-midi"]);
    assert_feature_includes(&features, "music-wasm-frame", &["music", "wasm"]);
    assert_feature_includes(
        &features,
        "sound-wasm-frame",
        &[
            "sim-lib-sound-wasm-frame/sound-music",
            "sound",
            "sound-music",
            "wasm",
        ],
    );
    assert_feature_includes(
        &features,
        "music-stack-wasm-frame",
        &[
            "music-stack",
            "pitch-wasm-frame",
            "midi-wasm-frame",
            "music-wasm-frame",
            "sound-wasm-frame",
        ],
    );
}

#[test]
fn r11_production_crate_dependency_boundaries_stay_wired() {
    let root = repo_root();
    assert_crate_cargo_tomls_do_not_contain(
        &root,
        "sim-lib-pitch-",
        &["sim-lib-midi-", "sim-lib-music-", "sim-lib-sound-"],
    );
    assert_crate_cargo_tomls_do_not_contain(
        &root,
        "sim-lib-midi-",
        &["sim-lib-pitch-", "sim-lib-music-", "sim-lib-sound-"],
    );
    assert_crate_cargo_tomls_do_not_contain(&root, "sim-lib-music-", &["sim-lib-sound-"]);
}

#[test]
fn r10_femm_feature_implications_stay_wired() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
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
