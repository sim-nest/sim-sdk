#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-cas",
    feature = "numbers-i64",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast",
    feature = "numbers-tensor-linalg"
))]
use sim_kernel::{Args, Expr, Factory, NumberLiteral, Symbol};

#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-cas",
    feature = "numbers-i64",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast",
    feature = "numbers-tensor-linalg"
))]
use super::support::eval_cx;

#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-cas",
    feature = "numbers-i64",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast",
    feature = "numbers-tensor-linalg"
))]
fn i64_num(text: &str) -> sim_kernel::Value {
    sim_kernel::DefaultFactory
        .number_literal(Symbol::qualified("numbers", "i64"), text.to_owned())
        .unwrap()
}

#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-cas",
    feature = "numbers-i64",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast",
    feature = "numbers-tensor-linalg"
))]
fn cas_var(cx: &mut sim_kernel::Cx, symbol: &str) -> sim_kernel::Value {
    cx.call_function(
        &Symbol::qualified("cas", "var"),
        Args::new(vec![
            sim_kernel::DefaultFactory
                .symbol(Symbol::new(symbol))
                .unwrap(),
        ]),
    )
    .unwrap()
}

#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-cas",
    feature = "numbers-i64",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast",
    feature = "numbers-tensor-linalg"
))]
#[test]
fn linalg_surface_supports_dot_and_matmul() {
    let mut cx = eval_cx();
    let left = cx
        .call_function(
            &Symbol::new("vec"),
            Args::new(vec![i64_num("1"), i64_num("2"), i64_num("3")]),
        )
        .unwrap();
    let right = cx
        .call_function(
            &Symbol::new("vec"),
            Args::new(vec![i64_num("4"), i64_num("5"), i64_num("6")]),
        )
        .unwrap();
    let dot = cx
        .call_function(&Symbol::new("dot"), Args::new(vec![left, right]))
        .unwrap();
    assert_eq!(
        dot.object().as_expr(&mut cx).unwrap(),
        Expr::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "i64"),
            canonical: "32".to_owned(),
        })
    );

    let eye = cx
        .call_function(&Symbol::new("eye"), Args::new(vec![i64_num("2")]))
        .unwrap();
    let rows = cx
        .factory()
        .list(vec![
            cx.factory().list(vec![i64_num("7"), i64_num("8")]).unwrap(),
            cx.factory()
                .list(vec![i64_num("9"), i64_num("10")])
                .unwrap(),
        ])
        .unwrap();
    let matrix = cx
        .call_function(&Symbol::new("mat"), Args::new(vec![rows]))
        .unwrap();
    let product = cx
        .call_function(&Symbol::new("matmul"), Args::new(vec![eye, matrix.clone()]))
        .unwrap();
    assert_eq!(
        product.object().as_expr(&mut cx).unwrap(),
        matrix.object().as_expr(&mut cx).unwrap()
    );
}

#[cfg(all(
    feature = "numbers-arith",
    feature = "numbers-cas",
    feature = "numbers-i64",
    feature = "numbers-tensor",
    feature = "numbers-tensor-bcast",
    feature = "numbers-tensor-linalg"
))]
#[test]
fn symbolic_matmul_yields_symbolic_tensor_cells() {
    let mut cx = eval_cx();
    let a = cas_var(&mut cx, "a");
    let b = cas_var(&mut cx, "b");
    let c = cas_var(&mut cx, "c");
    let d = cas_var(&mut cx, "d");
    let left_rows = cx
        .factory()
        .list(vec![
            cx.factory().list(vec![a, b]).unwrap(),
            cx.factory().list(vec![c, d]).unwrap(),
        ])
        .unwrap();
    let left = cx
        .call_function(&Symbol::new("mat"), Args::new(vec![left_rows]))
        .unwrap();
    let x = cas_var(&mut cx, "x");
    let y = cas_var(&mut cx, "y");
    let right_rows = cx
        .factory()
        .list(vec![
            cx.factory().list(vec![x]).unwrap(),
            cx.factory().list(vec![y]).unwrap(),
        ])
        .unwrap();
    let right = cx
        .call_function(&Symbol::new("mat"), Args::new(vec![right_rows]))
        .unwrap();
    let product = cx
        .call_function(&Symbol::new("matmul"), Args::new(vec![left, right]))
        .unwrap();
    match product.object().as_expr(&mut cx).unwrap() {
        Expr::Vector(rows) => assert_eq!(rows.len(), 2),
        other => panic!("expected symbolic tensor expression, got {other:?}"),
    }
}
