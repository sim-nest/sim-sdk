use std::sync::Arc;

use sim_kernel::{Args, Cx, DefaultFactory, Expr, NoopEvalPolicy, Symbol, Value};

use crate::runtime::install_core_runtime;

use super::support::table_value;

const SHAPE_HELPERS: &[&str] = &[
    "and",
    "all",
    "or",
    "any",
    "not",
    "none",
    "without",
    "list",
    "list-rest",
    "table",
    "table-required",
    "table-open",
    "table-closed",
    "repeat",
    "repeat-bounds",
    "compare",
    "compare-with",
    "venn",
    "venn-union",
    "venn-intersection",
    "venn-only",
    "venn-outside",
    "venn-exactly",
    "hook",
    "hook-trace",
    "hook-score-floor",
    "hook-accept-on-no-diagnostics",
    "hook-discard-on-diagnostic-prefix",
];

#[test]
fn shape_extension_helpers_are_browseable_with_authored_help() {
    let mut cx = test_cx();

    for name in SHAPE_HELPERS {
        let subject = Symbol::qualified("shape", *name);
        let subject_value = cx.factory().symbol(subject.clone()).unwrap();
        let card = call(
            &mut cx,
            Symbol::qualified("core", "browse"),
            vec![subject_value],
        );
        let card = expr(&mut cx, &card);

        assert_eq!(
            table_value(&card, &field("subject")),
            Some(&Expr::Symbol(subject.clone())),
            "{subject} Card should keep subject identity"
        );
        assert_eq!(
            table_value(&card, &field("kind")),
            Some(&Expr::Symbol(Symbol::qualified("core", "function"))),
            "{subject} should browse as a function"
        );
        assert_eq!(
            table_value(&card, &field("shape-known")),
            Some(&Expr::Bool(true)),
            "{subject} should publish call shapes"
        );

        let help = table_value(&card, &field("help")).expect("help");
        assert_eq!(
            table_value(help, &field("kind")),
            Some(&Expr::Symbol(Symbol::qualified("core", "function"))),
            "{subject} help should be authored as function help"
        );
        assert!(
            matches!(
                table_value(help, &field("summary")),
                Some(Expr::String(summary)) if !summary.is_empty()
            ),
            "{subject} should have a summary"
        );
        assert!(
            matches!(
                table_value(help, &field("detail")),
                Some(Expr::String(detail)) if detail.contains("shape:")
            ),
            "{subject} should have a shape helper detail"
        );
        assert!(
            has_shape_see_also(table_value(help, &field("see-also")).expect("see-also")),
            "{subject} should link to related shape helpers"
        );

        let subject_value = cx.factory().symbol(subject.clone()).unwrap();
        let legacy_help = call(
            &mut cx,
            Symbol::qualified("core", "help"),
            vec![subject_value],
        );
        let legacy_help = expr(&mut cx, &legacy_help);
        assert!(
            matches!(
                table_value(&legacy_help, &field("purpose")),
                Some(Expr::String(summary)) if !summary.is_empty()
            ),
            "{subject} should also be discoverable through core/help"
        );
    }
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn call(cx: &mut Cx, symbol: Symbol, args: Vec<Value>) -> Value {
    cx.call_function(&symbol, Args::new(args)).unwrap()
}

fn expr(cx: &mut Cx, value: &Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

fn has_shape_see_also(expr: &Expr) -> bool {
    let Expr::List(items) = expr else {
        return false;
    };
    items.iter().any(|item| {
        matches!(
            item,
            Expr::Symbol(symbol) if symbol.namespace.as_deref() == Some("shape")
        )
    })
}

fn field(name: &str) -> Symbol {
    Symbol::new(name)
}
