use std::sync::Arc;

use sim_kernel::{Args, Cx, DefaultFactory, EagerPolicy, Expr, Symbol, Value};

use crate::{
    install_skill_lib,
    lib_skill::{FixtureBehavior, FixtureSkillSpec, FixtureTransport, SkillCard},
    runtime::install_core_runtime,
    shape::{AnyShape, shape_value},
};

#[test]
fn skill_core_functions_have_browse_help_and_examples() {
    let mut cx = test_cx();
    install_skill_lib(&mut cx).unwrap();

    for symbol in skill_core_symbols() {
        let subject = symbol_value(&cx, symbol.clone());
        let card = call(&mut cx, Symbol::qualified("core", "browse"), vec![subject]);
        let card = expr(&mut cx, &card);
        let help = table_value(&card, "help").expect("help");
        assert!(
            matches!(help, Expr::Map(_)),
            "{symbol} should have fixed Help"
        );

        let subject = symbol_value(&cx, symbol.clone());
        let examples = call(
            &mut cx,
            Symbol::qualified("core", "examples"),
            vec![subject],
        );
        let examples = expr(&mut cx, &examples);
        let Expr::List(items) = examples else {
            panic!("{symbol} examples should be a list");
        };
        assert_eq!(items.len(), 1, "{symbol} should have one authored example");
    }
}

#[test]
fn bound_skill_card_is_reachable_from_root_browse_catalog() {
    let mut cx = test_cx();
    install_fixture(&mut cx, "math.add", "math", "add");

    let root = symbol_value(&cx, Symbol::qualified("browse", "catalog"));
    let target = symbol_value(&cx, Symbol::qualified("skill", "math.add"));
    let path = call(
        &mut cx,
        Symbol::qualified("core", "browse-path"),
        vec![root, target],
    );
    let path = expr(&mut cx, &path);
    let Expr::List(items) = path else {
        panic!("skill card should be reachable from root");
    };
    assert_eq!(
        items.last(),
        Some(&Expr::Symbol(Symbol::qualified("skill", "math.add")))
    );

    let target = symbol_value(&cx, Symbol::qualified("skill", "math.add"));
    let card = call(&mut cx, Symbol::qualified("core", "browse"), vec![target]);
    let card = expr(&mut cx, &card);
    assert_eq!(
        table_value(&card, "kind"),
        Some(&Expr::Symbol(Symbol::qualified("skill", "card")))
    );
}

#[test]
fn public_browse_card_does_not_expose_transport_handles() {
    let mut cx = test_cx();
    install_fixture(
        &mut cx,
        "safe.echo",
        "endpoint_ref_should_not_appear",
        "credential_scope_should_not_appear",
    );

    let subject = symbol_value(&cx, Symbol::qualified("skill", "safe.echo"));
    let card = call(&mut cx, Symbol::qualified("core", "browse"), vec![subject]);
    let card = expr(&mut cx, &card);

    assert!(!expr_contains_string(
        &card,
        "endpoint_ref_should_not_appear"
    ));
    assert!(!expr_contains_string(
        &card,
        "credential_scope_should_not_appear"
    ));
}

#[cfg(all(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64"
))]
#[test]
fn skill_card_expr_round_trips_through_installed_codecs() {
    let mut cx = test_cx();
    install_codecs(&mut cx);
    let card = skill_card("math.add", "math", "add");
    let expr = card.to_expr(&mut cx).unwrap();

    for codec in [
        Symbol::qualified("codec", "lisp"),
        Symbol::qualified("codec", "json"),
        Symbol::qualified("codec", "binary"),
        Symbol::qualified("codec", "binary-base64"),
    ] {
        let decoded = codec_roundtrip(&mut cx, &codec, &expr);
        assert!(
            decoded.canonical_eq(&expr),
            "codec {codec} changed SkillCard expr {decoded:?}"
        );
        let rebuilt = SkillCard::from_expr(&decoded).unwrap();
        assert!(rebuilt.to_expr(&mut cx).unwrap().canonical_eq(&decoded));
    }
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn install_fixture(cx: &mut Cx, id: &str, transport_id: &str, operation: &str) {
    let fixture = Arc::new(FixtureTransport::new(transport_id));
    fixture
        .insert(operation, FixtureBehavior::EchoArgs)
        .unwrap();
    crate::lib_skill::install_fixture_skill(cx, fixture, skill_card(id, transport_id, operation))
        .unwrap();
}

fn skill_card(id: &str, transport_id: &str, operation: &str) -> SkillCard {
    SkillCard::fixture(FixtureSkillSpec {
        id: id.to_owned(),
        symbol: Symbol::qualified("skill", id.to_owned()),
        title: "Fixture Skill".to_owned(),
        description: "Safe public fixture skill descriptor.".to_owned(),
        input_shape: any_shape(),
        output_shape: any_shape(),
        transport_id: transport_id.to_owned(),
        operation: operation.to_owned(),
    })
}

fn any_shape() -> Value {
    shape_value(Symbol::new("Any"), Arc::new(AnyShape))
}

fn skill_core_symbols() -> Vec<Symbol> {
    vec![
        Symbol::qualified("skill", "install"),
        Symbol::qualified("skill", "bind"),
        Symbol::qualified("skill", "list"),
        Symbol::qualified("skill", "card"),
        Symbol::qualified("skill", "call"),
    ]
}

fn call(cx: &mut Cx, symbol: Symbol, args: Vec<Value>) -> Value {
    cx.call_function(&symbol, Args::new(args))
        .unwrap_or_else(|err| panic!("{symbol} failed: {err}"))
}

fn symbol_value(cx: &Cx, symbol: Symbol) -> Value {
    cx.factory().symbol(symbol).unwrap()
}

fn expr(cx: &mut Cx, value: &Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

fn table_value<'a>(expr: &'a Expr, field: &str) -> Option<&'a Expr> {
    let Expr::Map(entries) = expr else {
        panic!("expected map");
    };
    let field = Symbol::new(field.to_owned());
    entries.iter().find_map(|(key, value)| match key {
        Expr::Symbol(symbol) if symbol == &field => Some(value),
        _ => None,
    })
}

fn expr_contains_string(expr: &Expr, needle: &str) -> bool {
    match expr {
        Expr::String(text) => text == needle,
        Expr::List(items) | Expr::Vector(items) | Expr::Set(items) => {
            items.iter().any(|item| expr_contains_string(item, needle))
        }
        Expr::Map(entries) => entries.iter().any(|(key, value)| {
            expr_contains_string(key, needle) || expr_contains_string(value, needle)
        }),
        Expr::Call { operator, args } => {
            expr_contains_string(operator, needle)
                || args.iter().any(|arg| expr_contains_string(arg, needle))
        }
        Expr::Infix { left, right, .. } => {
            expr_contains_string(left, needle) || expr_contains_string(right, needle)
        }
        Expr::Prefix { arg, .. } | Expr::Postfix { arg, .. } => expr_contains_string(arg, needle),
        Expr::Block(items) => items.iter().any(|item| expr_contains_string(item, needle)),
        Expr::Quote { expr, .. } | Expr::Extension { payload: expr, .. } => {
            expr_contains_string(expr, needle)
        }
        Expr::Annotated { expr, annotations } => {
            expr_contains_string(expr, needle)
                || annotations
                    .iter()
                    .any(|(_, value)| expr_contains_string(value, needle))
        }
        Expr::Nil
        | Expr::Bool(_)
        | Expr::Number(_)
        | Expr::Symbol(_)
        | Expr::Local(_)
        | Expr::Bytes(_) => false,
    }
}

#[cfg(all(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64"
))]
fn install_codecs(cx: &mut Cx) {
    let lisp = crate::codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    let json = crate::codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&json).unwrap();
    let binary = crate::codec_binary::BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&binary).unwrap();
    let binary_base64 =
        crate::codec_binary_base64::BinaryBase64CodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&binary_base64).unwrap();
}

#[cfg(all(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64"
))]
fn codec_roundtrip(cx: &mut Cx, codec: &Symbol, expr: &Expr) -> Expr {
    let output =
        sim_codec::encode_with_codec(cx, codec, expr, sim_kernel::EncodeOptions::default())
            .unwrap();
    let input = match output {
        sim_codec::Output::Text(text) => sim_codec::Input::Text(text),
        sim_codec::Output::Bytes(bytes) => sim_codec::Input::Bytes(bytes),
    };
    sim_codec::decode_with_codec(cx, codec, input, sim_kernel::ReadPolicy::default()).unwrap()
}
