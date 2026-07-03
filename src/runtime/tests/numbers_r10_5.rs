#[cfg(feature = "numbers-exotic")]
use crate::runtime::install_core_runtime;
#[cfg(feature = "numbers-exotic")]
use sim_kernel::{Args, Expr, Symbol};
#[cfg(feature = "numbers-exotic")]
use sim_kernel::{DefaultFactory, EagerPolicy};
#[cfg(feature = "numbers-exotic")]
use std::sync::Arc;

#[cfg(feature = "numbers-exotic")]
fn runtime() -> sim_kernel::Cx {
    let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

#[cfg(feature = "numbers-exotic")]
#[test]
fn continued_fraction_browse_reports_endless_tail() {
    let mut cx = runtime();
    let value = cx
        .registry()
        .value_by_symbol(&Symbol::new("cf-pi"))
        .unwrap()
        .clone();
    let table = value.object().as_table(&mut cx).unwrap();
    let Expr::Map(entries) = table.object().as_expr(&mut cx).unwrap() else {
        panic!("expected browse table");
    };
    let tail = entries
        .into_iter()
        .find_map(|(key, value)| match key {
            Expr::Symbol(symbol) if symbol == Symbol::new("tail") => Some(value),
            _ => None,
        })
        .unwrap();
    assert_eq!(tail, Expr::String("endless".to_owned()));
}

#[cfg(feature = "numbers-exotic")]
#[test]
fn take_reads_cf_pi_without_forcing_beyond_requested_coefficients() {
    let mut cx = runtime();
    let value = cx
        .call_function(
            &Symbol::new("take"),
            Args::new(vec![
                cx.registry()
                    .value_by_symbol(&Symbol::new("cf-pi"))
                    .unwrap()
                    .clone(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "5".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::List(vec![
            Expr::Number(sim_kernel::NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "3".to_owned(),
            }),
            Expr::Number(sim_kernel::NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "7".to_owned(),
            }),
            Expr::Number(sim_kernel::NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "15".to_owned(),
            }),
            Expr::Number(sim_kernel::NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "1".to_owned(),
            }),
            Expr::Number(sim_kernel::NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "292".to_owned(),
            }),
        ])
    );
}

#[cfg(all(
    feature = "numbers-exotic",
    feature = "numbers-f64",
    not(feature = "numbers-cas")
))]
#[test]
fn continued_fraction_adds_as_f64_without_cas() {
    let mut cx = runtime();
    let value = cx
        .call_function(
            &Symbol::qualified("math", "add"),
            Args::new(vec![
                cx.registry()
                    .value_by_symbol(&Symbol::new("cf-sqrt2"))
                    .unwrap()
                    .clone(),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "1".to_owned())
                    .unwrap(),
            ]),
        )
        .unwrap();
    let Expr::Number(number) = value.object().as_expr(&mut cx).unwrap() else {
        panic!("expected numeric literal");
    };
    assert_eq!(number.domain, Symbol::qualified("numbers", "f64"));
    let approx = number.canonical.parse::<f64>().unwrap();
    assert!((approx - (std::f64::consts::SQRT_2 + 1.0)).abs() < 1e-12);
}
