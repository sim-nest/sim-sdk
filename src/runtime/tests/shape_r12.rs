use std::sync::Arc;

use sim_kernel::{
    Args, DefaultFactory, EagerPolicy, Expr, NumberLiteral, Symbol, macro_expand_eval_capability,
};

use crate::runtime::install_core_runtime;

#[test]
fn shape_runtime_helpers_use_kernel_shape_protocol() {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let any = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    assert!(any.object().as_shape().is_some());
    assert!(any.object().as_callable().is_some());

    let checked = cx
        .call_value(
            any,
            Args::new(vec![cx.factory().string("ok".to_owned()).unwrap()]),
        )
        .unwrap();
    let accepted = cx
        .call_function(
            &Symbol::qualified("shape", "accepted?"),
            Args::new(vec![checked]),
        )
        .unwrap();
    assert_eq!(
        accepted.object().as_expr(&mut cx).unwrap(),
        Expr::Bool(true)
    );

    let number = cx
        .resolve_shape(&Symbol::qualified("core", "Number"))
        .unwrap();
    let checked = cx
        .call_function(
            &Symbol::qualified("shape", "check"),
            Args::new(vec![
                number,
                cx.factory().string("not-number".to_owned()).unwrap(),
            ]),
        )
        .unwrap();
    let accepted = cx
        .call_function(
            &Symbol::qualified("shape", "accepted?"),
            Args::new(vec![checked]),
        )
        .unwrap();
    assert_eq!(
        accepted.object().as_expr(&mut cx).unwrap(),
        Expr::Bool(false)
    );
}

#[test]
fn shape_subshape_helpers_cover_any_exact_one_of_and_class_ancestry() {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let any = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    let number = cx
        .resolve_shape(&Symbol::qualified("core", "Number"))
        .unwrap();
    let number_kind = cx
        .call_class(
            &Symbol::qualified("core", "ExprKindShape"),
            Args::new(vec![cx.factory().symbol(Symbol::new("number")).unwrap()]),
        )
        .unwrap();
    let exact_one = exact_number_shape(&mut cx, "1");
    let exact_two = exact_number_shape(&mut cx, "2");
    let exacts = cx.factory().list(vec![exact_one, exact_two]).unwrap();
    let one_of_numbers = cx
        .call_class(
            &Symbol::qualified("core", "OneOfShape"),
            Args::new(vec![exacts]),
        )
        .unwrap();
    let shape_class = cx
        .resolve_class(&Symbol::qualified("core", "Shape"))
        .unwrap();
    let any_shape_class = cx
        .resolve_class(&Symbol::qualified("core", "AnyShape"))
        .unwrap();
    let class_parent = cx
        .call_class(
            &Symbol::qualified("core", "ClassShape"),
            Args::new(vec![shape_class]),
        )
        .unwrap();
    let class_child = cx
        .call_class(
            &Symbol::qualified("core", "ClassShape"),
            Args::new(vec![any_shape_class.clone()]),
        )
        .unwrap();

    for (child, parent) in [
        (number, any),
        (one_of_numbers, number_kind),
        (class_child, class_parent),
    ] {
        let result = cx
            .call_function(
                &Symbol::qualified("shape", "subshape?"),
                Args::new(vec![child, parent]),
            )
            .unwrap();
        assert_eq!(result.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
    }

    let class_parent_shape = cx
        .call_class(
            &Symbol::qualified("core", "ClassShape"),
            Args::new(vec![
                cx.resolve_class(&Symbol::qualified("core", "Shape"))
                    .unwrap(),
            ]),
        )
        .unwrap();
    let matched = class_parent_shape
        .object()
        .as_shape()
        .unwrap()
        .check_value(&mut cx, any_shape_class)
        .unwrap();
    assert!(matched.accepted);
}

#[test]
fn shape_constructor_values_encode_without_opaque_display_text() {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let shape = cx
        .call_class(
            &Symbol::qualified("core", "ExprKindShape"),
            Args::new(vec![cx.factory().symbol(Symbol::new("number")).unwrap()]),
        )
        .unwrap();
    let expr = shape.object().as_expr(&mut cx).unwrap();

    assert_eq!(
        expr,
        Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::qualified("core", "ExprKindShape"))),
            args: vec![Expr::Symbol(Symbol::new("number"))],
        }
    );
    assert!(shape.object().as_shape().is_some());
    assert!(shape.object().as_callable().is_some());
}

#[cfg(feature = "codec-lisp")]
#[test]
fn shape_read_construct_decodes_to_callable_shape_value() {
    use sim_codec::{Input, decode_with_codec};
    use sim_codec_lisp::LispCodecLib;
    use sim_kernel::{CapabilitySet, ReadPolicy, TrustLevel, read_construct_capability};

    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx.grant(macro_expand_eval_capability());
    cx.grant(read_construct_capability());
    let codec_id = cx.registry_mut().fresh_codec_id();
    cx.load_lib(&LispCodecLib::new(codec_id).unwrap()).unwrap();
    let decoded = decode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text("#(core/ExprKindShape number)".to_owned()),
        ReadPolicy {
            trust: TrustLevel::TrustedSource,
            capabilities: CapabilitySet::new().grant(read_construct_capability()),
        },
    )
    .unwrap();
    let value = cx.eval_expr(decoded).unwrap();

    assert!(value.object().as_shape().is_some());
    assert!(value.object().as_callable().is_some());
}

fn exact_number_shape(cx: &mut sim_kernel::Cx, canonical: &str) -> sim_kernel::Value {
    cx.call_class(
        &Symbol::qualified("core", "ExactExprShape"),
        Args::new(vec![
            cx.factory()
                .expr(Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "f64"),
                    canonical: canonical.to_owned(),
                }))
                .unwrap(),
        ]),
    )
    .unwrap()
}
