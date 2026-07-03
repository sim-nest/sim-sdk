use std::sync::Arc;

use sim_kernel::{
    AbiVersion, Cx, Lib, LibManifest, LibTarget, Linker, LoadCx, Result, Symbol, Version,
};

use crate::{
    macros::RegistryMacroExpander,
    runtime::{
        browse::reflection::install_schema_examples,
        install::{
            register::{register_core_classes, register_core_functions, register_core_values},
            register_shapes::register_core_shapes,
        },
    },
};

use super::{exports::core_exports, optional::install_optional_runtime_libs};

/// Lib that registers the core runtime: its classes, shapes, functions, and
/// values.
pub struct CoreRuntimeLib;

impl Lib for CoreRuntimeLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: Symbol::new("core"),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: core_exports(),
        }
    }

    fn load(&self, cx: &mut LoadCx, linker: &mut Linker<'_>) -> Result<()> {
        register_core_classes(cx, linker)?;
        register_core_shapes(cx, linker)?;
        register_core_functions(cx, linker)?;
        register_core_values(cx, linker)?;
        Ok(())
    }
}

/// Installs the core runtime into a context: registers the macro expander, the
/// core lib, and the default number domains for the enabled features.
pub fn install_core_runtime(cx: &mut Cx) {
    cx.set_macro_expander(Arc::new(RegistryMacroExpander::new()));

    let loaded = sim_lib_core::install_once(cx, &CoreRuntimeLib)
        .expect("core runtime should load through the lib registry");
    if !loaded {
        install_schema_examples(cx).expect("core runtime should install browse schema examples");
        return;
    }

    #[cfg(feature = "numbers-f64")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "f64"),
        crate::numbers_f64::F64NumbersLib::new(),
        "core runtime should install the default numbers/f64 domain",
    );
    #[cfg(feature = "numbers-i64")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "i64"),
        crate::numbers_i64::I64NumbersLib::new(),
        "core runtime should install the default numbers/i64 domain",
    );
    #[cfg(feature = "numbers-bool")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "bool"),
        crate::numbers_bool::BoolNumbersLib::new(),
        "core runtime should install the numbers/bool domain",
    );
    #[cfg(feature = "numbers-fixed")]
    install_registered_lib(
        cx,
        Symbol::qualified("numbers", "fixed"),
        crate::numbers_fixed::FixedNumbersLib::new(),
        "core runtime should install fixed integer domains",
    );
    #[cfg(feature = "numbers-float")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "f32"),
        crate::numbers_float::F32NumbersLib::new(),
        "core runtime should install the numbers/f32 domain",
    );
    #[cfg(feature = "numbers-bigint")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "bigint"),
        crate::numbers_bigint::BigIntNumbersLib::new(),
        "core runtime should install the numbers/bigint domain",
    );
    #[cfg(feature = "numbers-rational")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "rational"),
        crate::numbers_rational::RationalNumbersLib::new(),
        "core runtime should install the default numbers/rational domain",
    );
    #[cfg(feature = "numbers-tensor")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "tensor"),
        crate::numbers_tensor::TensorNumbersLib::new(),
        "core runtime should install the default numbers/tensor domain",
    );
    #[cfg(feature = "numbers-cas")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "cas"),
        crate::numbers_cas::CasNumbersLib::new(),
        "core runtime should install the default numbers/cas domain",
    );
    #[cfg(feature = "numbers-cas-diff")]
    install_function_lib(
        cx,
        Symbol::new("diff"),
        crate::numbers_cas_diff::CasDiffLib::new(),
        "core runtime should install CAS differentiation helpers",
    );
    #[cfg(feature = "numbers-cas-eval")]
    install_function_lib(
        cx,
        Symbol::new("eval-cas"),
        crate::numbers_cas_eval::CasEvalLib::new(),
        "core runtime should install CAS evaluation helpers",
    );
    #[cfg(feature = "numbers-complex")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "complex"),
        crate::numbers_complex::ComplexNumbersLib::new(),
        "core runtime should install the default numbers/complex domain",
    );
    #[cfg(feature = "numbers-exotic")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "cf"),
        crate::numbers_exotic::ExoticNumbersLib::new(),
        "core runtime should install the default numbers/cf domain",
    );
    #[cfg(feature = "numbers-func")]
    install_number_domain(
        cx,
        Symbol::qualified("numbers", "func"),
        crate::numbers_func::FuncNumbersLib::new(),
        "core runtime should install the default numbers/func domain",
    );
    #[cfg(feature = "numbers-arith")]
    install_function_lib(
        cx,
        Symbol::qualified("math", "add"),
        crate::numbers_arith::NumbersArithmeticLib::new(),
        "core runtime should install generic numeric arithmetic functions",
    );
    #[cfg(feature = "numbers-numeric")]
    install_function_lib(
        cx,
        Symbol::new("numeric-diff"),
        crate::numbers_numeric::NumericNumbersLib::new(),
        "core runtime should install numeric method registry functions",
    );
    #[cfg(feature = "numbers-quad")]
    install_registered_lib(
        cx,
        Symbol::qualified("numbers", "quad"),
        crate::numbers_quad::QuadNumbersLib::new(),
        "core runtime should install standard quadrature and finite-difference plugins",
    );
    #[cfg(feature = "numbers-rk")]
    install_registered_lib(
        cx,
        Symbol::qualified("numbers", "rk"),
        crate::numbers_rk::RkNumbersLib::new(),
        "core runtime should install standard ODE solver plugins",
    );
    #[cfg(feature = "numbers-tensor-bcast")]
    install_registered_lib(
        cx,
        Symbol::qualified("numbers", "tensor-bcast"),
        crate::numbers_tensor_bcast::TensorBroadcastLib::new(),
        "core runtime should install tensor broadcasting rules",
    );
    #[cfg(feature = "numbers-tensor-linalg")]
    install_function_lib(
        cx,
        Symbol::new("dot"),
        crate::numbers_tensor_linalg::TensorLinalgLib::new(),
        "core runtime should install tensor linear algebra helpers",
    );

    install_optional_runtime_libs(cx);
    install_schema_examples(cx).expect("core runtime should install browse schema examples");
}

#[cfg(any(
    feature = "numbers-f64",
    feature = "numbers-i64",
    feature = "numbers-bool",
    feature = "numbers-float",
    feature = "numbers-bigint",
    feature = "numbers-rational",
    feature = "numbers-tensor",
    feature = "numbers-cas",
    feature = "numbers-complex",
    feature = "numbers-exotic",
    feature = "numbers-func"
))]
fn install_number_domain<L: Lib>(cx: &mut Cx, symbol: Symbol, lib: L, message: &'static str) {
    if cx.registry().number_domain_by_symbol(&symbol).is_none() {
        cx.load_lib(&lib).expect(message);
    }
}

#[cfg(any(
    feature = "numbers-fixed",
    feature = "numbers-tensor-bcast",
    feature = "numbers-quad",
    feature = "numbers-rk"
))]
fn install_registered_lib<L: Lib>(cx: &mut Cx, symbol: Symbol, lib: L, message: &'static str) {
    let _ = symbol;
    sim_lib_core::install_once(cx, &lib).expect(message);
}

#[cfg(any(
    feature = "numbers-cas-diff",
    feature = "numbers-cas-eval",
    feature = "numbers-arith",
    feature = "numbers-numeric",
    feature = "numbers-tensor-linalg"
))]
fn install_function_lib<L: Lib>(cx: &mut Cx, symbol: Symbol, lib: L, message: &'static str) {
    if cx.registry().function_by_symbol(&symbol).is_none() {
        cx.load_lib(&lib).expect(message);
    }
}
