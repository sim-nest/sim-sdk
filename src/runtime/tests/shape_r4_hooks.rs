use std::sync::Arc;

use sim_kernel::{Args, DefaultFactory, EagerPolicy, Expr, Symbol, Value};

use crate::runtime::install_core_runtime;

fn cx() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn number_value(cx: &mut sim_kernel::Cx, text: &str) -> Value {
    cx.factory()
        .number_literal(Symbol::qualified("numbers", "f64"), text.to_owned())
        .unwrap()
}

#[test]
fn runtime_shape_hook_wraps_shape_and_runs_trace_hook() {
    let mut cx = cx();
    let any = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    let trace = cx
        .call_function(&Symbol::qualified("shape", "hook-trace"), Args::new(vec![]))
        .unwrap();
    let hooks = cx.factory().list(vec![trace]).unwrap();
    let hooked = cx
        .call_function(
            &Symbol::qualified("shape", "hook"),
            Args::new(vec![any, hooks]),
        )
        .unwrap();

    let matched = hooked
        .object()
        .as_shape()
        .unwrap()
        .check_expr(&mut cx, &Expr::Bool(true))
        .unwrap();

    assert!(matched.accepted);
    assert_eq!(
        matched
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message.starts_with("shape-hook:mark"))
            .count(),
        2
    );
}

#[test]
fn runtime_score_floor_hook_adjusts_hooked_shape_score() {
    let mut cx = cx();
    let any = cx.resolve_shape(&Symbol::qualified("core", "Any")).unwrap();
    let floor = number_value(&mut cx, "40");
    let hook = cx
        .call_function(
            &Symbol::qualified("shape", "hook-score-floor"),
            Args::new(vec![floor]),
        )
        .unwrap();
    let hooks = cx.factory().list(vec![hook]).unwrap();
    let hooked = cx
        .call_function(
            &Symbol::qualified("shape", "hook"),
            Args::new(vec![any, hooks]),
        )
        .unwrap();

    let matched = hooked
        .object()
        .as_shape()
        .unwrap()
        .check_expr(&mut cx, &Expr::Bool(true))
        .unwrap();

    assert!(matched.accepted);
    assert_eq!(matched.score.value(), 40);
}

#[test]
fn runtime_builtin_hook_values_have_stable_display() {
    let mut cx = cx();
    let floor = number_value(&mut cx, "7");
    let prefix = cx.factory().string("shape-hook:accept".to_owned()).unwrap();
    let hooks = vec![
        (
            Symbol::qualified("shape", "hook-trace"),
            Vec::new(),
            "#<shape-hook shape/trace-mark mark>",
        ),
        (
            Symbol::qualified("shape", "hook-score-floor"),
            vec![floor],
            "#<shape-hook shape/score-floor annotate>",
        ),
        (
            Symbol::qualified("shape", "hook-accept-on-no-diagnostics"),
            Vec::new(),
            "#<shape-hook shape/accept-on-no-diagnostics accept>",
        ),
        (
            Symbol::qualified("shape", "hook-discard-on-diagnostic-prefix"),
            vec![prefix],
            "#<shape-hook shape/discard-on-diagnostic-prefix discard>",
        ),
    ];

    for (symbol, args, expected) in hooks {
        let value = cx.call_function(&symbol, Args::new(args)).unwrap();
        assert_eq!(value.object().display(&mut cx).unwrap(), expected);
    }
}
