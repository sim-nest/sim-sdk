use sim::kernel::{Expr, Symbol};

use crate::support::CONFORMANCE_CONTRACT;

pub(crate) const GPU_MATH_PATH: &str = "crates/sim-conformance/tests/spec/gpu_math.rs";

#[test]
fn gpu_math_composition_reexports_canonical_components() {
    assert_eq!(
        GPU_MATH_PATH,
        "crates/sim-conformance/tests/spec/gpu_math.rs"
    );
    assert!(CONFORMANCE_CONTRACT.contains(GPU_MATH_PATH));

    assert_eq!(
        sim::compute_model::compute_model_site_symbol().as_qualified_str(),
        "site/compute/model"
    );
    assert_eq!(
        sim::compute_femm::compute_femm_lib_symbol().as_qualified_str(),
        "compute/femm-lib"
    );
    assert_eq!(
        sim::numbers_tensor::tensor_site_symbol().as_qualified_str(),
        "site/tensor"
    );
    assert_eq!(
        sim::femm_solve::linear_solver_symbol().as_qualified_str(),
        "femm/linear-solver"
    );
}

#[test]
fn gpu_math_provider_swap_preserves_expression_structure() {
    let modeled = gpu_math_expression(sim::compute_model::compute_model_site_symbol());
    let automatic = gpu_math_expression(sim::compute_auto::compute_auto_site_symbol());

    assert_eq!(
        math_body(&modeled).canonical_key(),
        math_body(&automatic).canonical_key()
    );
    assert_ne!(
        placement(&modeled).canonical_key(),
        placement(&automatic).canonical_key()
    );
}

fn gpu_math_expression(site: Symbol) -> Expr {
    Expr::Map(vec![
        (
            Expr::Symbol(Symbol::new("placement")),
            Expr::Map(vec![(
                Expr::Symbol(Symbol::new("site")),
                Expr::Symbol(site),
            )]),
        ),
        (
            Expr::Symbol(Symbol::new("math")),
            Expr::List(vec![
                Expr::Symbol(Symbol::qualified("tensor", "matmul")),
                Expr::Symbol(Symbol::qualified("ode", "rk-fixed")),
                Expr::Symbol(Symbol::qualified("femm", "resident-csr-solve")),
            ]),
        ),
    ])
}

fn math_body(expr: &Expr) -> &Expr {
    map_value(expr, "math")
}

fn placement(expr: &Expr) -> &Expr {
    map_value(expr, "placement")
}

fn map_value<'a>(expr: &'a Expr, key: &str) -> &'a Expr {
    let Expr::Map(entries) = expr else {
        panic!("expected map expression");
    };
    entries
        .iter()
        .find_map(|(candidate, value)| match candidate {
            Expr::Symbol(symbol) if symbol.as_qualified_str() == key => Some(value),
            _ => None,
        })
        .expect("map key present")
}
