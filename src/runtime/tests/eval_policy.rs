use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use sim_kernel::{
    Args, Demand, EagerPolicy, Expr, HybridPolicy, LazyPolicy, NeedPolicy, QuoteMode,
    StrictByShapePolicy, Symbol,
};

use super::support::{
    TickCallable, call_expr, eval_cx_with_policy, force_first_arg_twice_impl, ignore_arg_impl,
    one_arg_function, return_first_arg_impl, table_value, two_arg_function,
};

#[test]
fn demand_never_does_not_evaluate_exploding_argument() {
    let mut cx = eval_cx_with_policy(Arc::new(HybridPolicy));
    let function = Symbol::qualified("test", "ignore");
    one_arg_function(&mut cx, function.clone(), Demand::Never, ignore_arg_impl);
    let result = cx
        .eval_expr(call_expr(
            function,
            vec![Expr::Call {
                operator: Box::new(Expr::Bool(true)),
                args: Vec::new(),
            }],
        ))
        .unwrap();
    assert_eq!(result.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
}

#[test]
fn demand_expr_receives_original_expression() {
    let mut cx = eval_cx_with_policy(Arc::new(HybridPolicy));
    let function = Symbol::qualified("test", "expr");
    one_arg_function(
        &mut cx,
        function.clone(),
        Demand::Expr,
        return_first_arg_impl,
    );
    let original = Expr::Call {
        operator: Box::new(Expr::Symbol(Symbol::qualified("maybe", "later"))),
        args: vec![Expr::Bool(true)],
    };
    let result = cx
        .eval_expr(call_expr(function, vec![original.clone()]))
        .unwrap();
    assert_eq!(result.object().as_expr(&mut cx).unwrap(), original);
}

#[test]
fn lazy_policy_delays_unused_args_and_recomputes_forces() {
    let mut cx = eval_cx_with_policy(Arc::new(LazyPolicy));
    let ignore_symbol = Symbol::qualified("test", "ignore");
    one_arg_function(
        &mut cx,
        ignore_symbol.clone(),
        Demand::Never,
        ignore_arg_impl,
    );
    let result = cx
        .eval_expr(call_expr(
            ignore_symbol,
            vec![Expr::Call {
                operator: Box::new(Expr::Bool(true)),
                args: Vec::new(),
            }],
        ))
        .unwrap();
    assert_eq!(result.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));

    let counter = Arc::new(AtomicUsize::new(0));
    let tick = cx
        .factory()
        .opaque(Arc::new(TickCallable {
            counter: counter.clone(),
        }))
        .unwrap();
    let tick_symbol = Symbol::qualified("test", "tick");
    cx.env_mut().define(tick_symbol.clone(), tick);
    let force_symbol = Symbol::qualified("test", "force-twice");
    one_arg_function(
        &mut cx,
        force_symbol.clone(),
        Demand::Never,
        force_first_arg_twice_impl,
    );
    let result = cx
        .eval_expr(call_expr(
            force_symbol,
            vec![call_expr(tick_symbol, Vec::new())],
        ))
        .unwrap();
    assert_eq!(result.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[test]
fn lazy_by_need_memoizes_argument_evaluation_once() {
    let mut cx = eval_cx_with_policy(Arc::new(NeedPolicy));
    let counter = Arc::new(AtomicUsize::new(0));
    let tick = cx
        .factory()
        .opaque(Arc::new(TickCallable {
            counter: counter.clone(),
        }))
        .unwrap();
    let tick_symbol = Symbol::qualified("test", "tick");
    cx.env_mut().define(tick_symbol.clone(), tick);
    let memo_symbol = Symbol::qualified("test", "memo");
    one_arg_function(
        &mut cx,
        memo_symbol.clone(),
        Demand::Never,
        force_first_arg_twice_impl,
    );
    let result = cx
        .eval_expr(call_expr(
            memo_symbol,
            vec![call_expr(tick_symbol, Vec::new())],
        ))
        .unwrap();
    assert_eq!(result.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn strict_by_shape_evaluates_only_required_positions() {
    let mut cx = eval_cx_with_policy(Arc::new(StrictByShapePolicy));
    let counter = Arc::new(AtomicUsize::new(0));
    let tick = cx
        .factory()
        .opaque(Arc::new(TickCallable {
            counter: counter.clone(),
        }))
        .unwrap();
    let tick_symbol = Symbol::qualified("test", "tick");
    cx.env_mut().define(tick_symbol.clone(), tick);
    let first_symbol = Symbol::qualified("test", "first");
    two_arg_function(
        &mut cx,
        first_symbol.clone(),
        [Demand::Value, Demand::Never],
        return_first_arg_impl,
    );
    let result = cx
        .eval_expr(call_expr(
            first_symbol,
            vec![
                call_expr(tick_symbol, Vec::new()),
                Expr::Call {
                    operator: Box::new(Expr::Bool(true)),
                    args: Vec::new(),
                },
            ],
        ))
        .unwrap();
    assert_eq!(result.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn eager_lazy_and_hybrid_can_run_the_same_simple_program() {
    for eval_policy in [
        Arc::new(EagerPolicy) as sim_kernel::EvalPolicyRef,
        Arc::new(LazyPolicy) as sim_kernel::EvalPolicyRef,
        Arc::new(NeedPolicy) as sim_kernel::EvalPolicyRef,
        Arc::new(StrictByShapePolicy) as sim_kernel::EvalPolicyRef,
        Arc::new(HybridPolicy) as sim_kernel::EvalPolicyRef,
    ] {
        let mut cx = eval_cx_with_policy(eval_policy);
        let identity = Symbol::qualified("test", "identity");
        one_arg_function(
            &mut cx,
            identity.clone(),
            Demand::Value,
            return_first_arg_impl,
        );
        let result = cx
            .eval_expr(call_expr(
                identity.clone(),
                vec![call_expr(identity, vec![Expr::Bool(true)])],
            ))
            .unwrap();
        assert_eq!(result.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
    }
}

#[test]
fn with_eval_policy_switches_for_body_and_restores_policy() {
    let mut cx = eval_cx_with_policy(Arc::new(EagerPolicy));
    let ignore_symbol = Symbol::qualified("test", "ignore");
    one_arg_function(
        &mut cx,
        ignore_symbol.clone(),
        Demand::Never,
        ignore_arg_impl,
    );
    let result = cx
        .eval_expr(call_expr(
            Symbol::qualified("core", "with-eval-policy"),
            vec![
                Expr::Quote {
                    mode: QuoteMode::Quote,
                    expr: Box::new(Expr::Symbol(Symbol::new("lazy"))),
                },
                call_expr(
                    ignore_symbol,
                    vec![Expr::Call {
                        operator: Box::new(Expr::Bool(true)),
                        args: Vec::new(),
                    }],
                ),
            ],
        ))
        .unwrap();
    assert_eq!(result.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
    assert_eq!(cx.eval_policy_name(), "eager");
}

#[test]
fn eval_policies_reports_typed_identity_for_active_policy() {
    let mut cx = eval_cx_with_policy(Arc::new(NeedPolicy));
    let policies = cx
        .call_function(
            &Symbol::qualified("core", "eval-policies"),
            Args::new(Vec::new()),
        )
        .unwrap();
    let policies_expr = policies.object().as_expr(&mut cx).unwrap();
    let Expr::List(entries) = policies_expr else {
        panic!("expected eval policy browse list");
    };
    for policy in ["eager", "lazy", "lazy-by-need", "strict-by-shape", "hybrid"] {
        assert!(entries.iter().any(|entry| {
            table_value(entry, &Symbol::new("id"))
                == Some(&Expr::Symbol(Symbol::qualified("core", policy)))
        }));
    }
    let current = entries
        .iter()
        .find(|entry| table_value(entry, &Symbol::new("current")) == Some(&Expr::Bool(true)))
        .unwrap();
    assert_eq!(
        table_value(current, &Symbol::new("id")),
        Some(&Expr::Symbol(Symbol::qualified("core", "lazy-by-need")))
    );
}
