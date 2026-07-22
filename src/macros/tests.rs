use std::sync::Arc;

use sim_kernel::{
    DefaultFactory, EagerPolicy, Expr, NoopEvalPolicy, QuoteMode, Symbol, macro_expand_capability,
    macro_expand_compile_capability, macro_expand_eval_capability, macro_expand_read_capability,
};
use sim_shape::{AnyShape, CaptureShape, EffectfulShape};

use crate::{
    macros::{
        MacroCx, MacroExpansionLimits, NativeExprMacro, RegistryMacroExpander, SourceTemplateMacro,
        expand_expr, list_macro_shape, list_macro_shape_with_rest, macro_value,
        positional_macro_shape, register_macro, register_macro_with_parser_trust,
    },
    runtime::install_core_runtime,
};

fn eager_cx() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx.grant(macro_expand_capability());
    cx.grant(macro_expand_compile_capability());
    cx.grant(macro_expand_eval_capability());
    cx.grant(macro_expand_read_capability());
    cx
}

fn ungranted_eager_cx() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn noop_cx() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx.grant(macro_expand_capability());
    cx
}

fn when_macro() -> NativeExprMacro {
    NativeExprMacro::new(
        Symbol::new("when"),
        list_macro_shape_with_rest(
            Symbol::new("when"),
            vec![Arc::new(CaptureShape::new(
                Symbol::new("condition"),
                Arc::new(AnyShape),
            ))],
            Arc::new(CaptureShape::new(Symbol::new("body"), Arc::new(AnyShape))),
        ),
        |_cx, _input, bindings| {
            let condition = bindings
                .exprs()
                .iter()
                .find_map(|(name, expr)| (name == &Symbol::new("condition")).then_some(expr))
                .cloned()
                .ok_or_else(|| sim_kernel::Error::Eval("missing condition".to_owned()))?;
            let body = bindings
                .exprs()
                .iter()
                .filter_map(|(name, expr)| (name == &Symbol::new("body")).then_some(expr))
                .cloned()
                .collect::<Vec<_>>();
            let mut do_items = vec![Expr::Symbol(Symbol::new("do"))];
            do_items.extend(body);
            Ok(Expr::List(vec![
                Expr::Symbol(Symbol::new("if")),
                condition,
                Expr::List(do_items),
                Expr::Nil,
            ]))
        },
    )
}

fn truthy_macro() -> NativeExprMacro {
    NativeExprMacro::new(
        Symbol::new("truthy"),
        list_macro_shape(Symbol::new("truthy"), Vec::new()),
        |_cx, _input, _bindings| Ok(Expr::Bool(true)),
    )
}

fn recursive_macro() -> NativeExprMacro {
    NativeExprMacro::new(
        Symbol::new("again"),
        list_macro_shape(Symbol::new("again"), Vec::new()),
        |_cx, _input, _bindings| Ok(Expr::List(vec![Expr::Symbol(Symbol::new("again"))])),
    )
}

fn effectful_untrusted_macro() -> NativeExprMacro {
    NativeExprMacro::new(
        Symbol::new("effectful"),
        Arc::new(EffectfulShape::new(list_macro_shape(
            Symbol::new("effectful"),
            Vec::new(),
        ))),
        |_cx, _input, _bindings| Ok(Expr::Bool(true)),
    )
}

fn source_when_macro() -> SourceTemplateMacro {
    SourceTemplateMacro::new(
        Symbol::new("when"),
        positional_macro_shape(
            Symbol::new("when"),
            &[Symbol::new("condition")],
            Some(&Symbol::new("body")),
        ),
        Expr::Quote {
            mode: QuoteMode::QuasiQuote,
            expr: Box::new(Expr::List(vec![
                Expr::Symbol(Symbol::new("if")),
                Expr::Quote {
                    mode: QuoteMode::Unquote,
                    expr: Box::new(Expr::Symbol(Symbol::new("condition"))),
                },
                Expr::List(vec![
                    Expr::Symbol(Symbol::new("do")),
                    Expr::Quote {
                        mode: QuoteMode::Splice,
                        expr: Box::new(Expr::Symbol(Symbol::new("body"))),
                    },
                ]),
                Expr::Nil,
            ])),
        },
    )
}

#[test]
fn macros_use_shape_captures_and_return_exprs() {
    let mut cx = eager_cx();
    register_macro(&mut cx, Arc::new(when_macro())).unwrap();

    let input = Expr::List(vec![
        Expr::Symbol(Symbol::new("when")),
        Expr::Symbol(Symbol::new("ready")),
        Expr::List(vec![
            Expr::Symbol(Symbol::new("send")),
            Expr::Symbol(Symbol::new("report")),
        ]),
    ]);
    let expanded = cx.expand_macros(sim_kernel::Phase::Expand, input).unwrap();

    assert_eq!(
        expanded,
        Expr::List(vec![
            Expr::Symbol(Symbol::new("if")),
            Expr::Symbol(Symbol::new("ready")),
            Expr::List(vec![
                Expr::Symbol(Symbol::new("do")),
                Expr::List(vec![
                    Expr::Symbol(Symbol::new("send")),
                    Expr::Symbol(Symbol::new("report")),
                ]),
            ]),
            Expr::Nil,
        ])
    );
}

#[test]
fn source_template_macros_expand_quasiquote_templates() {
    let mut cx = eager_cx();
    register_macro(&mut cx, Arc::new(source_when_macro())).unwrap();

    let expanded = cx
        .expand_macros(
            sim_kernel::Phase::Expand,
            Expr::List(vec![
                Expr::Symbol(Symbol::new("when")),
                Expr::Symbol(Symbol::new("ready")),
                Expr::List(vec![
                    Expr::Symbol(Symbol::new("send")),
                    Expr::Symbol(Symbol::new("report")),
                ]),
            ]),
        )
        .unwrap();

    assert_eq!(
        expanded,
        Expr::List(vec![
            Expr::Symbol(Symbol::new("if")),
            Expr::Symbol(Symbol::new("ready")),
            Expr::List(vec![
                Expr::Symbol(Symbol::new("do")),
                Expr::List(vec![
                    Expr::Symbol(Symbol::new("send")),
                    Expr::Symbol(Symbol::new("report")),
                ]),
            ]),
            Expr::Nil,
        ])
    );
}

#[test]
fn macro_expansion_is_controlled_by_phase_policy() {
    let mut cx = noop_cx();
    register_macro(&mut cx, Arc::new(truthy_macro())).unwrap();

    let error = cx
        .expand_macros(
            sim_kernel::Phase::Expand,
            Expr::List(vec![Expr::Symbol(Symbol::new("truthy"))]),
        )
        .unwrap_err();

    assert!(matches!(error, sim_kernel::Error::Eval(message) if
            message.contains("noop") &&
            (message.contains("denied by eval policy") || message.contains("not allowed"))));
}

#[test]
fn macro_shape_rejections_report_wrong_shape() {
    let mut cx = eager_cx();
    register_macro(&mut cx, Arc::new(when_macro())).unwrap();

    let error = cx
        .expand_macros(
            sim_kernel::Phase::Expand,
            Expr::List(vec![Expr::Symbol(Symbol::new("when"))]),
        )
        .unwrap_err();

    assert!(
        matches!(error, sim_kernel::Error::WrongShape { diagnostics, .. } if diagnostics.iter().any(|diagnostic| diagnostic.message.contains("macro when rejected syntax")))
    );
}

#[test]
fn macro_metadata_is_browseable() {
    let mut cx = eager_cx();
    register_macro(&mut cx, Arc::new(when_macro())).unwrap();

    let table = cx
        .registry()
        .macro_by_symbol(&Symbol::new("when"))
        .cloned()
        .unwrap()
        .object()
        .as_table(&mut cx)
        .unwrap();
    let expr = table.object().as_expr(&mut cx).unwrap();

    assert!(matches!(expr, Expr::Map(ref entries) if
        entries.iter().any(|(key, value)| {
            key == &Expr::Symbol(Symbol::new("symbol"))
                && value == &Expr::String("when".to_owned())
        })
        && entries.iter().any(|(key, value)| {
            key == &Expr::Symbol(Symbol::new("parser-trusted"))
                && value == &Expr::Bool(true)
        })
    ));
}

#[test]
fn effectful_shapes_are_rejected_in_untrusted_macro_parse_positions() {
    let mut cx = eager_cx();
    register_macro_with_parser_trust(&mut cx, Arc::new(effectful_untrusted_macro()), false)
        .unwrap();

    let error = cx
        .expand_macros(
            sim_kernel::Phase::Expand,
            Expr::List(vec![Expr::Symbol(Symbol::new("effectful"))]),
        )
        .unwrap_err();

    assert!(
        matches!(error, sim_kernel::Error::Eval(message) if message.contains("effectful syntax shape") && message.contains("untrusted parse position"))
    );
}

#[test]
fn eval_phase_expands_call_macros() {
    let mut cx = eager_cx();
    register_macro(&mut cx, Arc::new(truthy_macro())).unwrap();

    let value = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::new("truthy"))),
            args: Vec::new(),
        })
        .unwrap();

    assert_eq!(value.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
}

#[test]
fn macroexpand_function_expands_quoted_data() {
    let mut cx = eager_cx();
    register_macro(&mut cx, Arc::new(truthy_macro())).unwrap();

    let value = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::qualified("core", "macroexpand"))),
            args: vec![Expr::Quote {
                mode: QuoteMode::Quote,
                expr: Box::new(Expr::List(vec![Expr::Symbol(Symbol::new("truthy"))])),
            }],
        })
        .unwrap();

    assert_eq!(value.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
}

#[test]
fn direct_macro_context_expands_without_context_registry_hook() {
    let mut cx = eager_cx();
    register_macro(&mut cx, Arc::new(truthy_macro())).unwrap();
    let mut macro_cx = MacroCx::new(&mut cx, sim_kernel::Phase::Expand);

    let expanded = expand_expr(
        &mut macro_cx,
        Expr::List(vec![Expr::Symbol(Symbol::new("truthy"))]),
    )
    .unwrap();

    assert_eq!(expanded, Expr::Bool(true));
}

#[test]
fn macro_expansion_requires_phase_capability() {
    let mut cx = ungranted_eager_cx();
    register_macro(&mut cx, Arc::new(truthy_macro())).unwrap();

    let error = cx
        .expand_macros(
            sim_kernel::Phase::Expand,
            Expr::List(vec![Expr::Symbol(Symbol::new("truthy"))]),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        sim_kernel::Error::CapabilityDenied { capability }
            if capability == macro_expand_capability()
    ));
}

#[test]
fn direct_macro_registration_rejects_duplicates() {
    let mut cx = eager_cx();
    register_macro(&mut cx, Arc::new(truthy_macro())).unwrap();

    let error = register_macro(&mut cx, Arc::new(truthy_macro())).unwrap_err();

    assert!(matches!(
        error,
        sim_kernel::Error::DuplicateExport { kind: "macro", .. }
    ));
}

#[test]
fn recursive_macro_expansion_is_budgeted() {
    let mut cx = eager_cx();
    cx.set_macro_expander(Arc::new(RegistryMacroExpander::with_limits(
        MacroExpansionLimits {
            max_depth: 4,
            max_steps: 64,
        },
    )));
    register_macro(&mut cx, Arc::new(recursive_macro())).unwrap();

    let error = cx
        .expand_macros(
            sim_kernel::Phase::Expand,
            Expr::List(vec![Expr::Symbol(Symbol::new("again"))]),
        )
        .unwrap_err();

    assert!(
        matches!(error, sim_kernel::Error::Eval(message) if message.contains("depth limit") && message.contains("again"))
    );
}

#[test]
fn macro_context_can_make_hygienic_symbols_without_exposing_runtime_mutation() {
    let mut cx = eager_cx();
    let mut macro_cx = MacroCx::new(&mut cx, sim_kernel::Phase::Expand);

    let first = macro_cx.hygienic_symbol("tmp");
    let second = macro_cx.hygienic_symbol("tmp");

    assert_ne!(first, second);
    assert_eq!(first.namespace.as_deref(), Some("macro/anonymous"));
}

#[test]
fn macro_aliases_check_syntax_against_original_symbol() {
    let mut cx = eager_cx();
    cx.registry_mut()
        .register_macro_value(
            Symbol::qualified("alias", "truthy"),
            macro_value(Arc::new(truthy_macro())),
        )
        .unwrap();

    let expanded = cx
        .expand_macros(
            sim_kernel::Phase::Expand,
            Expr::List(vec![Expr::Symbol(Symbol::qualified("alias", "truthy"))]),
        )
        .unwrap();

    assert_eq!(expanded, Expr::Bool(true));
}
