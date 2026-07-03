#![cfg(all(feature = "agent", feature = "server-net-http"))]
#![allow(deprecated)]

#[path = "server_agent_e2e_support.rs"]
mod support;

use std::sync::{Arc, Mutex};

use sim::kernel::{Args, Expr, Symbol};
use sim::lib_server::Connection;

use support::{
    CaptureReplySite, DriverPersonaSite, FixedReplySite, TransformSite, call_exprs, cx,
    flatten_text, keyword, make_connection, map_field, number_expr, quoted, register_connection,
    register_value,
};

#[test]
fn r21_swarm_proving_over_ring_and_mesh_records_multi_turn_results() {
    let mut cx = cx();

    register_connection(
        &mut cx,
        Symbol::qualified("test", "ring-a"),
        make_connection(Arc::new(TransformSite {
            prefix: "alpha:",
            hops: Arc::new(Mutex::new(Vec::new())),
            seen: Arc::new(Mutex::new(Vec::new())),
        })),
    );
    register_connection(
        &mut cx,
        Symbol::qualified("test", "ring-b"),
        make_connection(Arc::new(TransformSite {
            prefix: "beta:",
            hops: Arc::new(Mutex::new(Vec::new())),
            seen: Arc::new(Mutex::new(Vec::new())),
        })),
    );
    let ring = call_exprs(
        &mut cx,
        Symbol::qualified("topology", "ring"),
        vec![
            keyword("agents"),
            quoted(Expr::List(vec![
                Expr::Symbol(Symbol::qualified("test", "ring-a")),
                Expr::Symbol(Symbol::qualified("test", "ring-b")),
            ])),
            keyword("max-turns"),
            number_expr(2),
        ],
    );
    let ring_result = ring
        .object()
        .downcast_ref::<Connection>()
        .unwrap()
        .request(&mut cx, Expr::String("prove".to_owned()), None, Vec::new())
        .unwrap()
        .object()
        .as_expr(&mut cx)
        .unwrap();
    let ring_text = flatten_text(&ring_result);
    assert!(ring_text.contains("alpha"));
    assert!(ring_text.contains("beta"));

    register_connection(
        &mut cx,
        Symbol::qualified("test", "mesh-a"),
        make_connection(Arc::new(FixedReplySite {
            reply: Expr::String("weak".to_owned()),
        })),
    );
    register_connection(
        &mut cx,
        Symbol::qualified("test", "mesh-b"),
        make_connection(Arc::new(FixedReplySite {
            reply: Expr::String("perfect target".to_owned()),
        })),
    );
    let judge = cx
        .call_function(
            &Symbol::qualified("judge", "rubric"),
            Args::new(vec![
                cx.factory().symbol(Symbol::new(":reference")).unwrap(),
                cx.factory().string("perfect target".to_owned()).unwrap(),
            ]),
        )
        .unwrap();
    register_value(&mut cx, Symbol::qualified("test", "mesh-judge"), judge);
    let mesh = call_exprs(
        &mut cx,
        Symbol::qualified("topology", "mesh"),
        vec![
            keyword("agents"),
            quoted(Expr::List(vec![
                Expr::Symbol(Symbol::qualified("test", "mesh-a")),
                Expr::Symbol(Symbol::qualified("test", "mesh-b")),
            ])),
            keyword("judge"),
            Expr::Symbol(Symbol::qualified("test", "mesh-judge")),
            keyword("max-rounds"),
            number_expr(2),
        ],
    );
    let mesh_result = mesh
        .object()
        .downcast_ref::<Connection>()
        .unwrap()
        .request(&mut cx, Expr::String("prove".to_owned()), None, Vec::new())
        .unwrap()
        .object()
        .as_expr(&mut cx)
        .unwrap();
    assert!(flatten_text(&mesh_result).contains("perfect target"));

    let swarm = call_exprs(
        &mut cx,
        Symbol::qualified("swarm", "make"),
        vec![
            keyword("name"),
            Expr::Symbol(Symbol::new("proof-swarm")),
            keyword("max-turns"),
            number_expr(2),
        ],
    );
    let launched = cx
        .call_function(
            &Symbol::qualified("swarm", "launch"),
            Args::new(vec![
                swarm.clone(),
                cx.factory().string("prove".to_owned()).unwrap(),
            ]),
        )
        .unwrap()
        .object()
        .as_expr(&mut cx)
        .unwrap();
    assert!(flatten_text(&launched).contains("transcript"));
}

#[test]
fn r21_debate_judged_names_a_winner_and_carries_both_sides() {
    let mut cx = cx();
    register_connection(
        &mut cx,
        Symbol::qualified("test", "pro"),
        make_connection(Arc::new(FixedReplySite {
            reply: Expr::String("pro wins evidence".to_owned()),
        })),
    );
    register_connection(
        &mut cx,
        Symbol::qualified("test", "con"),
        make_connection(Arc::new(FixedReplySite {
            reply: Expr::String("con loses".to_owned()),
        })),
    );
    let judge = cx
        .call_function(
            &Symbol::qualified("judge", "rubric"),
            Args::new(vec![
                cx.factory().symbol(Symbol::new(":reference")).unwrap(),
                cx.factory().string("pro wins evidence".to_owned()).unwrap(),
            ]),
        )
        .unwrap();
    let debate = cx
        .call_function(
            &Symbol::qualified("topology", "debate"),
            Args::new(vec![
                cx.factory().symbol(Symbol::new(":pro")).unwrap(),
                cx.resolve_value(&Symbol::qualified("test", "pro")).unwrap(),
                cx.factory().symbol(Symbol::new(":con")).unwrap(),
                cx.resolve_value(&Symbol::qualified("test", "con")).unwrap(),
                cx.factory().symbol(Symbol::new(":judge")).unwrap(),
                judge,
                cx.factory().symbol(Symbol::new(":rounds")).unwrap(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "1".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    let reply = debate
        .object()
        .downcast_ref::<Connection>()
        .unwrap()
        .request(&mut cx, Expr::String("topic".to_owned()), None, Vec::new())
        .unwrap()
        .object()
        .as_expr(&mut cx)
        .unwrap();
    assert_eq!(
        map_field(&reply, "winner"),
        Some(Expr::String("pro".to_owned()))
    );
    let text = flatten_text(&reply);
    assert!(text.contains("pro wins evidence"));
    assert!(text.contains("con loses"));
}

#[test]
fn r21_self_driving_repl_uses_the_agent_driver_until_done() {
    let mut cx = cx();
    let seen = Arc::new(Mutex::new(Vec::new()));
    register_connection(
        &mut cx,
        Symbol::qualified("test", "repl-target"),
        make_connection(Arc::new(CaptureReplySite {
            seen: seen.clone(),
            reply: number_expr(42),
        })),
    );
    register_connection(
        &mut cx,
        Symbol::qualified("test", "driver-persona"),
        make_connection(Arc::new(DriverPersonaSite)),
    );
    let agent = call_exprs(
        &mut cx,
        Symbol::qualified("agent", "make"),
        vec![
            keyword("name"),
            Expr::Symbol(Symbol::new("dev")),
            keyword("persona"),
            Expr::Symbol(Symbol::qualified("test", "driver-persona")),
        ],
    );
    register_value(&mut cx, Symbol::new("dev"), agent);

    let direct = call_exprs(
        &mut cx,
        Symbol::qualified("server", "repl"),
        vec![
            keyword("connection"),
            Expr::Symbol(Symbol::qualified("test", "repl-target")),
            keyword("codec"),
            Expr::Symbol(Symbol::qualified("codec", "lisp")),
            keyword("driver"),
            Expr::List(vec![
                Expr::Symbol(Symbol::new("agent")),
                Expr::Symbol(Symbol::new("dev")),
            ]),
        ],
    );
    assert_eq!(direct.object().as_expr(&mut cx).unwrap(), Expr::Nil);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(matches!(
        &seen[0],
        Expr::Call { operator, args }
            if **operator == Expr::Symbol(Symbol::qualified("math", "add"))
                && args == &vec![number_expr(40), number_expr(2)]
    ));
}

#[test]
fn r21_speculate_verify_uses_fast_path_and_mismatch_behavior() {
    let mut cx = cx();
    register_connection(
        &mut cx,
        Symbol::qualified("test", "spec"),
        make_connection(Arc::new(FixedReplySite {
            reply: Expr::String("fast".to_owned()),
        })),
    );
    register_connection(
        &mut cx,
        Symbol::qualified("test", "verify-same"),
        make_connection(Arc::new(FixedReplySite {
            reply: Expr::String("fast".to_owned()),
        })),
    );
    register_connection(
        &mut cx,
        Symbol::qualified("test", "verify-other"),
        make_connection(Arc::new(FixedReplySite {
            reply: Expr::String("slow".to_owned()),
        })),
    );

    let agree = cx
        .call_function(
            &Symbol::qualified("topology", "speculate-verify"),
            Args::new(vec![
                cx.factory().symbol(Symbol::new(":speculator")).unwrap(),
                cx.resolve_value(&Symbol::qualified("test", "spec"))
                    .unwrap(),
                cx.factory().symbol(Symbol::new(":verifier")).unwrap(),
                cx.resolve_value(&Symbol::qualified("test", "verify-same"))
                    .unwrap(),
            ]),
        )
        .unwrap();
    let agree_expr = agree
        .object()
        .downcast_ref::<Connection>()
        .unwrap()
        .request(&mut cx, Expr::String("task".to_owned()), None, Vec::new())
        .unwrap()
        .object()
        .as_expr(&mut cx)
        .unwrap();
    assert_eq!(map_field(&agree_expr, "agreed"), Some(Expr::Bool(true)));
    assert_eq!(
        map_field(&agree_expr, "result"),
        Some(Expr::String("fast".to_owned()))
    );

    let mismatch = cx
        .call_function(
            &Symbol::qualified("topology", "speculate-verify"),
            Args::new(vec![
                cx.factory().symbol(Symbol::new(":speculator")).unwrap(),
                cx.resolve_value(&Symbol::qualified("test", "spec"))
                    .unwrap(),
                cx.factory().symbol(Symbol::new(":verifier")).unwrap(),
                cx.resolve_value(&Symbol::qualified("test", "verify-other"))
                    .unwrap(),
                cx.factory().symbol(Symbol::new(":on-mismatch")).unwrap(),
                cx.factory().symbol(Symbol::new("escalate")).unwrap(),
            ]),
        )
        .unwrap();
    let mismatch_expr = mismatch
        .object()
        .downcast_ref::<Connection>()
        .unwrap()
        .request(&mut cx, Expr::String("task".to_owned()), None, Vec::new())
        .unwrap()
        .object()
        .as_expr(&mut cx)
        .unwrap();
    assert_eq!(map_field(&mismatch_expr, "agreed"), Some(Expr::Bool(false)));
    assert_eq!(
        map_field(&mismatch_expr, "result"),
        Some(Expr::String("slow".to_owned()))
    );
}
