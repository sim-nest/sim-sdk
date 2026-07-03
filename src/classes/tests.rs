use std::sync::Arc;

use sim_kernel::{
    Args, Class, DefaultFactory, EagerPolicy, Expr, NumberLiteral, ObjectCompat, ObjectEncode,
    PreparedArgs, read_construct_capability,
};
use sim_shape::{
    Bindings, CaptureShape, ExprKind, ExprKindShape, FieldShape, FieldSpec, ListShape, ObjectExpr,
};

use crate::{
    classes::{ClassInstance, NativeClass, NativeClassLib},
    functions::{FunctionCase, FunctionObject},
    runtime::install_core_runtime,
};

fn cx() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn point_impl(
    cx: &mut sim_kernel::Cx,
    _prepared: &PreparedArgs,
    bindings: Bindings,
) -> sim_kernel::Result<sim_kernel::Value> {
    let mut constructor_args = Vec::new();
    let mut fields = Vec::new();
    for (name, expr) in bindings.exprs() {
        constructor_args.push(expr.clone());
        fields.push((name.clone(), cx.factory().expr(expr.clone())?));
    }
    cx.factory().opaque(Arc::new(ClassInstance::new(
        sim_kernel::Symbol::new("Point"),
        constructor_args,
        fields,
    )))
}

fn point_constructor(cx: &mut sim_kernel::Cx) -> FunctionObject {
    FunctionObject::new(
        cx.registry_mut().fresh_function_id(),
        sim_kernel::Symbol::new("Point"),
        vec![FunctionCase {
            id: cx.registry_mut().fresh_case_id(),
            name: sim_kernel::Symbol::new("point-new"),
            args: Arc::new(ListShape::new(vec![
                Arc::new(CaptureShape::new(
                    sim_kernel::Symbol::new("x"),
                    Arc::new(ExprKindShape::new(ExprKind::Number)),
                )),
                Arc::new(CaptureShape::new(
                    sim_kernel::Symbol::new("y"),
                    Arc::new(ExprKindShape::new(ExprKind::Number)),
                )),
            ])),
            result: Some(point_instance_shape()),
            demand: Vec::new(),
            priority: 10,
            implementation: point_impl,
        }],
    )
}

fn point_instance_shape() -> Arc<dyn sim_shape::Shape> {
    Arc::new(FieldShape::new(
        sim_kernel::Symbol::new("Point"),
        vec![
            FieldSpec::required(
                sim_kernel::Symbol::new("x"),
                Arc::new(ExprKindShape::new(ExprKind::Number)),
            ),
            FieldSpec::required(
                sim_kernel::Symbol::new("y"),
                Arc::new(ExprKindShape::new(ExprKind::Number)),
            ),
        ],
    ))
}

#[test]
fn class_is_callable_through_constructor() {
    let mut cx = cx();
    let class = NativeClass::new(
        cx.registry_mut().fresh_class_id(),
        sim_kernel::Symbol::new("Point"),
        point_constructor(&mut cx),
        Some(point_instance_shape()),
        vec![sim_kernel::Symbol::new("x"), sim_kernel::Symbol::new("y")],
    );
    let args = Args::new(vec![
        cx.factory()
            .number_literal(
                sim_kernel::Symbol::qualified("numbers", "f64"),
                "3".to_owned(),
            )
            .unwrap(),
        cx.factory()
            .number_literal(
                sim_kernel::Symbol::qualified("numbers", "f64"),
                "4".to_owned(),
            )
            .unwrap(),
    ]);

    let value = sim_kernel::Callable::call(&class, &mut cx, args).unwrap();
    let expr = value.object().as_expr(&mut cx).unwrap();
    assert!(ObjectExpr::parse(&expr).is_some());
    assert_eq!(
        value
            .object()
            .class(&mut cx)
            .unwrap()
            .object()
            .as_expr(&mut cx)
            .unwrap(),
        Expr::Symbol(sim_kernel::Symbol::new("Point"))
    );
}

#[test]
fn class_browse_surface_exposes_symbol_and_members() {
    let mut cx = cx();
    let class = NativeClass::new(
        cx.registry_mut().fresh_class_id(),
        sim_kernel::Symbol::new("Point"),
        point_constructor(&mut cx),
        Some(point_instance_shape()),
        vec![sim_kernel::Symbol::new("x"), sim_kernel::Symbol::new("y")],
    );

    let table = class.as_table(&mut cx).unwrap();
    let expr = table.object().as_expr(&mut cx).unwrap();
    assert!(matches!(expr, Expr::Map(_)));
    assert_eq!(class.member_names().count(), 2);
    assert_eq!(class.symbol(), sim_kernel::Symbol::new("Point"));
}

#[test]
fn member_functions_are_callable() {
    let mut cx = cx();
    let class = NativeClass::new(
        cx.registry_mut().fresh_class_id(),
        sim_kernel::Symbol::new("Point"),
        point_constructor(&mut cx),
        Some(point_instance_shape()),
        vec![sim_kernel::Symbol::new("x"), sim_kernel::Symbol::new("y")],
    );
    let args = Args::new(vec![
        cx.factory()
            .number_literal(
                sim_kernel::Symbol::qualified("numbers", "f64"),
                "3".to_owned(),
            )
            .unwrap(),
        cx.factory()
            .number_literal(
                sim_kernel::Symbol::qualified("numbers", "f64"),
                "4".to_owned(),
            )
            .unwrap(),
    ]);
    let point = sim_kernel::Callable::call(&class, &mut cx, args).unwrap();

    let x_member = class
        .member_function(&sim_kernel::Symbol::new("x"))
        .unwrap();
    let value = sim_kernel::Callable::call(x_member, &mut cx, Args::new(vec![point])).unwrap();
    let expr = value.object().as_expr(&mut cx).unwrap();
    assert!(matches!(expr, Expr::Number(_)));
}

#[test]
fn instance_and_constructor_shapes_are_shape_objects() {
    let mut cx = cx();
    let class = NativeClass::new(
        cx.registry_mut().fresh_class_id(),
        sim_kernel::Symbol::new("Point"),
        point_constructor(&mut cx),
        Some(point_instance_shape()),
        vec![sim_kernel::Symbol::new("x"), sim_kernel::Symbol::new("y")],
    );

    let constructor_shape = class.constructor_shape(&mut cx).unwrap();
    let constructor_expr = constructor_shape.object().as_expr(&mut cx).unwrap();
    assert!(matches!(constructor_expr, Expr::Symbol(_)));

    let instance_shape = class.instance_shape(&mut cx).unwrap();
    let instance_expr = instance_shape.object().as_expr(&mut cx).unwrap();
    assert!(matches!(instance_expr, Expr::Symbol(_)));
}

#[test]
fn native_class_lib_registers_class_member_and_shape_exports() {
    let mut cx = cx();
    let class = NativeClass::new(
        cx.registry_mut().fresh_class_id(),
        sim_kernel::Symbol::new("Point"),
        point_constructor(&mut cx),
        Some(point_instance_shape()),
        vec![sim_kernel::Symbol::new("x"), sim_kernel::Symbol::new("y")],
    );
    let lib = NativeClassLib::from_class(
        sim_kernel::Symbol::qualified("test", "geometry"),
        &class,
        "0.1.0",
    );

    cx.load_lib(&lib).unwrap();
    assert!(
        cx.registry()
            .class_by_symbol(&sim_kernel::Symbol::new("Point"))
            .is_some()
    );
    assert!(
        cx.registry()
            .function_by_symbol(&sim_kernel::Symbol::qualified("Point", "x"))
            .is_some()
    );
    assert!(
        cx.registry()
            .shape_by_symbol(&sim_kernel::Symbol::qualified("Point", "instance-shape"))
            .is_some()
    );
}

#[test]
fn class_instances_provide_constructor_encoding() {
    let mut cx = cx();
    let instance = ClassInstance::new(
        sim_kernel::Symbol::new("Point"),
        vec![
            Expr::Number(NumberLiteral {
                domain: sim_kernel::Symbol::qualified("numbers", "f64"),
                canonical: "1".to_owned(),
            }),
            Expr::Number(NumberLiteral {
                domain: sim_kernel::Symbol::qualified("numbers", "f64"),
                canonical: "2".to_owned(),
            }),
        ],
        Vec::new(),
    );

    let encoding = instance.object_encoding(&mut cx).unwrap();
    assert!(matches!(
        encoding,
        sim_kernel::ObjectEncoding::Constructor { class, args }
            if class == sim_kernel::Symbol::new("Point") && args.len() == 2
    ));
}

#[test]
fn read_construct_uses_registered_class_constructor() {
    let mut cx = cx();
    let class = NativeClass::new(
        cx.registry_mut().fresh_class_id(),
        sim_kernel::Symbol::new("Point"),
        point_constructor(&mut cx),
        Some(point_instance_shape()),
        vec![sim_kernel::Symbol::new("x"), sim_kernel::Symbol::new("y")],
    );
    let lib = NativeClassLib::from_class(
        sim_kernel::Symbol::qualified("test", "geometry"),
        &class,
        "0.1.0",
    );
    cx.load_lib(&lib).unwrap();

    let denied = cx.read_construct(
        &sim_kernel::Symbol::new("Point"),
        vec![
            cx.factory()
                .number_literal(
                    sim_kernel::Symbol::qualified("numbers", "f64"),
                    "1".to_owned(),
                )
                .unwrap(),
            cx.factory()
                .number_literal(
                    sim_kernel::Symbol::qualified("numbers", "f64"),
                    "2".to_owned(),
                )
                .unwrap(),
        ],
    );
    assert!(matches!(
        denied,
        Err(sim_kernel::Error::CapabilityDenied { capability })
            if capability == read_construct_capability()
    ));

    cx.grant(read_construct_capability());
    let value = cx
        .read_construct(
            &sim_kernel::Symbol::new("Point"),
            vec![
                cx.factory()
                    .number_literal(
                        sim_kernel::Symbol::qualified("numbers", "f64"),
                        "1".to_owned(),
                    )
                    .unwrap(),
                cx.factory()
                    .number_literal(
                        sim_kernel::Symbol::qualified("numbers", "f64"),
                        "2".to_owned(),
                    )
                    .unwrap(),
            ],
        )
        .unwrap();
    assert!(matches!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Extension { .. }
    ));
}

#[test]
fn point_expr_does_not_match_rectangle_shape() {
    let mut cx = cx();
    let point = ObjectExpr {
        class: sim_kernel::Symbol::new("Point"),
        fields: vec![
            (
                sim_kernel::Symbol::new("x"),
                Expr::Number(NumberLiteral {
                    domain: sim_kernel::Symbol::qualified("numbers", "f64"),
                    canonical: "1".to_owned(),
                }),
            ),
            (
                sim_kernel::Symbol::new("y"),
                Expr::Number(NumberLiteral {
                    domain: sim_kernel::Symbol::qualified("numbers", "f64"),
                    canonical: "2".to_owned(),
                }),
            ),
        ],
    }
    .to_expr();
    let shape = sim_shape::ClassShape::new(sim_kernel::Symbol::new("Rectangle"));
    let matched = sim_shape::Shape::check_expr(&shape, &mut cx, &point).unwrap();
    assert!(!matched.accepted);
}

#[test]
fn point_expr_missing_x_field_rejects() {
    let mut cx = cx();
    let shape = point_instance_shape();
    let point = ObjectExpr {
        class: sim_kernel::Symbol::new("Point"),
        fields: vec![(
            sim_kernel::Symbol::new("y"),
            Expr::Number(NumberLiteral {
                domain: sim_kernel::Symbol::qualified("numbers", "f64"),
                canonical: "2".to_owned(),
            }),
        )],
    }
    .to_expr();
    let matched = shape.check_expr(&mut cx, &point).unwrap();
    assert!(!matched.accepted);
}
