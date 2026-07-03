use std::{
    collections::{BTreeSet, VecDeque},
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use sim::{
    codec::{Input, Output, decode_with_codec, encode_with_codec},
    kernel::{
        AbiVersion, Cx, DefaultFactory, EagerPolicy, ExportKind, ExportState, Expr, Lib,
        LibManifest, LibTarget, Linker, LoadCx, NumberLiteral, PreparedArgs, QuoteMode, ReadPolicy,
        Symbol, Value, Version,
    },
};

/// The authored architecture and conformance contract (`SIM.md`).
///
/// The suite checks itself against this document so the claim list and the
/// executable assertions cannot drift apart. It is located from the crate
/// manifest and validated to be the conformance contract before use.
pub(crate) static CONFORMANCE_CONTRACT: LazyLock<String> = LazyLock::new(|| {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for candidate in [
        manifest_dir.join("../../SIM.md"),
        manifest_dir.join("../sim/SIM.md"),
        manifest_dir.join("../sim-sdk/SIM.md"),
    ] {
        if let Ok(text) = std::fs::read_to_string(&candidate)
            && text.contains("`sim-conformance`")
            && text.contains("public facade")
        {
            return text;
        }
    }
    panic!("could not locate the SIM.md conformance contract for sim-conformance")
});

pub(crate) fn cx() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    sim::runtime::install_core_runtime(&mut cx);
    sim::numbers_prelude::NumbersPreludeLib::new()
        .install_all(&mut cx)
        .unwrap();
    install_codecs(&mut cx);
    cx
}

fn install_codecs(cx: &mut Cx) {
    let lisp = sim::codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    let json = sim::codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&json).unwrap();
    let binary = sim::codec_binary::BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&binary).unwrap();
    let binary_base64 =
        sim::codec_binary_base64::BinaryBase64CodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&binary_base64).unwrap();
    let algol = sim::codec_algol::AlgolCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&algol).unwrap();
}

pub(crate) fn q(namespace: &str, name: &str) -> Symbol {
    Symbol::qualified(namespace, name)
}

pub(crate) fn codec_symbols() -> [Symbol; 5] {
    [
        q("codec", "lisp"),
        q("codec", "json"),
        q("codec", "binary"),
        q("codec", "binary-base64"),
        q("codec", "algol"),
    ]
}

pub(crate) fn expr_corpus() -> Vec<Expr> {
    let mut exprs = vec![
        Expr::Nil,
        Expr::Bool(true),
        Expr::Number(NumberLiteral {
            domain: q("numbers", "f64"),
            canonical: "42.5".to_owned(),
        }),
        Expr::Symbol(q("math", "pi")),
        Expr::Local(Symbol::new("arg0")),
        Expr::String("line\n\"quoted\"".to_owned()),
        Expr::Bytes(vec![0, 1, 2, 0xff]),
        Expr::List(vec![Expr::Symbol(Symbol::new("f")), Expr::Bool(false)]),
        Expr::Vector(vec![Expr::Number(NumberLiteral {
            domain: q("numbers", "i64"),
            canonical: "7".to_owned(),
        })]),
        Expr::Map(vec![
            (Expr::Symbol(Symbol::new("b")), Expr::Bool(false)),
            (Expr::Symbol(Symbol::new("a")), Expr::Bool(true)),
        ]),
        Expr::Set(vec![
            Expr::String("z".to_owned()),
            Expr::String("a".to_owned()),
        ]),
        Expr::Call {
            operator: Box::new(Expr::Symbol(q("math", "add"))),
            args: vec![Expr::Number(NumberLiteral {
                domain: q("numbers", "f64"),
                canonical: "1.25".to_owned(),
            })],
        },
        Expr::Infix {
            operator: Symbol::new("+"),
            left: Box::new(Expr::Number(NumberLiteral {
                domain: q("numbers", "f64"),
                canonical: "1.25".to_owned(),
            })),
            right: Box::new(Expr::Prefix {
                operator: Symbol::new("-"),
                arg: Box::new(Expr::Postfix {
                    operator: Symbol::new("!"),
                    arg: Box::new(Expr::Symbol(Symbol::new("n"))),
                }),
            }),
        },
        Expr::Block(vec![
            Expr::Symbol(Symbol::new("x")),
            Expr::String("done".to_owned()),
        ]),
        Expr::Annotated {
            expr: Box::new(Expr::Bool(true)),
            annotations: vec![(q("meta", "source"), Expr::String("sim".to_owned()))],
        },
        Expr::Extension {
            tag: q("demo", "escape"),
            payload: Box::new(Expr::Vector(vec![Expr::Bool(true)])),
        },
    ];
    exprs.extend(quote_modes().into_iter().map(|mode| Expr::Quote {
        mode,
        expr: Box::new(Expr::Symbol(Symbol::new(format!(
            "quote-target-{}",
            quote_mode_name(mode)
        )))),
    }));
    exprs
}

pub(crate) fn encode_once(cx: &mut Cx, codec: &Symbol, expr: &Expr) -> Output {
    encode_with_codec(cx, codec, expr, sim::kernel::EncodeOptions::default())
        .unwrap_or_else(|err| panic!("codec {codec} failed to encode {expr:?}: {err:?}"))
}

pub(crate) fn decode_once(cx: &mut Cx, codec: &Symbol, output: Output) -> Expr {
    let input = match output {
        Output::Text(text) => Input::Text(text),
        Output::Bytes(bytes) => Input::Bytes(bytes),
    };
    decode_with_codec(cx, codec, input.clone(), ReadPolicy::default())
        .unwrap_or_else(|err| panic!("codec {codec} failed to decode {input:?}: {err:?}"))
}

pub(crate) fn assert_expr_coverage(exprs: &[Expr]) {
    let mut variants = BTreeSet::new();
    let mut modes = BTreeSet::new();
    for expr in exprs {
        collect_expr_coverage(expr, &mut variants, &mut modes);
    }
    assert_eq!(
        variants,
        [
            "Nil",
            "Bool",
            "Number",
            "Symbol",
            "Local",
            "String",
            "Bytes",
            "List",
            "Vector",
            "Map",
            "Set",
            "Call",
            "Infix",
            "Prefix",
            "Postfix",
            "Block",
            "Quote",
            "Annotated",
            "Extension",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
    );
    assert_eq!(
        modes,
        quote_modes()
            .into_iter()
            .map(quote_mode_name)
            .collect::<BTreeSet<_>>()
    );
}

fn collect_expr_coverage(
    expr: &Expr,
    variants: &mut BTreeSet<&'static str>,
    modes: &mut BTreeSet<&'static str>,
) {
    variants.insert(variant_name(expr));
    match expr {
        Expr::Nil
        | Expr::Bool(_)
        | Expr::Number(_)
        | Expr::Symbol(_)
        | Expr::Local(_)
        | Expr::String(_)
        | Expr::Bytes(_) => {}
        Expr::List(items) | Expr::Vector(items) | Expr::Set(items) | Expr::Block(items) => {
            for item in items {
                collect_expr_coverage(item, variants, modes);
            }
        }
        Expr::Map(entries) => {
            for (key, value) in entries {
                collect_expr_coverage(key, variants, modes);
                collect_expr_coverage(value, variants, modes);
            }
        }
        Expr::Call { operator, args } => {
            collect_expr_coverage(operator, variants, modes);
            for arg in args {
                collect_expr_coverage(arg, variants, modes);
            }
        }
        Expr::Infix { left, right, .. } => {
            collect_expr_coverage(left, variants, modes);
            collect_expr_coverage(right, variants, modes);
        }
        Expr::Prefix { arg, .. } | Expr::Postfix { arg, .. } => {
            collect_expr_coverage(arg, variants, modes);
        }
        Expr::Quote { mode, expr } => {
            modes.insert(quote_mode_name(*mode));
            collect_expr_coverage(expr, variants, modes);
        }
        Expr::Annotated { expr, annotations } => {
            collect_expr_coverage(expr, variants, modes);
            for (_, value) in annotations {
                collect_expr_coverage(value, variants, modes);
            }
        }
        Expr::Extension { payload, .. } => collect_expr_coverage(payload, variants, modes),
    }
}

fn variant_name(expr: &Expr) -> &'static str {
    match expr {
        Expr::Nil => "Nil",
        Expr::Bool(_) => "Bool",
        Expr::Number(_) => "Number",
        Expr::Symbol(_) => "Symbol",
        Expr::Local(_) => "Local",
        Expr::String(_) => "String",
        Expr::Bytes(_) => "Bytes",
        Expr::List(_) => "List",
        Expr::Vector(_) => "Vector",
        Expr::Map(_) => "Map",
        Expr::Set(_) => "Set",
        Expr::Call { .. } => "Call",
        Expr::Infix { .. } => "Infix",
        Expr::Prefix { .. } => "Prefix",
        Expr::Postfix { .. } => "Postfix",
        Expr::Block(_) => "Block",
        Expr::Quote { .. } => "Quote",
        Expr::Annotated { .. } => "Annotated",
        Expr::Extension { .. } => "Extension",
    }
}

fn quote_modes() -> [QuoteMode; 5] {
    [
        QuoteMode::Quote,
        QuoteMode::QuasiQuote,
        QuoteMode::Unquote,
        QuoteMode::Splice,
        QuoteMode::Syntax,
    ]
}

fn quote_mode_name(mode: QuoteMode) -> &'static str {
    match mode {
        QuoteMode::Quote => "Quote",
        QuoteMode::QuasiQuote => "QuasiQuote",
        QuoteMode::Unquote => "Unquote",
        QuoteMode::Splice => "Splice",
        QuoteMode::Syntax => "Syntax",
    }
}

pub(crate) fn assert_lattice_reaches(cx: &Cx, from: Symbol, to: Symbol) {
    let mut graph = cx
        .registry()
        .promotion_rules()
        .iter()
        .map(|rule| (rule.from_domain.clone(), rule.to_domain.clone()))
        .collect::<BTreeSet<_>>();
    graph.extend(
        cx.registry()
            .value_promotion_rules()
            .iter()
            .map(|rule| (rule.from_domain.clone(), rule.to_domain.clone())),
    );
    assert!(reaches(&graph, &from, &to), "expected {from} to reach {to}");
}

fn reaches(graph: &BTreeSet<(Symbol, Symbol)>, from: &Symbol, to: &Symbol) -> bool {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([from.clone()]);
    while let Some(domain) = queue.pop_front() {
        if &domain == to {
            return true;
        }
        if !seen.insert(domain.clone()) {
            continue;
        }
        for (edge_from, edge_to) in graph {
            if edge_from == &domain {
                queue.push_back(edge_to.clone());
            }
        }
    }
    false
}

pub(crate) fn table_value<'a>(expr: &'a Expr, key: &Symbol) -> Option<&'a Expr> {
    let Expr::Map(entries) = expr else {
        return None;
    };
    entries
        .iter()
        .find_map(|(candidate, value)| match candidate {
            Expr::Symbol(symbol) if symbol == key => Some(value),
            _ => None,
        })
}

pub(crate) fn marker_symbol() -> Symbol {
    Symbol::new("ConformanceMarker")
}

pub(crate) fn marker_class_lib(cx: &mut Cx) -> sim::classes::NativeClassLib {
    let constructor = sim::functions::FunctionObject::new(
        cx.registry_mut().fresh_function_id(),
        marker_symbol(),
        vec![sim::functions::FunctionCase {
            id: cx.registry_mut().fresh_case_id(),
            name: Symbol::new("conformance-marker-new"),
            args: Arc::new(sim::shape::ListShape::new(Vec::new())),
            result: None,
            demand: Vec::new(),
            priority: 10,
            implementation: marker_constructor,
        }],
    );
    let class = sim::classes::NativeClass::new(
        cx.registry_mut().fresh_class_id(),
        marker_symbol(),
        constructor,
        Some(Arc::new(sim::shape::AnyShape)),
        Vec::new(),
    );
    sim::classes::NativeClassLib::from_class(q("conformance", "marker-lib"), &class, "0.1.0")
}

fn marker_constructor(
    cx: &mut Cx,
    _prepared: &PreparedArgs,
    _bindings: sim::shape::Bindings,
) -> sim::kernel::Result<Value> {
    cx.factory()
        .opaque(Arc::new(sim::classes::ClassInstance::new(
            marker_symbol(),
            Vec::new(),
            Vec::new(),
        )))
}

pub(crate) fn assert_loader_selected(result: sim::kernel::Result<Box<dyn Lib>>, label: &str) {
    let Err(err) = result else {
        panic!("fixture source for {label} loaded unexpectedly");
    };
    let text = format!("{err:?}");
    assert!(
        !text.contains("no loader accepted"),
        "{label} was not accepted by any loader: {text}"
    );
}

pub(crate) struct StubWasmExportsLib {
    pub(crate) exports: Vec<sim::wasm_abi::WasmExport>,
}

impl Lib for StubWasmExportsLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: q("wasm-test", "abi"),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::WasmComponent,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: self
                .exports
                .iter()
                .map(|export| export.to_export())
                .collect(),
        }
    }

    fn load(&self, cx: &mut LoadCx, linker: &mut Linker<'_>) -> sim::kernel::Result<()> {
        sim::wasm_abi::register_stub_exports(cx, linker, &self.exports)
    }
}

pub(crate) fn assert_export_state(
    lib: &sim::kernel::LoadedLib,
    kind: &'static str,
    symbol: Symbol,
    predicate: impl FnOnce(&ExportState) -> bool,
) {
    let kind = ExportKind::named(kind);
    let record = lib
        .exports
        .iter()
        .find(|record| record.kind == kind && record.symbol == symbol)
        .unwrap_or_else(|| panic!("missing export record for {symbol}"));
    assert!(
        predicate(&record.state),
        "unexpected export state for {symbol}: {:?}",
        record.state
    );
}
