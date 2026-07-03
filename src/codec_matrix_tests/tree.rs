use sim_codec::{
    Input, Output, decode_located_with_codec, decode_tree_with_codec, encode_located_with_codec,
    encode_tree_with_codec,
};
use sim_kernel::{
    EncodeOptions, Expr, LocatedExpr, Origin, ReadPolicy, SourceId, Span, Symbol, Trivia,
};

use super::support::cx;

#[test]
fn generic_located_decode_uses_codec_specific_origin_support() {
    let mut cx = cx();

    let lisp = decode_located_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text(" ; c\n(+ 1 2)".to_owned()),
        ReadPolicy::default(),
        "demo.lisp",
    )
    .unwrap();
    assert_eq!(
        lisp.origin.unwrap().source,
        SourceId("demo.lisp".to_owned())
    );

    let json = decode_located_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "json"),
        Input::Text("{\"$expr\":\"string\",\"value\":\"x\"}".to_owned()),
        ReadPolicy::default(),
        "ignored.json",
    )
    .unwrap();
    assert!(json.origin.is_none());
}

#[test]
fn generic_located_encode_respects_lossless_origin_policy() {
    let mut cx = cx();
    let located = LocatedExpr {
        expr: Expr::String("x".to_owned()),
        origin: Some(Origin {
            codec: sim_kernel::CodecId(1),
            source: SourceId("origin.txt".to_owned()),
            span: Span { start: 1, end: 2 },
            trivia: vec![Trivia::Whitespace(" ".to_owned())],
        }),
    };

    let json_lossless = encode_located_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "json"),
        &located,
        EncodeOptions {
            lossless_origin: true,
            ..Default::default()
        },
    )
    .unwrap()
    .into_text()
    .unwrap();
    assert!(json_lossless.contains("\"$located\""));
    assert!(json_lossless.contains("\"origin\""));

    let json_lossy = encode_located_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "json"),
        &located,
        EncodeOptions::default(),
    )
    .unwrap()
    .into_text()
    .unwrap();
    assert!(!json_lossy.contains("\"$located\""));

    let lisp = encode_located_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        &located,
        EncodeOptions {
            lossless_origin: true,
            ..Default::default()
        },
    )
    .unwrap()
    .into_text()
    .unwrap();
    assert_eq!(lisp, "\"x\"");
}

#[test]
fn generic_tree_decode_exposes_recursive_structure() {
    let mut cx = cx();

    let lisp = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text("(f [1 2] true)".to_owned()),
        ReadPolicy::default(),
        "tree.lisp",
    )
    .unwrap();
    assert!(matches!(lisp.expr, Expr::List(_)));
    assert_eq!(lisp.children.len(), 3);
    assert!(lisp.origin.is_some());
    assert_eq!(lisp.origin.as_ref().unwrap().span.start, 0);
    assert_eq!(lisp.origin.as_ref().unwrap().span.end, 14);
    assert_eq!(lisp.children[0].origin.as_ref().unwrap().span.start, 1);
    assert_eq!(lisp.children[0].origin.as_ref().unwrap().span.end, 2);
    assert_eq!(lisp.children[1].origin.as_ref().unwrap().span.start, 3);
    assert_eq!(lisp.children[1].origin.as_ref().unwrap().span.end, 8);

    let algol = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "algol"),
        Input::Text("1 + 2 * 3".to_owned()),
        ReadPolicy::default(),
        "tree.alg",
    )
    .unwrap();
    assert!(matches!(algol.expr, Expr::Infix { .. }));
    assert_eq!(algol.children.len(), 2);
    assert!(matches!(algol.children[1].expr, Expr::Infix { .. }));
    assert_eq!(algol.origin.as_ref().unwrap().span.start, 0);
    assert_eq!(algol.origin.as_ref().unwrap().span.end, 9);
    assert_eq!(algol.children[0].origin.as_ref().unwrap().span.start, 0);
    assert_eq!(algol.children[0].origin.as_ref().unwrap().span.end, 1);
    assert_eq!(algol.children[1].origin.as_ref().unwrap().span.start, 4);
    assert_eq!(algol.children[1].origin.as_ref().unwrap().span.end, 9);

    let json = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "json"),
        Input::Text(
            "{\"$expr\":\"list\",\"items\":[{\"$expr\":\"bool\",\"value\":true}]}".to_owned(),
        ),
        ReadPolicy::default(),
        "tree.json",
    )
    .unwrap();
    assert!(matches!(json.expr, Expr::List(_)));
    assert_eq!(json.children.len(), 1);
    assert!(matches!(json.children[0].expr, Expr::Bool(true)));

    let json_located = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "json"),
        Input::Text(
            "{\"$expr\":\"list\",\"items\":[{\"$located\":{\"$expr\":\"string\",\"value\":\"x\"},\"origin\":{\"codec\":7,\"source\":\"tree.json\",\"span\":{\"start\":2,\"end\":5},\"trivia\":[],\"raw\":[120]}}]}".to_owned(),
        ),
        ReadPolicy::default(),
        "tree.json",
    )
    .unwrap();
    assert_eq!(json_located.children.len(), 1);
    assert_eq!(
        json_located.children[0].origin.as_ref().unwrap().source,
        SourceId("tree.json".to_owned())
    );
    assert_eq!(
        json_located.children[0].origin.as_ref().unwrap().span.start,
        2
    );
    assert_eq!(
        json_located.children[0].origin.as_ref().unwrap().span.end,
        5
    );
}

#[test]
fn generic_tree_encode_preserves_nested_origin_for_lossless_codecs() {
    let mut cx = cx();
    let tree = sim_kernel::LocatedExprTree {
        expr: Expr::List(vec![Expr::String("x".to_owned())]),
        origin: Some(Origin {
            codec: sim_kernel::CodecId(7),
            source: SourceId("root".to_owned()),
            span: Span { start: 0, end: 3 },
            trivia: vec![Trivia::Whitespace(" ".to_owned())],
        }),
        children: vec![sim_kernel::LocatedExprTree::without_children(
            Expr::String("x".to_owned()),
            Some(Origin {
                codec: sim_kernel::CodecId(7),
                source: SourceId("child".to_owned()),
                span: Span { start: 1, end: 2 },
                trivia: vec![Trivia::LineComment("; child".to_owned())],
            }),
        )],
    };

    let json = encode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "json"),
        &tree,
        EncodeOptions {
            lossless_origin: true,
            ..Default::default()
        },
    )
    .unwrap()
    .into_text()
    .unwrap();
    let json_roundtrip = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "json"),
        Input::Text(json),
        ReadPolicy::default(),
        "tree.json",
    )
    .unwrap();
    assert_eq!(
        json_roundtrip.children[0].origin.as_ref().unwrap().source,
        SourceId("child".to_owned())
    );

    let binary = encode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "binary"),
        &tree,
        EncodeOptions {
            lossless_origin: true,
            ..Default::default()
        },
    )
    .unwrap();
    let binary_roundtrip = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "binary"),
        match binary {
            Output::Bytes(bytes) => Input::Bytes(bytes),
            Output::Text(text) => Input::Text(text),
        },
        ReadPolicy::default(),
        "tree.bin",
    )
    .unwrap();
    assert_eq!(
        binary_roundtrip.children[0].origin.as_ref().unwrap().source,
        SourceId("child".to_owned())
    );

    let binary_base64 = encode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "binary-base64"),
        &tree,
        EncodeOptions {
            lossless_origin: true,
            ..Default::default()
        },
    )
    .unwrap();
    let binary_base64_roundtrip = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "binary-base64"),
        match binary_base64 {
            Output::Bytes(bytes) => Input::Bytes(bytes),
            Output::Text(text) => Input::Text(text),
        },
        ReadPolicy::default(),
        "tree.simb64",
    )
    .unwrap();
    assert_eq!(
        binary_base64_roundtrip.children[0]
            .origin
            .as_ref()
            .unwrap()
            .source,
        SourceId("child".to_owned())
    );

    #[cfg(feature = "codec-bitwise")]
    {
        let bitwise = encode_tree_with_codec(
            &mut cx,
            &Symbol::qualified("codec", "bitwise"),
            &tree,
            EncodeOptions {
                lossless_origin: true,
                ..Default::default()
            },
        )
        .unwrap();
        let bitwise_roundtrip = decode_tree_with_codec(
            &mut cx,
            &Symbol::qualified("codec", "bitwise"),
            match bitwise {
                Output::Bytes(bytes) => Input::Bytes(bytes),
                Output::Text(text) => Input::Text(text),
            },
            ReadPolicy::default(),
            "tree.bit",
        )
        .unwrap();
        assert_eq!(
            bitwise_roundtrip.children[0].origin.as_ref().unwrap().source,
            SourceId("child".to_owned())
        );
    }

    #[cfg(feature = "codec-bitwise-base64")]
    {
        let bitwise_base64 = encode_tree_with_codec(
            &mut cx,
            &Symbol::qualified("codec", "bitwise-base64"),
            &tree,
            EncodeOptions {
                lossless_origin: true,
                ..Default::default()
            },
        )
        .unwrap();
        let bitwise_base64_roundtrip = decode_tree_with_codec(
            &mut cx,
            &Symbol::qualified("codec", "bitwise-base64"),
            match bitwise_base64 {
                Output::Bytes(bytes) => Input::Bytes(bytes),
                Output::Text(text) => Input::Text(text),
            },
            ReadPolicy::default(),
            "tree.bitb64",
        )
        .unwrap();
        assert_eq!(
            bitwise_base64_roundtrip.children[0]
                .origin
                .as_ref()
                .unwrap()
                .source,
            SourceId("child".to_owned())
        );
    }
}

#[test]
fn generic_tree_encode_preserves_text_codec_leading_trivia_best_effort() {
    let mut cx = cx();

    let lisp_tree = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text("(f ; note\n [1 2])".to_owned()),
        ReadPolicy::default(),
        "tree.lisp",
    )
    .unwrap();
    let lisp_encoded = encode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        &lisp_tree,
        EncodeOptions {
            lossless_origin: true,
            ..Default::default()
        },
    )
    .unwrap()
    .into_text()
    .unwrap();
    let lisp_roundtrip = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text(lisp_encoded),
        ReadPolicy::default(),
        "tree.lisp",
    )
    .unwrap();
    assert!(
        lisp_roundtrip.children[1]
            .origin
            .as_ref()
            .unwrap()
            .trivia
            .iter()
            .any(|item| matches!(item, Trivia::LineComment(text) if text.contains("note")))
    );

    let algol_tree = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "algol"),
        Input::Text("1 + /* note */ 2".to_owned()),
        ReadPolicy::default(),
        "tree.alg",
    )
    .unwrap();
    let algol_encoded = encode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "algol"),
        &algol_tree,
        EncodeOptions {
            lossless_origin: true,
            ..Default::default()
        },
    )
    .unwrap()
    .into_text()
    .unwrap();
    let algol_roundtrip = decode_tree_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "algol"),
        Input::Text(algol_encoded),
        ReadPolicy::default(),
        "tree.alg",
    )
    .unwrap();
    assert!(
        algol_roundtrip.children[1]
            .origin
            .as_ref()
            .unwrap()
            .trivia
            .iter()
            .any(|item| matches!(item, Trivia::BlockComment(text) if text.contains("note")))
    );
}
