#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast"
))]
use sim_kernel::{Args, Expr, Factory, NumberLiteral, Symbol};

#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast"
))]
use super::support::eval_cx;

#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast"
))]
fn num(domain: &str, canonical: &str) -> sim_kernel::Value {
    sim_kernel::DefaultFactory
        .number_literal(Symbol::qualified("numbers", domain), canonical.to_owned())
        .unwrap()
}

#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-rational",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast"
))]
#[test]
fn tensor_surface_supports_build_index_and_broadcast() {
    let mut cx = eval_cx();
    let vector = cx
        .call_function(
            &Symbol::new("vec"),
            Args::new(vec![num("i64", "1"), num("i64", "2"), num("i64", "3")]),
        )
        .unwrap();
    let matrix_rows = cx
        .factory()
        .list(vec![
            cx.factory()
                .list(vec![num("i64", "1"), num("i64", "2")])
                .unwrap(),
            cx.factory()
                .list(vec![num("i64", "3"), num("i64", "4")])
                .unwrap(),
        ])
        .unwrap();
    let matrix = cx
        .call_function(&Symbol::new("mat"), Args::new(vec![matrix_rows]))
        .unwrap();
    let index = cx
        .call_function(
            &Symbol::new("index"),
            Args::new(vec![vector.clone(), num("i64", "1")]),
        )
        .unwrap();
    assert_eq!(
        index.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "i64"),
            canonical: "2".to_owned(),
        })
    );

    let scalar_added = cx
        .call_function(&Symbol::new("+"), Args::new(vec![num("i64", "1"), vector]))
        .unwrap();
    assert_eq!(
        scalar_added.object().as_expr(&mut cx).unwrap(),
        Expr::Vector(vec![
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "2".to_owned(),
            }),
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "3".to_owned(),
            }),
            Expr::Number(NumberLiteral {
                domain: Symbol::qualified("numbers", "i64"),
                canonical: "4".to_owned(),
            }),
        ])
    );

    let scaled = cx
        .call_function(
            &Symbol::new("*"),
            Args::new(vec![matrix.clone(), num("i64", "10")]),
        )
        .unwrap();
    assert_eq!(
        scaled.object().as_expr(&mut cx).unwrap(),
        Expr::Vector(vec![
            Expr::Vector(vec![
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "10".to_owned(),
                }),
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "20".to_owned(),
                }),
            ]),
            Expr::Vector(vec![
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "30".to_owned(),
                }),
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "40".to_owned(),
                }),
            ]),
        ])
    );

    let vector_rhs = cx
        .call_function(
            &Symbol::new("vec"),
            Args::new(vec![num("i64", "10"), num("i64", "20")]),
        )
        .unwrap();
    let broadcast = cx
        .call_function(&Symbol::new("+"), Args::new(vec![matrix, vector_rhs]))
        .unwrap();
    assert_eq!(
        broadcast.object().as_expr(&mut cx).unwrap(),
        Expr::Vector(vec![
            Expr::Vector(vec![
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "11".to_owned(),
                }),
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "22".to_owned(),
                }),
            ]),
            Expr::Vector(vec![
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "13".to_owned(),
                }),
                Expr::Number(NumberLiteral {
                    domain: Symbol::qualified("numbers", "i64"),
                    canonical: "24".to_owned(),
                }),
            ]),
        ])
    );
}
