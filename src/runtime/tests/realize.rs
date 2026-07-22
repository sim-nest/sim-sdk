use std::sync::Arc;
use std::time::Duration;

use sim_kernel::{Error, EvalRequest, Expr, NumberLiteral, Symbol, read_construct_capability};

#[cfg(feature = "server")]
use crate::install_server_lib;

use super::support::eval_cx;

#[cfg(feature = "logic-core")]
fn logic_db_write_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("logic.db.write")
}

#[test]
fn realize_evaluates_through_local_fabric() {
    let mut cx = eval_cx();
    let value = cx
        .call_exprs(
            cx.resolve_function(&Symbol::new("realize")).unwrap(),
            vec![
                Expr::Call {
                    operator: Box::new(Expr::Symbol(Symbol::qualified("math", "add"))),
                    args: vec![
                        Expr::Number(NumberLiteral {
                            domain: Symbol::qualified("numbers", "f64"),
                            canonical: "1".to_owned(),
                        }),
                        Expr::Number(NumberLiteral {
                            domain: Symbol::qualified("numbers", "f64"),
                            canonical: "2".to_owned(),
                        }),
                    ],
                },
                Expr::Symbol(Symbol::new(":result")),
                Expr::Symbol(Symbol::qualified("core", "Number")),
            ],
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "3".to_owned()
        })
    );
}

#[test]
fn realize_enforces_required_capabilities() {
    let mut cx = eval_cx();
    let denied = cx.call_exprs(
        cx.resolve_function(&Symbol::new("realize")).unwrap(),
        vec![
            Expr::Nil,
            Expr::Symbol(Symbol::new(":requires")),
            Expr::Quote {
                mode: sim_kernel::QuoteMode::Quote,
                expr: Box::new(Expr::List(vec![Expr::Symbol(Symbol::new(
                    "read-construct",
                ))])),
            },
        ],
    );
    assert!(
        matches!(denied, Err(Error::CapabilityDenied { capability }) if capability == read_construct_capability())
    );
    cx.grant(read_construct_capability());
    let value = cx
        .call_exprs(
            cx.resolve_function(&Symbol::new("realize")).unwrap(),
            vec![
                Expr::Nil,
                Expr::Symbol(Symbol::new(":requires")),
                Expr::Quote {
                    mode: sim_kernel::QuoteMode::Quote,
                    expr: Box::new(Expr::List(vec![Expr::Symbol(Symbol::new(
                        "read-construct",
                    ))])),
                },
            ],
        )
        .unwrap();
    assert!(matches!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Nil
    ));
}

#[cfg(feature = "server")]
#[test]
fn realize_evaluates_through_server_connection_fabric() {
    let mut cx = eval_cx();
    install_server_lib(&mut cx).unwrap();
    let json = crate::codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&json).unwrap();
    let connection = cx
        .call_exprs(
            cx.resolve_function(&Symbol::qualified("server", "connect"))
                .unwrap(),
            vec![Expr::Quote {
                mode: sim_kernel::QuoteMode::Quote,
                expr: Box::new(Expr::Symbol(Symbol::new("local"))),
            }],
        )
        .unwrap();
    cx.registry_mut()
        .register_value(Symbol::qualified("test", "conn"), connection)
        .unwrap();
    let value = cx
        .call_exprs(
            cx.resolve_function(&Symbol::new("realize")).unwrap(),
            vec![
                Expr::Nil,
                Expr::Symbol(Symbol::new(":fabric")),
                Expr::Symbol(Symbol::qualified("test", "conn")),
            ],
        )
        .unwrap();
    assert!(matches!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::Nil
    ));
}

#[test]
fn realize_runtime_classes_and_local_fabric_are_registered() {
    let mut cx = eval_cx();
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "EvalRequest"))
            .is_some()
    );
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "EvalReply"))
            .is_some()
    );
    let request = EvalRequest {
        expr: Expr::Nil,
        mode: sim_kernel::EvalMode::Eval,
        result_shape: None,
        answer_limit: None,
        stream_buffer: None,
        stream: false,
        required_capabilities: Vec::new(),
        deadline: Some(Duration::from_secs(2)),
        consistency: sim_kernel::Consistency::LocalFirst,
        trace: false,
    };
    let request_value = cx.factory().opaque(Arc::new(request)).unwrap();
    assert_eq!(
        request_value
            .object()
            .class(&mut cx)
            .unwrap()
            .object()
            .as_expr(&mut cx)
            .unwrap(),
        Expr::Symbol(Symbol::qualified("core", "EvalRequest"))
    );
    let fabric_value = cx
        .resolve_value(&Symbol::qualified("core", "local-fabric"))
        .unwrap();
    let fabric = fabric_value.object().as_eval_fabric().unwrap();
    let reply = fabric
        .realize(
            &mut cx,
            EvalRequest {
                expr: Expr::Nil,
                mode: sim_kernel::EvalMode::Eval,
                result_shape: None,
                answer_limit: None,
                stream_buffer: None,
                stream: false,
                required_capabilities: Vec::new(),
                deadline: None,
                consistency: sim_kernel::Consistency::LocalFirst,
                trace: true,
            },
        )
        .unwrap();
    let reply_value = cx.factory().opaque(Arc::new(reply)).unwrap();
    assert_eq!(
        reply_value
            .object()
            .class(&mut cx)
            .unwrap()
            .object()
            .as_expr(&mut cx)
            .unwrap(),
        Expr::Symbol(Symbol::qualified("core", "EvalReply"))
    );
}

#[cfg(feature = "logic-core")]
#[test]
fn realize_logic_mode_queries_the_logic_db() {
    let mut cx = eval_cx();
    cx.grant(logic_db_write_capability());
    let assert_fn = cx
        .resolve_function(&Symbol::qualified("logic", "assert!"))
        .unwrap();
    cx.call_exprs(
        assert_fn,
        vec![Expr::Quote {
            mode: sim_kernel::QuoteMode::Quote,
            expr: Box::new(Expr::List(vec![
                Expr::Symbol(Symbol::new("fact")),
                Expr::List(vec![
                    Expr::Symbol(Symbol::new("parent")),
                    Expr::Symbol(Symbol::new("alice")),
                    Expr::Symbol(Symbol::new("bob")),
                ]),
            ])),
        }],
    )
    .unwrap();
    let value = cx
        .call_exprs(
            cx.resolve_function(&Symbol::new("realize")).unwrap(),
            vec![
                Expr::List(vec![
                    Expr::Symbol(Symbol::new("parent")),
                    Expr::Symbol(Symbol::new("alice")),
                    Expr::Local(Symbol::new("x")),
                ]),
                Expr::Symbol(Symbol::new(":mode")),
                Expr::Quote {
                    mode: sim_kernel::QuoteMode::Quote,
                    expr: Box::new(Expr::Symbol(Symbol::new("logic"))),
                },
            ],
        )
        .unwrap();
    assert!(value.object().truth(&mut cx).unwrap());
    let expr = value.object().as_expr(&mut cx).unwrap();
    assert!(matches!(expr, Expr::Map(_)));
}
