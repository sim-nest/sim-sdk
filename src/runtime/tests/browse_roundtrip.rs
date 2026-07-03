use std::sync::Arc;

use sim_kernel::{
    Args, ContentId, Cx, Datum, DatumStore, DefaultFactory, EagerPolicy, Expr, Ref, Symbol, Value,
    card::ref_value, datum_content_algorithm,
};

use crate::runtime::{
    SimTest, TestExpected,
    browse::schema::{
        CoverageBuilder, FacetBuilder, HelpBuilder, RedactionBuilder, TestReportBuilder,
    },
    install_core_runtime,
};

use super::support::table_value;

#[test]
fn content_ref_browse_resolves_stored_datum_value() {
    let mut cx = test_cx();
    let id = cx
        .datum_store_mut()
        .intern(Datum::String("stored browse prose".to_owned()))
        .unwrap();
    let subject = ref_value(&mut cx, &Ref::Content(id)).unwrap();

    let card = cx
        .call_function(
            &Symbol::qualified("core", "browse"),
            Args::new(vec![subject]),
        )
        .unwrap();
    let expr = card.object().as_expr(&mut cx).unwrap();

    assert_eq!(
        table_value(&expr, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("browse", "content")))
    );
    let content = facet_payload(&expr, &Symbol::qualified("browse", "content"));
    assert_eq!(
        table_value(content, &Symbol::new("datum-kind")),
        Some(&Expr::Symbol(Symbol::qualified("datum", "string")))
    );
    assert_eq!(
        table_value(content, &Symbol::new("value")),
        Some(&Expr::String("stored browse prose".to_owned()))
    );
}

#[test]
fn missing_content_ref_browses_as_missing_ref_card() {
    let mut cx = test_cx();
    let id = ContentId::from_bytes(datum_content_algorithm(), [7; 32]);
    let subject = ref_value(&mut cx, &Ref::Content(id)).unwrap();

    let card = cx
        .call_function(
            &Symbol::qualified("core", "browse"),
            Args::new(vec![subject]),
        )
        .unwrap();
    let expr = card.object().as_expr(&mut cx).unwrap();

    assert_eq!(
        table_value(&expr, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("browse", "missing-ref")))
    );
    let missing = facet_payload(&expr, &Symbol::qualified("browse", "missing-ref"));
    assert!(matches!(
        table_value(missing, &Symbol::new("missing-ref")),
        Some(Expr::Extension { tag, .. }) if tag == &Symbol::qualified("core", "ref")
    ));
    assert_eq!(
        table_value(&expr, &Symbol::new("shape-known")),
        Some(&Expr::Bool(false))
    );
}

#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-algol"
))]
#[test]
fn browse_schema_values_roundtrip_through_enabled_codecs() {
    let mut cx = test_cx();
    let codecs = install_enabled_codecs(&mut cx);
    let samples = schema_samples(&mut cx);

    for (label, expr) in samples {
        for codec in &codecs {
            let decoded = codec_roundtrip(&mut cx, codec, &expr);
            assert!(
                decoded.canonical_eq(&expr),
                "{label} did not round-trip through {codec}: {decoded:?}"
            );
        }
    }
}

#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-algol"
))]
#[test]
fn test_expr_and_expected_codec_ids_survive_codec_roundtrips() {
    let mut cx = test_cx();
    let codecs = install_enabled_codecs(&mut cx);
    let expr = registered_codec_id_test_expr(&mut cx);
    assert_codec_id_fields(&expr);

    for codec in &codecs {
        let decoded = codec_roundtrip(&mut cx, codec, &expr);
        assert_codec_id_fields(&decoded);
    }
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn facet_payload<'a>(card: &'a Expr, name: &Symbol) -> &'a Expr {
    let Some(Expr::List(facets)) = table_value(card, &Symbol::new("facets")) else {
        panic!("Card should contain facets");
    };
    facets
        .iter()
        .find_map(|facet| {
            let facet_name = table_value(facet, &Symbol::new("name"))?;
            (facet_name == &Expr::Symbol(name.clone()))
                .then(|| table_value(facet, &Symbol::new("value")).expect("facet value"))
        })
        .unwrap_or_else(|| panic!("missing facet {name}"))
}

#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-algol"
))]
fn schema_samples(cx: &mut Cx) -> Vec<(&'static str, Expr)> {
    let subject = cx
        .factory()
        .symbol(Symbol::qualified("core", "help"))
        .unwrap();
    let card = cx
        .call_function(
            &Symbol::qualified("core", "browse"),
            Args::new(vec![subject]),
        )
        .unwrap();
    let help_subject = cx
        .factory()
        .symbol(Symbol::qualified("core", "help"))
        .unwrap();
    let help = HelpBuilder::new(help_subject).build(cx).unwrap();
    let test = registered_codec_id_test_expr(cx);
    let coverage = CoverageBuilder {
        tests: 1,
        examples: 1,
        runnable: 1,
        passed: Some(1),
        failed: Some(0),
        skipped: Some(0),
        last_run: None,
        stale: false,
    }
    .build(cx)
    .unwrap();
    let mut facet_builder = FacetBuilder::new(Symbol::qualified("browse", "codec-roundtrip"));
    facet_builder.kind = Symbol::new("roundtrip");
    facet_builder.value = Some(
        cx.factory()
            .string("facet payload survives codec data mode".to_owned())
            .unwrap(),
    );
    let facet = facet_builder.build(cx).unwrap();
    let mut redaction_builder = RedactionBuilder::unavailable();
    redaction_builder.summary = "hidden payload".to_owned();
    let redaction = redaction_builder.build(cx).unwrap();
    let mut report_builder = TestReportBuilder::new(Symbol::qualified("test", "codec-ids"));
    report_builder.passed = true;
    report_builder.mode = Symbol::qualified("test", "value");
    report_builder.detail = Some("passed".to_owned());
    let report = report_builder.build(cx).unwrap();

    vec![
        ("Card", value_expr(cx, card)),
        ("Help", value_expr(cx, help)),
        ("Test", test),
        ("Coverage", value_expr(cx, coverage)),
        ("Facet", value_expr(cx, facet)),
        ("Redaction", value_expr(cx, redaction)),
        ("TestReport", value_expr(cx, report)),
    ]
}

#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-algol"
))]
fn registered_codec_id_test_expr(cx: &mut Cx) -> Expr {
    let name = Symbol::qualified("test", "codec-ids");
    let test = SimTest::new(
        name.clone(),
        Symbol::qualified("test", "runtime"),
        Expr::List(vec![
            Expr::Symbol(Symbol::qualified("core", "quote")),
            Expr::String("lisp-authored expr".to_owned()),
        ]),
        TestExpected::Value(Expr::Map(vec![(
            Expr::Symbol(Symbol::new("expected")),
            Expr::String("json-authored expected".to_owned()),
        )])),
        vec![Symbol::qualified("core", "browse")],
    )
    .with_expr_codec(Symbol::qualified("codec", "lisp"))
    .with_expected_codec(Symbol::qualified("codec", "json"));
    cx.registry_mut()
        .register_test(
            name.clone(),
            Symbol::qualified("test", "runtime"),
            Arc::new(test),
            vec![Symbol::qualified("core", "browse")],
        )
        .unwrap();

    let tests = cx
        .call_function(&Symbol::qualified("core", "tests"), Args::new(Vec::new()))
        .unwrap();
    let Expr::List(entries) = tests.object().as_expr(cx).unwrap() else {
        panic!("core/tests should return a list");
    };
    entries
        .into_iter()
        .find(|entry| table_value(entry, &Symbol::new("name")) == Some(&Expr::Symbol(name.clone())))
        .expect("registered codec id test")
}

#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-algol"
))]
fn assert_codec_id_fields(expr: &Expr) {
    assert_eq!(
        table_value(expr, &Symbol::new("expr-codec")),
        Some(&Expr::Symbol(Symbol::qualified("codec", "lisp")))
    );
    assert_eq!(
        table_value(expr, &Symbol::new("expected-codec")),
        Some(&Expr::Symbol(Symbol::qualified("codec", "json")))
    );
}

#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-algol"
))]
fn value_expr(cx: &mut Cx, value: Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-algol"
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

#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-algol"
))]
fn install_enabled_codecs(cx: &mut Cx) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    #[cfg(feature = "codec-lisp")]
    {
        let symbol = Symbol::qualified("codec", "lisp");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib =
                crate::codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
            cx.load_lib(&lib).unwrap();
        }
        symbols.push(symbol);
    }

    #[cfg(feature = "codec-json")]
    {
        let symbol = Symbol::qualified("codec", "json");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib = crate::codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
            cx.load_lib(&lib).unwrap();
        }
        symbols.push(symbol);
    }

    #[cfg(feature = "codec-binary")]
    {
        let symbol = Symbol::qualified("codec", "binary");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib = crate::codec_binary::BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
            cx.load_lib(&lib).unwrap();
        }
        symbols.push(symbol);
    }

    #[cfg(feature = "codec-binary-base64")]
    {
        let symbol = Symbol::qualified("codec", "binary-base64");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib = crate::codec_binary_base64::BinaryBase64CodecLib::new(
                cx.registry_mut().fresh_codec_id(),
            );
            cx.load_lib(&lib).unwrap();
        }
        symbols.push(symbol);
    }

    #[cfg(feature = "codec-algol")]
    {
        let symbol = Symbol::qualified("codec", "algol");
        if cx.registry().codec_by_symbol(&symbol).is_none() {
            let lib = crate::codec_algol::AlgolCodecLib::new(cx.registry_mut().fresh_codec_id());
            cx.load_lib(&lib).unwrap();
        }
        symbols.push(symbol);
    }

    assert!(!symbols.is_empty());
    symbols
}
