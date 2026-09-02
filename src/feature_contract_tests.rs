mod cfg_sentinels;
mod facade_thinness;
mod glasses;
mod support;
mod watch;

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

// Public feature closures that intentionally compose dependencies without
// adding a separate facade module or cfg gate of their own.
const COMPOSITION_ONLY_FEATURES: &[&str] = &[
    "numbers-method",
    "numbers-quantity",
    "physics-adapter-femm",
    "physics-adapter-interference",
    "physics-full",
    "physics-proof-extended",
];

#[test]
fn declared_features_match_cfg_usage() {
    let root = repo_root();
    let cargo_toml = include_str!("../Cargo.toml");
    let declared = collect_declared_features(cargo_toml);
    let mut used = collect_cfg_features(&root);
    used.extend(
        COMPOSITION_ONLY_FEATURES
            .iter()
            .map(|feature| (*feature).to_owned()),
    );
    assert_eq!(
        declared, used,
        "declared features must be cfg gates or documented dependency-only compositions"
    );

    let dependencies = collect_feature_dependencies(cargo_toml);
    for feature in COMPOSITION_ONLY_FEATURES {
        assert!(
            dependencies
                .get(*feature)
                .is_some_and(|edges| !edges.is_empty()),
            "composition-only feature {feature} must retain a non-empty dependency closure"
        );
    }
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
fn device_feature_installs_reference_base_and_recipes() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(&features, "device", &["device-reference", "cookbook"]);
}

#[test]
fn python_features_preserve_the_one_way_distribution_boundary() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(&features, "codec-python", &["dep:sim-codec-python"]);
    assert_feature_includes(
        &features,
        "standard-python",
        &[
            "dep:sim-lib-lang-python",
            "codec-python",
            "standard-gc-tracing",
        ],
    );
    assert_feature_includes(&features, "python", &["standard-python"]);
    assert_feature_includes(&features, "standard", &["standard-python"]);

    let process_adapter = include_str!("bin/sim.rs");
    let bootloader = include_str!("facade_cli.rs");
    assert!(process_adapter.contains("facade_cli::process_main()"));
    assert!(bootloader.contains("Bootloader::standard()"));
    assert!(!bootloader.contains("PythonRuntime"));
    assert!(!repo_root().join("src/bin/python.rs").exists());
}

#[test]
fn javascript_features_preserve_the_one_way_distribution_boundary() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(&features, "codec-javascript", &["dep:sim-codec-javascript"]);
    assert_feature_includes(
        &features,
        "standard-javascript",
        &[
            "dep:sim-lib-lang-javascript",
            "codec-javascript",
            "standard-gc-tracing",
        ],
    );
    assert_feature_includes(&features, "javascript", &["standard-javascript"]);
    assert_feature_includes(&features, "standard", &["standard-javascript"]);
    let process_adapter = include_str!("bin/sim.rs");
    let bootloader = include_str!("facade_cli.rs");
    assert!(process_adapter.contains("facade_cli::process_main()"));
    assert!(bootloader.contains("Bootloader::standard()"));
    assert!(!repo_root().join("src/bin/javascript.rs").exists());
    assert!(!repo_root().join("src/bin/node.rs").exists());
}

#[test]
fn typescript_notation_features_preserve_the_one_way_distribution_boundary() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(
        &features,
        "codec-typescript",
        &["dep:sim-codec-typescript", "codec-javascript", "shape"],
    );
    assert_feature_includes(
        &features,
        "standard-typescript",
        &[
            "dep:sim-lib-lang-typescript",
            "codec-typescript",
            "standard-javascript",
            "shape",
        ],
    );
    assert_feature_includes(&features, "typescript", &["standard-typescript"]);
    assert_feature_includes(&features, "standard", &["standard-typescript"]);

    let process_adapter = include_str!("bin/sim.rs");
    let bootloader = include_str!("facade_cli.rs");
    assert!(process_adapter.contains("facade_cli::process_main()"));
    assert!(bootloader.contains("TypeScript notation; does not type-check"));
    assert!(bootloader.contains("language/typescript-notation"));
    for executable in ["typescript", "tsc", "tsserver"] {
        assert!(
            !repo_root()
                .join(format!("src/bin/{executable}.rs"))
                .exists()
        );
    }
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

const REQUIRED_PUBLIC_GATES: &[&str] = &[
    "cargo fmt --all --check",
    "cargo test -p sim-conformance",
    "cargo test --workspace",
    "cargo clippy --workspace --all-targets -- -D warnings",
    "cargo doc --workspace --no-deps",
    "cargo clippy --workspace --all-features --all-targets -- -D warnings",
    "cargo test --workspace --all-features",
    "cargo run -p xtask -- simdoc --check",
];

#[test]
fn ci_and_public_checklists_name_required_gates() {
    let checked_files = [
        (
            ".github/workflows/ci.yml",
            include_str!("../.github/workflows/ci.yml"),
        ),
        ("README.md", include_str!("../README.md")),
        ("CONTRIBUTING.md", include_str!("../CONTRIBUTING.md")),
        (
            ".github/pull_request_template.md",
            include_str!("../.github/pull_request_template.md"),
        ),
    ];
    let missing = checked_files
        .into_iter()
        .flat_map(|(file, text)| {
            REQUIRED_PUBLIC_GATES
                .iter()
                .filter(move |command| !text.contains(**command))
                .map(move |command| format!("{file}: {command}"))
        })
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "CI and public checklists must stay aligned with repos.toml gates: {missing:?}"
    );
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
#[rustfmt::skip] const MCP_HTTP_DEPS: &[&str] = &["mcp-stream", "server", "server-net-http", "dep:sim-lib-mcp-http"];
const MCP_SAMPLING_DEPS: &[&str] = &["mcp", "agent-runner-core", "sim-lib-mcp/sampling"];

#[test]
fn g6_mcp_feature_implications_stay_wired() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    let cases: &[(&str, &[&str])] = &[
        ("mcp", &["dep:sim-lib-mcp", "codec-mcp", "core", "shape"]),
        ("mcp-skill", &["mcp", "skill", "sim-lib-mcp/skill"]),
        (
            "mcp-stdio",
            &["mcp", "sim-lib-mcp/stdio", "dep:sim-lib-mcp-stdio"],
        ),
        ("mcp-stream", MCP_STREAM_DEPS),
        ("mcp-http", MCP_HTTP_DEPS),
        ("mcp-legacy", &["mcp", "dep:sim-lib-mcp-legacy"]),
        (
            "mcp-oauth",
            &[
                "mcp-http",
                "dep:sim-lib-oauth-core",
                "dep:sim-lib-oauth-http",
                "dep:sim-lib-oauth-jose",
            ],
        ),
        (
            "mcp-protected-state",
            &["mcp", "dep:sim-lib-protected-state"],
        ),
        ("mcp-cancellation", &["mcp", "dep:sim-cancel"]),
        (
            "mcp-client",
            &["mcp-skill", "sim-lib-mcp/client", "dep:sim-lib-mcp-client"],
        ),
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

// conformance: GenAI SDK feature bundles keep base, local, and provider closures explicit.
#[test]
fn genai_feature_bundles_select_base_local_and_provider_closures() {
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

    let base = features.get("genai").expect("genai feature");
    for excluded in [
        "agent-runner-process",
        "agent-runner-ollama",
        "agent-runner-http",
        "agent-runner-http-tls",
    ] {
        assert!(
            !base.contains(excluded),
            "`genai` should not directly enable `{excluded}`"
        );
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
fn music_algorithm_features_preserve_focused_and_grouped_selection() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(&features, "signal", &["dep:sim-lib-numbers-signal"]);
    assert_feature_includes(
        &features,
        "music-algorithms",
        &[
            "signal",
            "dep:sim-lib-discrete-search",
            "dep:sim-lib-pitch-ratio",
        ],
    );
    assert_feature_includes(
        &features,
        "music-inference",
        &[
            "music-algorithms",
            "dep:sim-lib-music-consonance",
            "dep:sim-lib-music-counterpoint",
        ],
    );
    for (feature, dependency) in [
        ("discrete-search", "dep:sim-lib-discrete-search"),
        ("pitch-ratio", "dep:sim-lib-pitch-ratio"),
        ("music-consonance", "dep:sim-lib-music-consonance"),
        ("music-counterpoint", "dep:sim-lib-music-counterpoint"),
    ] {
        assert_feature_includes(&features, feature, &[dependency]);
    }

    let exports = include_str!("music_algorithm_exports.rs");
    assert!(!exports.contains("MeanDialect"));
    assert!(!exports.contains("compile_counterpoint_csp"));
    assert!(!exports.contains("CounterpointCsp"));
}

#[test]
fn serial_music_features_preserve_curated_grouping() {
    let features = collect_feature_dependencies(include_str!("../Cargo.toml"));
    assert_feature_includes(&features, "serial-core", &["dep:sim-lib-serial-core"]);
    assert_feature_includes(
        &features,
        "pitch-serial",
        &["dep:sim-lib-pitch-serial", "serial-core", "pitch-core"],
    );
    assert_feature_includes(
        &features,
        "serial-music",
        &[
            "dep:sim-lib-music-serial",
            "pitch-serial",
            "music-consonance",
            "music-notation",
            "music-shapes",
            "pitch-shapes",
            "cookbook",
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

mod femm;
