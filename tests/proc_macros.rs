#![cfg(feature = "proc-macros")]
#![allow(deprecated)]

use std::sync::Arc;

#[allow(unused_imports)]
use sim::{
    case,
    codec_lisp::{LispCodecLib, encode_object_lisp},
    kernel::{
        Args, CanonicalPolicy, DefaultFactory, EagerPolicy, EncodeOptions, EncodePosition, Error,
        Expr, Lib, NumberLiteral, ReadConstructEncodePolicy, ReadEvalEncodePolicy, Symbol, WriteCx,
        read_construct_capability,
    },
    runtime::install_core_runtime,
    shape, sim_class, sim_codec, sim_constructor, sim_fn, sim_lib,
};

#[sim_lib(id = "geometry", version = "0.1.0")]
mod geometry {
    #[sim_class(name = "Point")]
    #[shape("(fields (:x Number) (:y Number))")]
    #[derive(Clone)]
    pub struct Point {
        x: f64,
        y: f64,
    }

    #[sim_constructor(class = "Point")]
    #[case(args = "((capture x Number) (capture y Number))", result = "Point")]
    pub fn point(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[sim_fn(name = "distance")]
    #[case(args = "((capture a Point) (capture b Point))", result = "Number")]
    pub fn distance(a: &Point, b: &Point) -> f64 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[sim_lib(id = "utility", version = "0.1.0", native_export = true)]
mod utility {
    use sim::kernel::{Expr, QuoteMode, Symbol};

    #[sim_codec(symbol = "codec/mock", decode = "decode_mock", encode = "encode_mock")]
    pub fn mock_codec() {}

    pub fn decode_mock(text: String) -> Expr {
        Expr::List(vec![
            Expr::Symbol(Symbol::qualified("mock", "decoded")),
            Expr::String(text),
        ])
    }

    pub fn encode_mock(expr: Expr) -> String {
        match expr {
            Expr::Symbol(symbol) => format!("mock:{symbol}"),
            other => format!("mock:{other:?}"),
        }
    }

    #[sim_fn(name = "echo-string")]
    #[case(args = "((capture value String))", result = "String")]
    pub fn echo_string(value: String) -> String {
        value
    }

    #[sim_fn(name = "flip")]
    #[case(args = "((capture value Bool))", result = "Bool")]
    pub fn flip(value: bool) -> bool {
        !value
    }

    #[sim_fn(name = "make-symbol")]
    #[case(args = "((capture value String))", result = "Symbol")]
    pub fn make_symbol(value: String) -> Symbol {
        Symbol::new(value)
    }

    #[sim_fn(name = "quote-it")]
    #[case(args = "((capture value Any))", result = "Any")]
    pub fn quote_it(value: Expr) -> Expr {
        Expr::Quote {
            mode: QuoteMode::Quote,
            expr: Box::new(value),
        }
    }

    #[sim_fn(name = "describe")]
    #[case(args = "((capture value Number))", result = "String")]
    #[case(args = "((capture value String))", result = "String")]
    pub fn describe(value: Expr) -> String {
        match value {
            Expr::Number(number) => format!("number:{}", number.canonical),
            Expr::String(value) => format!("string:{value}"),
            other => format!("other:{other:?}"),
        }
    }
}

#[sim_lib(id = "geometry-conflict", version = "0.1.0")]
mod geometry_conflict {
    #[sim_fn(name = "distance")]
    #[case(args = "((capture value Number))", result = "Number")]
    pub fn distance(value: f64) -> f64 {
        value
    }
}

fn cx() -> sim::kernel::Cx {
    let mut cx = sim::kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn cx_with_lisp_codec() -> sim::kernel::Cx {
    let mut cx = cx();
    let lisp = LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    cx
}

fn normalize_spaces(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn proc_macros_generate_manifest_runtime_and_inspectable_output() {
    let lib = geometry::GeometryLib;
    let manifest = Lib::manifest(&lib);
    assert_eq!(manifest.id, Symbol::new("geometry"));
    assert!(
        manifest
            .exports
            .iter()
            .any(|export| export.symbol() == &Symbol::new("Point"))
    );
    assert!(
        manifest
            .exports
            .iter()
            .any(|export| export.symbol() == &Symbol::new("distance"))
    );
    assert!(geometry::__SIM_LIB_EXPANSION.contains("GeometryLib"));
    assert!(geometry::__SIM_LIB_EXPANSION.contains("__LispPointValue"));
    let normalized = normalize_spaces(geometry::__SIM_LIB_EXPANSION);
    assert!(normalized.contains("pub struct GeometryLib"));
    assert!(normalized.contains("impl :: sim :: kernel :: Lib for GeometryLib"));
    assert!(normalized.contains("build_distance_function"));
    let utility_normalized = normalize_spaces(utility::__SIM_LIB_EXPANSION);
    assert!(utility_normalized.contains("NativeLibAbiV1"));
    assert!(utility_normalized.contains("sim_native_abi_v1"));
    let utility_manifest = Lib::manifest(&utility::UtilityLib);
    assert!(
        utility_manifest
            .exports
            .iter()
            .any(|export| export.kind() == "codec"
                && export.symbol() == &Symbol::qualified("codec", "mock"))
    );

    let mut cx = cx();
    cx.load_lib(&lib).unwrap();

    assert!(cx.resolve_class(&Symbol::new("Point")).is_ok());
    assert!(cx.resolve_function(&Symbol::new("point")).is_ok());
    assert!(cx.resolve_function(&Symbol::new("distance")).is_ok());
    assert!(
        cx.resolve_shape(&Symbol::qualified("Point", "instance-shape"))
            .is_ok()
    );

    let origin = cx
        .call_function(
            &Symbol::new("point"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "0".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    let point = cx
        .call_function(
            &Symbol::new("point"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "3".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "4".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    let distance = cx
        .call_function(&Symbol::new("distance"), Args::new(vec![origin, point]))
        .unwrap();

    assert_eq!(
        distance.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "5".to_owned(),
        })
    );
}

#[test]
fn generated_class_shape_enforces_required_fields() {
    let mut cx = cx();
    cx.load_lib(&geometry::GeometryLib).unwrap();

    let point = cx
        .call_function(
            &Symbol::new("point"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "1".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "2".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert!(matches!(
        point.object().as_expr(&mut cx).unwrap(),
        Expr::Extension { .. }
    ));

    let shape_value = cx
        .resolve_shape(&Symbol::qualified("Point", "instance-shape"))
        .unwrap();
    let shape = shape_value
        .object()
        .downcast_ref::<sim::shape::ShapeObject>()
        .unwrap();
    let missing_x = Expr::Extension {
        tag: Symbol::qualified("expr", "object"),
        payload: Box::new(Expr::Map(vec![
            (
                Expr::Symbol(Symbol::new("class")),
                Expr::Symbol(Symbol::new("Point")),
            ),
            (
                Expr::Symbol(Symbol::new("fields")),
                Expr::Map(vec![(
                    Expr::Symbol(Symbol::new("y")),
                    Expr::Number(NumberLiteral {
                        domain: Symbol::qualified("numbers", "f64"),
                        canonical: "2".to_owned(),
                    }),
                )]),
            ),
        ])),
    };
    let matched = shape.shape.check_expr(&mut cx, &missing_x).unwrap();
    assert!(!matched.accepted);
}

#[test]
fn generated_lib_collisions_fail_cleanly() {
    let mut cx = cx();
    cx.load_lib(&geometry::GeometryLib).unwrap();
    let duplicate = cx.load_lib(&geometry::GeometryLib);
    assert!(matches!(duplicate, Err(Error::DuplicateLib { .. })));
}

#[test]
fn generated_symbol_collisions_fail_cleanly() {
    let mut cx = cx();
    cx.load_lib(&geometry::GeometryLib).unwrap();
    let duplicate = cx.load_lib(&geometry_conflict::GeometryConflictLib);
    assert!(matches!(duplicate, Err(Error::DuplicateExport { .. })));
}

#[test]
fn generated_read_construct_round_trips_through_constructor_encoding() {
    let mut cx = cx_with_lisp_codec();
    cx.load_lib(&geometry::GeometryLib).unwrap();

    let denied = cx.read_construct(
        &Symbol::new("Point"),
        vec![
            cx.factory()
                .number_literal(Symbol::qualified("numbers", "f64"), "1".to_owned())
                .unwrap(),
            cx.factory()
                .number_literal(Symbol::qualified("numbers", "f64"), "2".to_owned())
                .unwrap(),
        ],
    );
    assert!(matches!(
        denied,
        Err(Error::CapabilityDenied { capability })
            if capability == read_construct_capability()
    ));

    cx.grant(read_construct_capability());
    let point = cx
        .read_construct(
            &Symbol::new("Point"),
            vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "1".to_owned())
                    .unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "2".to_owned())
                    .unwrap(),
            ],
        )
        .unwrap();

    let encoding = point
        .object()
        .as_object_encoder()
        .unwrap()
        .object_encoding(&mut cx)
        .unwrap();
    assert!(matches!(
        encoding,
        sim::kernel::ObjectEncoding::Constructor { ref class, ref args }
            if class == &Symbol::new("Point") && args.len() == 2
    ));

    let codec_id = cx
        .resolve_codec(&Symbol::qualified("codec", "lisp"))
        .unwrap()
        .object()
        .downcast_ref::<sim::codec::CodecRuntime>()
        .unwrap()
        .id;
    let mut write = WriteCx {
        cx: &mut cx,
        codec: codec_id,
        options: EncodeOptions {
            position: EncodePosition::Quote,
            canonical: CanonicalPolicy::Canonical,
            lossless_origin: false,
            read_construct: ReadConstructEncodePolicy::Allow,
            read_eval: ReadEvalEncodePolicy::Forbid,
        },
    };
    let encoded = encode_object_lisp(&mut write, point).unwrap();
    assert_eq!(
        encoded,
        "#(Point (expr:number numbers/f64 \"1\") (expr:number numbers/f64 \"2\"))"
    );
}

#[test]
fn proc_macros_support_builtin_and_expr_conversions() {
    let mut cx = cx();
    cx.load_lib(&utility::UtilityLib).unwrap();

    let echoed = cx
        .call_function(
            &Symbol::new("echo-string"),
            Args::new(vec![cx.factory().string("hello".to_owned()).unwrap()]),
        )
        .unwrap();
    assert_eq!(
        echoed.object().as_expr(&mut cx).unwrap(),
        Expr::String("hello".to_owned())
    );

    let flipped = cx
        .call_function(
            &Symbol::new("flip"),
            Args::new(vec![cx.factory().bool(true).unwrap()]),
        )
        .unwrap();
    assert_eq!(
        flipped.object().as_expr(&mut cx).unwrap(),
        Expr::Bool(false)
    );

    let symbol = cx
        .call_function(
            &Symbol::new("make-symbol"),
            Args::new(vec![cx.factory().string("name".to_owned()).unwrap()]),
        )
        .unwrap();
    assert_eq!(
        symbol.object().as_expr(&mut cx).unwrap(),
        Expr::Symbol(Symbol::new("name"))
    );

    let quoted = cx
        .call_function(
            &Symbol::new("quote-it"),
            Args::new(vec![
                cx.factory().expr(Expr::String("x".to_owned())).unwrap(),
            ]),
        )
        .unwrap();
    assert!(matches!(
        quoted.object().as_expr(&mut cx).unwrap(),
        Expr::Quote { .. }
    ));
}

#[test]
fn proc_macros_support_multi_case_expr_overloads() {
    let mut cx = cx();
    cx.load_lib(&utility::UtilityLib).unwrap();

    let number = cx
        .call_function(
            &Symbol::new("describe"),
            Args::new(vec![
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "7".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        number.object().as_expr(&mut cx).unwrap(),
        Expr::String("number:7".to_owned())
    );

    let string = cx
        .call_function(
            &Symbol::new("describe"),
            Args::new(vec![cx.factory().string("ok".to_owned()).unwrap()]),
        )
        .unwrap();
    assert_eq!(
        string.object().as_expr(&mut cx).unwrap(),
        Expr::String("string:ok".to_owned())
    );
}
