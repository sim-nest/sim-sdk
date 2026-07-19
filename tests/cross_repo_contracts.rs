#![cfg(all(
    feature = "core",
    feature = "shape",
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational",
    feature = "numbers-arith",
    feature = "server",
    feature = "view"
))]
//! Cross-repo contract integration tests for the public facade.
//!
//! These tests use only the public `sim` facade and the public types it
//! re-exports. They protect contracts that cross repository boundaries:
//!
//! - codec surfaces can feed Shape checking, evaluation, `as_expr`, and
//!   cross-codec re-encoding;
//! - number literals parsed by a codec promote through the runtime number
//!   lattice and re-encode losslessly;
//! - `realize` can evaluate through a server-backed `EvalFabric`;
//! - loader registry users can load a host lib and invoke its exports;
//! - the web view dispatcher renders a Scene and an Intent-derived operation
//!   can be submitted through `realize`.

use std::sync::Arc;

use sim::codec::{
    DecodePosition, DecodedForm, Input, Output, decode_default_with_codec, decode_with_codec,
    encode_with_codec,
};
use sim::kernel::{
    AbiVersion, Args, CORE_FUNCTION_CLASS_ID, Callable, ClassRef, Cx, DefaultFactory, EagerPolicy,
    EncodeOptions, Export, Expr, Lib, LibManifest, LibSource, LibTarget, Linker, LoadCx,
    NumberLiteral, Object, ObjectCompat, QuoteMode, ReadPolicy, Symbol, Value, Version,
    eval_fabric_capability, macro_expand_eval_capability,
};
use sim::lib_intent::{Origin, intent};
use sim::lib_view::{LensRegistry, UNIVERSAL_EDITOR_ID, register_universal_default};

fn cx_with_public_runtime() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    sim::runtime::install_core_runtime(&mut cx);
    cx.grant(eval_fabric_capability());
    cx.grant(macro_expand_eval_capability());
    install_public_codecs(&mut cx);
    cx
}

fn install_public_codecs(cx: &mut Cx) {
    let lisp = sim::codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    let json = sim::codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&json).unwrap();
    let binary = sim::codec_binary::BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&binary).unwrap();
}

fn codec_symbol(name: &str) -> Symbol {
    Symbol::qualified("codec", name)
}

fn output_to_input(output: Output) -> Input {
    match output {
        Output::Text(text) => Input::Text(text),
        Output::Bytes(bytes) => Input::Bytes(bytes),
    }
}

fn decode_lisp_eval(cx: &mut Cx, text: &str) -> Expr {
    match decode_default_with_codec(
        cx,
        &codec_symbol("lisp"),
        Input::Text(text.to_owned()),
        ReadPolicy::default(),
        DecodePosition::Eval,
    )
    .unwrap()
    {
        DecodedForm::Term(term) => Expr::from(term),
        DecodedForm::Datum(datum) => Expr::from(datum),
    }
}

fn expect_number(expr: Expr, domain: Symbol, canonical: &str) {
    assert_eq!(
        expr,
        Expr::Number(NumberLiteral {
            domain,
            canonical: canonical.to_owned(),
        })
    );
}

#[test]
fn codec_shape_eval_as_expr_encode_pipeline_uses_public_surfaces() {
    let mut cx = cx_with_public_runtime();
    let decoded = decode_lisp_eval(&mut cx, "(math/add 1 2)");
    let expr_shape = cx
        .resolve_shape(&Symbol::qualified("core", "Expr"))
        .unwrap();
    let number_shape = cx
        .resolve_shape(&Symbol::qualified("core", "Number"))
        .unwrap();

    let expr_match = sim::shapes::check_shape_expr(&mut cx, &expr_shape, &decoded).unwrap();
    assert!(
        expr_match.accepted,
        "decoded expression must satisfy core/Expr"
    );

    let value = cx.eval_expr(decoded).unwrap();
    let value_match =
        sim::shapes::check_shape_value(&mut cx, &number_shape, value.clone()).unwrap();
    assert!(
        value_match.accepted,
        "evaluated value must satisfy core/Number"
    );

    let expr = value.object().as_expr(&mut cx).unwrap();
    expect_number(expr.clone(), Symbol::qualified("numbers", "i64"), "3");

    for codec in [
        codec_symbol("lisp"),
        codec_symbol("json"),
        codec_symbol("binary"),
    ] {
        let encoded = encode_with_codec(&mut cx, &codec, &expr, EncodeOptions::default()).unwrap();
        let decoded = decode_with_codec(
            &mut cx,
            &codec,
            output_to_input(encoded),
            ReadPolicy::default(),
        )
        .unwrap();
        assert_eq!(decoded, expr, "{codec} must round-trip the evaluated expr");
    }
}

#[test]
fn parsed_number_literal_promotes_and_reencodes_across_codecs() {
    let mut cx = cx_with_public_runtime();
    let decoded = decode_lisp_eval(&mut cx, "(math/add 0.25 1/2)");
    let value = cx.eval_expr(decoded).unwrap();
    let expr = value.object().as_expr(&mut cx).unwrap();
    expect_number(
        expr.clone(),
        Symbol::qualified("numbers", "rational"),
        "3/4",
    );

    for codec in [codec_symbol("lisp"), codec_symbol("json")] {
        let encoded = encode_with_codec(&mut cx, &codec, &expr, EncodeOptions::default()).unwrap();
        let decoded = decode_with_codec(
            &mut cx,
            &codec,
            output_to_input(encoded),
            ReadPolicy::default(),
        )
        .unwrap();
        assert_eq!(decoded, expr, "{codec} must preserve the promoted literal");
    }
}

#[test]
fn realize_round_trips_through_server_connection_fabric() {
    let mut cx = cx_with_public_runtime();
    sim::install_server_lib(&mut cx).unwrap();

    let connection = cx
        .call_exprs(
            cx.resolve_function(&Symbol::qualified("server", "connect"))
                .unwrap(),
            vec![Expr::Quote {
                mode: QuoteMode::Quote,
                expr: Box::new(Expr::Symbol(Symbol::new("local"))),
            }],
        )
        .unwrap();
    cx.registry_mut()
        .register_value(Symbol::qualified("integration", "conn"), connection)
        .unwrap();

    let value = cx
        .call_exprs(
            cx.resolve_function(&Symbol::new("realize")).unwrap(),
            vec![
                Expr::Call {
                    operator: Box::new(Expr::Symbol(Symbol::qualified("math", "add"))),
                    args: vec![
                        Expr::Number(NumberLiteral {
                            domain: Symbol::qualified("numbers", "f64"),
                            canonical: "4".to_owned(),
                        }),
                        Expr::Number(NumberLiteral {
                            domain: Symbol::qualified("numbers", "f64"),
                            canonical: "5".to_owned(),
                        }),
                    ],
                },
                Expr::Symbol(Symbol::new(":fabric")),
                Expr::Symbol(Symbol::qualified("integration", "conn")),
                Expr::Symbol(Symbol::new(":result")),
                Expr::Symbol(Symbol::qualified("core", "Number")),
            ],
        )
        .unwrap();

    expect_number(
        value.object().as_expr(&mut cx).unwrap(),
        Symbol::qualified("numbers", "f64"),
        "9",
    );
}

#[test]
fn loader_registry_loads_public_host_lib_and_invokes_export() {
    let mut cx = cx_with_public_runtime();
    let registry = sim::loaders::standard_loader_registry();

    registry
        .load_and_register(&mut cx, LibSource::Host(Box::new(IntegrationLib)))
        .unwrap();

    let value = cx
        .call_function(
            &Symbol::qualified("integration", "answer"),
            Args::new(Vec::new()),
        )
        .unwrap();
    expect_number(
        value.object().as_expr(&mut cx).unwrap(),
        Symbol::qualified("numbers", "f64"),
        "42",
    );
}

#[test]
fn view_dispatcher_renders_scene_and_intent_operation_realizes() {
    let mut cx = cx_with_public_runtime();
    let value = Expr::Map(vec![
        (
            Expr::Symbol(Symbol::new("title")),
            Expr::String("draft".to_owned()),
        ),
        (
            Expr::Symbol(Symbol::new("count")),
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "1".to_owned(),
            }),
        ),
    ]);

    let mut registry = LensRegistry::new();
    register_universal_default(&mut registry, false);
    let grant = |_: &sim::kernel::CapabilityName| true;
    let ctx = sim::lib_view::DispatchContext::permissive(&grant);
    let outcome = registry.dispatch_view(&mut cx, &value, &ctx).unwrap();
    let scene = registry.render(&mut cx, &outcome.lens_id, &value).unwrap();
    sim::lib_scene::validate_scene(&scene).unwrap();

    let edit = intent(
        "edit-field",
        Origin::human(1),
        vec![
            ("target", value.clone()),
            (
                "path",
                Expr::List(vec![Expr::Vector(vec![
                    Expr::Symbol(Symbol::new("k")),
                    Expr::Symbol(Symbol::new("count")),
                ])]),
            ),
            (
                "value",
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "2".to_owned(),
                }),
            ),
        ],
    );
    let editor = registry
        .get(&Symbol::new(UNIVERSAL_EDITOR_ID))
        .unwrap()
        .editor
        .clone()
        .unwrap();
    let draft = editor.decode(&mut cx, &value, &edit).unwrap();
    assert!(draft.committable, "valid edit intent must produce a commit");
    let operation = editor.commit(&mut cx, &draft).unwrap();

    let realized = cx
        .call_exprs(
            cx.resolve_function(&Symbol::new("realize")).unwrap(),
            vec![Expr::Quote {
                mode: QuoteMode::Quote,
                expr: Box::new(operation.form.clone()),
            }],
        )
        .unwrap();
    assert_eq!(realized.object().as_expr(&mut cx).unwrap(), operation.form);
}

struct IntegrationLib;

impl Lib for IntegrationLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: Symbol::qualified("integration", "lib"),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![Export::Function {
                symbol: Symbol::qualified("integration", "answer"),
                function_id: None,
            }],
        }
    }

    fn load(&self, cx: &mut LoadCx, linker: &mut Linker<'_>) -> sim::kernel::Result<()> {
        let value = cx.factory().opaque(Arc::new(IntegrationAnswer))?;
        linker.function_value(Symbol::qualified("integration", "answer"), value)?;
        Ok(())
    }
}

#[derive(Clone)]
struct IntegrationAnswer;

impl Object for IntegrationAnswer {
    fn display(&self, _cx: &mut Cx) -> sim::kernel::Result<String> {
        Ok("#<function integration/answer>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ObjectCompat for IntegrationAnswer {
    fn class(&self, cx: &mut Cx) -> sim::kernel::Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&Symbol::qualified("core", "Function"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            CORE_FUNCTION_CLASS_ID,
            Symbol::qualified("core", "Function"),
        )
    }

    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for IntegrationAnswer {
    fn call(&self, cx: &mut Cx, _args: Args) -> sim::kernel::Result<Value> {
        cx.factory()
            .number_literal(Symbol::qualified("numbers", "f64"), "42".to_owned())
    }
}
