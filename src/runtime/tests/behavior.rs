use std::sync::Arc;

use sim_kernel::{
    Args, Cx, DefaultFactory, EagerPolicy, Expr, NoopEvalPolicy, NumberLiteral, Symbol,
};

use crate::runtime::{SimTest, TestExpected, install_core_runtime};

use super::support::{UnsupportedExportLib, call_expr, table_value};

#[test]
fn installs_core_runtime_objects() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "Class"))
            .is_some()
    );
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "Function"))
            .is_some()
    );
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "Macro"))
            .is_some()
    );
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "Shape"))
            .is_some()
    );
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "Codec"))
            .is_some()
    );
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "Help"))
            .is_some()
    );
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "Test"))
            .is_some()
    );
    assert!(
        cx.registry()
            .shape_by_symbol(&Symbol::qualified("core", "Any"))
            .is_some()
    );
    assert!(
        cx.registry()
            .shape_by_symbol(&Symbol::qualified("core", "Expr"))
            .is_some()
    );
    assert!(
        cx.registry()
            .shape_by_symbol(&Symbol::qualified("core", "EncodeOptions"))
            .is_some()
    );
    assert!(
        cx.registry()
            .shape_by_symbol(&Symbol::qualified("core", "MacroSyntax"))
            .is_some()
    );
    assert!(
        cx.registry()
            .function_by_symbol(&Symbol::qualified("core", "macroexpand"))
            .is_some()
    );
    for symbol in [
        "classes",
        "functions",
        "macros",
        "shapes",
        "codecs",
        "number-domains",
        "eval-policies",
        "with-eval-policy",
        "tests",
        "lib-tests",
        "run-tests",
        "help",
    ] {
        assert!(
            cx.registry()
                .function_by_symbol(&Symbol::qualified("core", symbol))
                .is_some()
        );
    }
    assert!(
        cx.registry()
            .shape_by_symbol(&Symbol::qualified("codec", "LispSurface"))
            .is_some()
    );
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::qualified("core", "String"))
            .is_some()
    );
}

#[test]
fn loaded_lib_manifests_are_browseable_as_data() {
    let mut cx = sim_kernel::Cx::new(Arc::new(sim_kernel::EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let libs = cx
        .call_function(&Symbol::qualified("core", "libs"), Args::new(Vec::new()))
        .unwrap();
    let libs_expr = libs.object().as_expr(&mut cx).unwrap();
    let Expr::List(entries) = libs_expr else {
        panic!("expected lib browse list");
    };
    assert!(!entries.is_empty());
    assert!(table_value(&entries[0], &Symbol::new("id")).is_some());
    assert!(table_value(&entries[0], &Symbol::new("exports")).is_some());
    assert!(table_value(&entries[0], &Symbol::new("trusted")).is_some());
    assert!(table_value(&entries[0], &Symbol::new("tests")).is_some());
}

#[test]
fn core_runtime_loads_as_a_browseable_lib() {
    let mut cx = sim_kernel::Cx::new(Arc::new(sim_kernel::EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let value = cx
        .call_function(
            &Symbol::qualified("core", "lib"),
            Args::new(vec![cx.factory().symbol(Symbol::new("core")).unwrap()]),
        )
        .unwrap();
    let expr = value.object().as_expr(&mut cx).unwrap();
    assert_eq!(
        table_value(&expr, &Symbol::new("id")),
        Some(&Expr::Symbol(Symbol::new("core")))
    );
}

#[test]
fn export_browse_surfaces_return_stable_tables() {
    let mut cx = sim_kernel::Cx::new(Arc::new(sim_kernel::EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let export = cx
        .call_function(
            &Symbol::qualified("core", "export"),
            Args::new(vec![
                cx.factory()
                    .symbol(Symbol::qualified("core", "help"))
                    .unwrap(),
            ]),
        )
        .unwrap();
    let export_expr = export.object().as_expr(&mut cx).unwrap();
    assert_eq!(
        table_value(&export_expr, &Symbol::new("lib")),
        Some(&Expr::Symbol(Symbol::new("core")))
    );
    assert_eq!(
        table_value(&export_expr, &Symbol::new("state")),
        Some(&Expr::Symbol(Symbol::new("resolved")))
    );
}

#[test]
fn loaded_lib_browse_surface_can_report_unsupported_exports() {
    let mut cx = sim_kernel::Cx::new(Arc::new(sim_kernel::EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx.load_lib(&UnsupportedExportLib).unwrap();
    let value = cx
        .call_function(
            &Symbol::qualified("core", "lib"),
            Args::new(vec![
                cx.factory()
                    .symbol(Symbol::qualified("test", "unsupported"))
                    .unwrap(),
            ]),
        )
        .unwrap();
    let expr = value.object().as_expr(&mut cx).unwrap();
    let Some(Expr::List(exports)) = table_value(&expr, &Symbol::new("exports")) else {
        panic!("expected export browse list");
    };
    let unsupported = exports
        .iter()
        .find(|entry| {
            table_value(entry, &Symbol::new("symbol"))
                == Some(&Expr::Symbol(Symbol::qualified("codec", "future")))
        })
        .unwrap();
    assert_eq!(
        table_value(unsupported, &Symbol::new("state")),
        Some(&Expr::Symbol(Symbol::new("unsupported")))
    );
}

#[test]
fn lambda_uses_scoped_locals_without_leaking_bindings() {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let false_value = cx.factory().bool(false).unwrap();
    cx.env_mut().define(Symbol::new("x"), false_value);
    let lambda = call_expr(
        Symbol::new("lambda"),
        vec![
            Expr::List(vec![Expr::Symbol(Symbol::new("x"))]),
            Expr::Symbol(Symbol::new("x")),
        ],
    );
    let result = cx
        .eval_expr(Expr::Call {
            operator: Box::new(lambda),
            args: vec![Expr::Bool(true)],
        })
        .unwrap();
    assert_eq!(result.object().as_expr(&mut cx).unwrap(), Expr::Bool(true));
}

#[test]
fn lambda_class_destructuring_binds_object_fields() {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let lambda = call_expr(
        Symbol::new("lambda"),
        vec![
            Expr::List(vec![Expr::List(vec![
                Expr::Symbol(Symbol::new("Point")),
                Expr::Symbol(Symbol::new("x")),
                Expr::Symbol(Symbol::new("y")),
            ])]),
            Expr::Symbol(Symbol::new("x")),
        ],
    );
    let point = sim_shape::ObjectExpr {
        class: Symbol::new("Point"),
        fields: vec![
            (
                Symbol::new("x"),
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "f64"),
                    canonical: "3".to_owned(),
                }),
            ),
            (
                Symbol::new("y"),
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "f64"),
                    canonical: "4".to_owned(),
                }),
            ),
        ],
    }
    .to_expr();
    let result = cx
        .eval_expr(Expr::Call {
            operator: Box::new(lambda),
            args: vec![point],
        })
        .unwrap();
    assert_eq!(
        result.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "3".to_owned()
        })
    );
}

#[test]
fn help_surface_describes_function_calls() {
    let mut cx = sim_kernel::Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let help = cx
        .call_function(
            &Symbol::qualified("core", "help"),
            Args::new(vec![
                cx.factory()
                    .symbol(Symbol::qualified("core", "lib"))
                    .unwrap(),
            ]),
        )
        .unwrap();
    let table = help.object().as_table(&mut cx).unwrap();
    let expr = table.object().as_expr(&mut cx).unwrap();
    assert_eq!(
        table_value(&expr, &Symbol::new("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "function")))
    );
}

#[test]
fn registered_tests_are_browseable_and_runnable() {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    let test = SimTest::new(
        Symbol::qualified("test", "truthy"),
        Symbol::qualified("test", "runtime"),
        Expr::Bool(true),
        TestExpected::Truthy,
        vec![Symbol::qualified("core", "help")],
    );
    cx.registry_mut()
        .register_test(
            Symbol::qualified("test", "truthy"),
            Symbol::qualified("test", "runtime"),
            Arc::new(test),
            vec![Symbol::qualified("core", "help")],
        )
        .unwrap();
    let tests = cx
        .call_function(&Symbol::qualified("core", "tests"), Args::new(Vec::new()))
        .unwrap();
    let tests_expr = tests.object().as_expr(&mut cx).unwrap();
    let Expr::List(tests) = tests_expr else {
        panic!("expected tests list");
    };
    assert!(tests.iter().any(|test| {
        table_value(test, &Symbol::new("name"))
            == Some(&Expr::Symbol(Symbol::qualified("test", "truthy")))
    }));
}
