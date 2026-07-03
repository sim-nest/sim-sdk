use std::collections::BTreeSet;

use sim_kernel::{Expr, Symbol, catalog::CatalogSnapshot};

use super::support::{
    codec_symbols, corpus, cx, decode_once, encode_once, generated_expr_corpus, quote_mode_name,
    quote_modes, variant_name,
};

#[test]
fn every_codec_roundtrips_the_shared_corpus() {
    let mut cx = cx();
    for codec in codec_symbols() {
        for expr in corpus() {
            let encoded = encode_once(&mut cx, &codec, &expr);
            let decoded = decode_once(&mut cx, &codec, encoded);
            assert!(
                decoded.canonical_eq(&expr),
                "codec {} failed to roundtrip {:?} -> {:?}",
                codec,
                expr,
                decoded
            );
        }
    }
}

#[test]
fn registry_catalog_snapshot_expr_roundtrips_through_snapshot_codecs() {
    let mut cx = cx();
    let expr = cx.registry().catalog_snapshot().to_expr();

    for codec in [
        Symbol::qualified("codec", "lisp"),
        Symbol::qualified("codec", "json"),
        Symbol::qualified("codec", "binary"),
        Symbol::qualified("codec", "binary-base64"),
    ] {
        let encoded = encode_once(&mut cx, &codec, &expr);
        let decoded = decode_once(&mut cx, &codec, encoded);
        assert!(
            decoded.canonical_eq(&expr),
            "codec {} failed to roundtrip registry catalog snapshot",
            codec
        );
        assert_eq!(CatalogSnapshot::from_expr(decoded).unwrap().to_expr(), expr);
    }
}

#[test]
fn cross_codec_transcodes_preserve_semantics() {
    let mut cx = cx();
    for source in codec_symbols() {
        for target in codec_symbols() {
            for expr in corpus() {
                let source_output = encode_once(&mut cx, &source, &expr);
                let shared = decode_once(&mut cx, &source, source_output);
                let target_output = encode_once(&mut cx, &target, &shared);
                let final_expr = decode_once(&mut cx, &target, target_output);
                assert!(
                    final_expr.canonical_eq(&expr),
                    "transcode {} -> {} changed {:?} -> {:?}",
                    source,
                    target,
                    expr,
                    final_expr
                );
            }
        }
    }
}

#[test]
fn generated_exprs_roundtrip_and_transcode_across_every_general_codec() {
    let mut cx = cx();
    let exprs = generated_expr_corpus();
    assert_generator_coverage(&exprs);

    for codec in codec_symbols() {
        for expr in &exprs {
            let encoded = encode_once(&mut cx, &codec, expr);
            let decoded = decode_once(&mut cx, &codec, encoded);
            assert_eq!(
                decoded, *expr,
                "codec {} failed generated roundtrip {:?} -> {:?}",
                codec, expr, decoded
            );
        }
    }

    for source in codec_symbols() {
        for target in codec_symbols() {
            for expr in &exprs {
                let source_output = encode_once(&mut cx, &source, expr);
                let shared = decode_once(&mut cx, &source, source_output);
                let target_output = encode_once(&mut cx, &target, &shared);
                let final_expr = decode_once(&mut cx, &target, target_output);
                assert_eq!(
                    final_expr, *expr,
                    "generated transcode {} -> {} changed {:?} -> {:?}",
                    source, target, expr, final_expr
                );
            }
        }
    }
}

#[test]
fn repeated_encodes_are_stable() {
    let mut cx = cx();
    for codec in codec_symbols() {
        for expr in corpus() {
            let left = encode_once(&mut cx, &codec, &expr);
            let right = encode_once(&mut cx, &codec, &expr);
            assert_eq!(left, right, "codec {} changed output for {:?}", codec, expr);
        }
    }
}

fn assert_generator_coverage(exprs: &[Expr]) {
    let mut variants = BTreeSet::new();
    let mut modes = BTreeSet::new();
    for expr in exprs {
        collect_coverage(expr, &mut variants, &mut modes);
    }

    let expected_variants = [
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
    .collect::<BTreeSet<_>>();
    assert_eq!(variants, expected_variants);

    let expected_modes = quote_modes()
        .into_iter()
        .map(quote_mode_name)
        .collect::<BTreeSet<_>>();
    assert_eq!(modes, expected_modes);
}

fn collect_coverage(
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
                collect_coverage(item, variants, modes);
            }
        }
        Expr::Map(entries) => {
            for (key, value) in entries {
                collect_coverage(key, variants, modes);
                collect_coverage(value, variants, modes);
            }
        }
        Expr::Call { operator, args } => {
            collect_coverage(operator, variants, modes);
            for arg in args {
                collect_coverage(arg, variants, modes);
            }
        }
        Expr::Infix { left, right, .. } => {
            collect_coverage(left, variants, modes);
            collect_coverage(right, variants, modes);
        }
        Expr::Prefix { arg, .. } | Expr::Postfix { arg, .. } => {
            collect_coverage(arg, variants, modes);
        }
        Expr::Quote { mode, expr } => {
            modes.insert(quote_mode_name(*mode));
            collect_coverage(expr, variants, modes);
        }
        Expr::Annotated { expr, annotations } => {
            collect_coverage(expr, variants, modes);
            for (_, value) in annotations {
                collect_coverage(value, variants, modes);
            }
        }
        Expr::Extension { payload, .. } => {
            collect_coverage(payload, variants, modes);
        }
    }
}

#[test]
fn canonical_json_and_binary_ignore_map_and_set_order() {
    let mut cx = cx();
    let left = Expr::Map(vec![
        (
            Expr::Symbol(Symbol::new("b")),
            Expr::Set(vec![
                Expr::String("y".to_owned()),
                Expr::String("x".to_owned()),
            ]),
        ),
        (Expr::Symbol(Symbol::new("a")), Expr::Bool(true)),
    ]);
    let right = Expr::Map(vec![
        (Expr::Symbol(Symbol::new("a")), Expr::Bool(true)),
        (
            Expr::Symbol(Symbol::new("b")),
            Expr::Set(vec![
                Expr::String("x".to_owned()),
                Expr::String("y".to_owned()),
            ]),
        ),
    ]);

    for codec in [
        Symbol::qualified("codec", "json"),
        Symbol::qualified("codec", "binary"),
        Symbol::qualified("codec", "binary-base64"),
    ] {
        assert_eq!(
            encode_once(&mut cx, &codec, &left),
            encode_once(&mut cx, &codec, &right),
            "codec {} was not canonical for reordered map/set data",
            codec
        );
    }
}
