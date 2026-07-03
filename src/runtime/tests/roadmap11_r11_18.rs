use std::sync::Arc;

use sim_kernel::{Args, DefaultFactory, EagerPolicy, Expr, Symbol};

use crate::runtime::install_core_runtime;

use super::support::table_value;

#[test]
fn roadmap11_browse_lists_builtin_plugins_and_help_shows_shape_columns() {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);

    let lib = cx
        .call_function(
            &Symbol::qualified("core", "lib"),
            Args::new(vec![
                cx.factory()
                    .symbol(Symbol::new("pitch-namer"))
                    .expect("symbol"),
            ]),
        )
        .expect("lib browse");
    let lib_expr = lib.object().as_expr(&mut cx).expect("expr");
    let Some(Expr::List(exports)) = table_value(&lib_expr, &Symbol::new("exports")) else {
        panic!("expected export list");
    };
    assert!(exports.iter().any(|entry| {
        table_value(entry, &Symbol::new("symbol"))
            == Some(&Expr::Symbol(Symbol::qualified("pitch", "ForteNamer")))
    }));

    let help = cx
        .call_function(
            &Symbol::qualified("core", "help"),
            Args::new(vec![
                cx.factory()
                    .symbol(Symbol::qualified("pitch", "ForteNamer"))
                    .expect("symbol"),
            ]),
        )
        .expect("help");
    let help_expr = help
        .object()
        .as_table(&mut cx)
        .expect("table")
        .object()
        .as_expr(&mut cx)
        .expect("expr");
    assert_eq!(
        table_value(&help_expr, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "value")))
    );

    let browse = cx
        .resolve_value(&Symbol::qualified("pitch", "ForteNamer"))
        .expect("browse value");
    let browse_expr = browse.object().as_expr(&mut cx).expect("expr");
    assert_eq!(
        table_value(&browse_expr, &Symbol::new("shape")),
        Some(&Expr::Symbol(Symbol::qualified("pitch", "ClusterNamer")))
    );
    assert_eq!(
        table_value(&browse_expr, &Symbol::new("layer")),
        Some(&Expr::String("pitch".to_owned()))
    );
    assert_eq!(
        table_value(&browse_expr, &Symbol::new("lossless")),
        Some(&Expr::Bool(true))
    );
}
