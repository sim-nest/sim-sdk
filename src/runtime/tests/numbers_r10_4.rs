#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-bigint",
    feature = "numbers-i64",
    feature = "numbers-rational"
))]
#[test]
fn mixed_bigint_rational_values_reduce_after_arithmetic() {
    use std::sync::Arc;

    use sim_kernel::{Args, DefaultFactory, EagerPolicy, Expr, Symbol};

    use crate::runtime::install_core_runtime;

    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let rational_class = cx
        .resolve_class(&Symbol::qualified("numbers", "Rational"))
        .unwrap();
    let numerator = cx
        .factory()
        .number_literal(
            Symbol::qualified("numbers", "bigint"),
            "1267650600228229401496703205376".to_owned(),
        )
        .unwrap();
    let denominator = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "i64"), "3".to_owned())
        .unwrap();
    let left = rational_class
        .object()
        .as_callable()
        .unwrap()
        .call(&mut cx, Args::new(vec![numerator, denominator]))
        .unwrap();
    let value = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                left,
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "rational"), "1/3".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(sim_kernel::NumberLiteral {
            domain: Symbol::qualified("numbers", "rational"),
            canonical: "1267650600228229401496703205377/3".to_owned(),
        })
    );
}

#[cfg(all(
    feature = "codec-lisp",
    feature = "numbers-bigint",
    feature = "numbers-i64",
    feature = "numbers-rational"
))]
#[test]
fn noncompact_rational_values_encode_as_read_constructs() {
    use std::sync::Arc;

    use sim_codec::{Input, decode_with_codec};
    use sim_codec_lisp::{LispCodecLib, encode_object_lisp};
    use sim_kernel::{
        Args, CapabilitySet, DefaultFactory, EagerPolicy, EncodeOptions, EncodePosition, Expr,
        ReadPolicy, Symbol, TrustLevel, read_construct_capability,
    };

    use crate::runtime::install_core_runtime;

    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let codec_id = cx.registry_mut().fresh_codec_id();
    cx.load_lib(&LispCodecLib::new(codec_id).unwrap()).unwrap();
    let rational_class = cx
        .resolve_class(&Symbol::qualified("numbers", "Rational"))
        .unwrap();
    let numerator = cx
        .factory()
        .number_literal(
            Symbol::qualified("numbers", "bigint"),
            "1267650600228229401496703205376".to_owned(),
        )
        .unwrap();
    let denominator = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "i64"), "3".to_owned())
        .unwrap();
    let value = rational_class
        .object()
        .as_callable()
        .unwrap()
        .call(&mut cx, Args::new(vec![numerator, denominator]))
        .unwrap();

    cx.grant(read_construct_capability());
    let encoded = encode_object_lisp(
        &mut sim_kernel::WriteCx {
            cx: &mut cx,
            codec: codec_id,
            options: EncodeOptions {
                position: EncodePosition::Quote,
                ..Default::default()
            },
        },
        value,
    )
    .unwrap();
    assert_eq!(
        encoded,
        "#(numbers/Rational 1267650600228229401496703205376 3)"
    );

    let decoded = decode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text(encoded),
        ReadPolicy {
            trust: TrustLevel::TrustedSource,
            capabilities: CapabilitySet::new().grant(read_construct_capability()),
        },
    )
    .unwrap();
    let Expr::Extension { tag, .. } = decoded else {
        panic!("expected decoded noncompact rational to stay structured");
    };
    assert_eq!(tag, Symbol::qualified("numbers", "Rational"));
}
