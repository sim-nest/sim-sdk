//! Conformance for the VIEW_4 surface protocol: the universal surface codec.
//!
//! VIEW_4 frames a view as a reversible codec at the open `surface` output
//! position. This spec pins the three checkable properties of that contract,
//! all reached through the public `sim` facade:
//!
//! 1. ROUNDTRIP -- a no-op edit preserves the value, for several baseline kinds.
//! 2. PROJECTION DETERMINISM -- for every surface preset, encoding is
//!    deterministic and yields a valid Scene; `project_for_preset` is likewise
//!    deterministic.
//! 3. GOLDEN -- the exact projected Scene for the `cli` and `watch` presets is
//!    locked, so a regression in projection is caught.

use std::sync::Arc;

use sim::{
    kernel::{Cx, DefaultFactory, EagerPolicy, Expr, Symbol},
    lib_scene::{node, sym, validate_scene},
    lib_view::{
        UniversalEditor, UniversalView,
        codec::{PairCodec, SurfaceCodec, roundtrip_holds},
        profiles::project_for_preset,
        surface::{self, SURFACE_PRESETS},
    },
};

/// A bare runtime context: the surface codec needs no installed libraries.
fn surface_cx() -> Cx {
    Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory))
}

/// The universal default surface codec under test.
fn universal_codec() -> PairCodec {
    PairCodec::new(
        Arc::new(UniversalView),
        Arc::new(UniversalEditor::writable()),
    )
}

/// One representative value per baseline kind: a string, a nil, a list, a map.
fn baseline_values() -> Vec<Expr> {
    vec![
        Expr::String("hello".to_owned()),
        Expr::Nil,
        Expr::List(vec![
            Expr::Nil,
            Expr::Bool(true),
            Expr::String("x".to_owned()),
        ]),
        Expr::Map(vec![
            (Expr::Symbol(Symbol::new("a")), Expr::Bool(true)),
            (Expr::Symbol(Symbol::new("b")), Expr::Nil),
        ]),
    ]
}

#[test]
fn surface_codec_roundtrip_holds_for_baseline_values() {
    let mut cx = surface_cx();
    let codec = universal_codec();
    for value in baseline_values() {
        assert!(
            roundtrip_holds(&mut cx, &codec, &value).unwrap(),
            "a no-op edit must preserve {value:?}"
        );
    }
}

#[test]
fn surface_projection_is_deterministic_and_valid_for_every_preset() {
    let mut cx = surface_cx();
    let codec = universal_codec();
    assert!(
        !SURFACE_PRESETS.is_empty(),
        "the surface preset catalog must not be empty"
    );

    for value in baseline_values() {
        for name in SURFACE_PRESETS {
            let caps = surface::preset(name).unwrap_or_else(|| panic!("preset {name} must exist"));

            let first = codec.encode(&mut cx, &value, &caps).unwrap();
            let second = codec.encode(&mut cx, &value, &caps).unwrap();
            assert_eq!(first, second, "{name} encode must be deterministic");
            validate_scene(&first)
                .unwrap_or_else(|err| panic!("{name} produced an invalid scene: {err}"));

            // The profile-level projection is likewise deterministic per preset.
            let projected_once = project_for_preset(&first, name)
                .unwrap_or_else(|| panic!("project_for_preset must know {name}"));
            let projected_twice = project_for_preset(&first, name).unwrap();
            assert_eq!(
                projected_once, projected_twice,
                "{name} profile projection must be deterministic"
            );
        }
    }
}

#[test]
fn surface_projection_matches_golden_for_cli_and_watch() {
    let mut cx = surface_cx();
    let codec = universal_codec();
    let value = Expr::Nil;

    // `cli` is a dense surface: projection keeps the whole universal Scene.
    let cli = surface::preset("cli").unwrap();
    let cli_scene = codec.encode(&mut cx, &value, &cli).unwrap();
    assert_eq!(
        cli_scene,
        golden_universal_nil(),
        "cli projection of nil regressed"
    );

    // `watch` is a glance surface: projection keeps one child, recursively.
    let watch = surface::preset("watch").unwrap();
    let watch_scene = codec.encode(&mut cx, &value, &watch).unwrap();
    assert_eq!(
        watch_scene,
        golden_watch_nil(),
        "watch projection of nil regressed"
    );

    // The golden also proves the two surfaces project the same value differently.
    assert_ne!(
        cli_scene, watch_scene,
        "a glance surface must reduce where a dense one does not"
    );
}

/// A `scene/text` node carrying exactly `text`.
fn text_line(text: &str) -> Expr {
    node("text", vec![("text", Expr::String(text.to_owned()))])
}

/// The full, unreduced universal Scene for `nil` (the dense/`cli` golden).
fn golden_universal_nil() -> Expr {
    node(
        "stack",
        vec![
            ("id", sym("universal")),
            ("dir", sym("column")),
            (
                "children",
                Expr::List(vec![
                    golden_summary_card_nil(),
                    node(
                        "box",
                        vec![
                            ("role", sym("structure")),
                            ("children", Expr::List(vec![text_line("value: nil")])),
                        ],
                    ),
                    node(
                        "box",
                        vec![
                            ("role", sym("canonical-text")),
                            (
                                "children",
                                Expr::List(vec![
                                    text_line("nil"),
                                    node(
                                        "field",
                                        vec![
                                            ("input-kind", sym("text")),
                                            ("value", Expr::String("nil".to_owned())),
                                            ("target", Expr::Nil),
                                            ("path", Expr::List(vec![])),
                                            ("readonly", Expr::Bool(false)),
                                        ],
                                    ),
                                ]),
                            ),
                        ],
                    ),
                    node(
                        "stack",
                        vec![
                            ("role", sym("operations")),
                            ("dir", sym("column")),
                            (
                                "children",
                                Expr::List(vec![
                                    golden_action_button("copy", "Copy"),
                                    golden_action_button("edit", "Edit"),
                                ]),
                            ),
                        ],
                    ),
                ]),
            ),
        ],
    )
}

/// The summary card region of the universal Scene for `nil`.
fn golden_summary_card_nil() -> Expr {
    node(
        "box",
        vec![
            ("role", sym("summary")),
            (
                "children",
                Expr::List(vec![
                    text_line("kind: nil"),
                    text_line("label: nil"),
                    node(
                        "badge",
                        vec![
                            ("status", sym("ok")),
                            ("label", Expr::String("round-trips".to_owned())),
                        ],
                    ),
                ]),
            ),
        ],
    )
}

/// An operations-inspector action button targeting `nil`.
fn golden_action_button(control: &str, label: &str) -> Expr {
    node(
        "button",
        vec![
            ("control", sym(control)),
            ("label", Expr::String(label.to_owned())),
            ("target", Expr::Nil),
        ],
    )
}

/// The glance-reduced universal Scene for `nil` (the `watch` golden): the top
/// stack keeps its first child, and that child keeps its own first child.
fn golden_watch_nil() -> Expr {
    node(
        "stack",
        vec![
            ("id", sym("universal")),
            ("dir", sym("column")),
            (
                "children",
                Expr::List(vec![node(
                    "box",
                    vec![
                        ("role", sym("summary")),
                        ("children", Expr::List(vec![text_line("kind: nil")])),
                    ],
                )]),
            ),
        ],
    )
}
