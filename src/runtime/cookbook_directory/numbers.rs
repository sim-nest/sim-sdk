macro_rules! cookbook_directory_numbers {
    ($m:ident) => {
        $m!(
            "numbers/i64",
            "I64 numbers",
            "numbers-i64",
            Some(crate::numbers_i64::RECIPES),
            || Box::new(crate::numbers_i64::I64NumbersLib::new())
        );
        $m!(
            "numbers/arith",
            "Arithmetic numbers",
            "numbers-arith",
            Some(crate::numbers_arith::RECIPES),
            || Box::new(crate::numbers_arith::NumbersArithmeticLib::new())
        );
        $m!(
            "numbers/bigint",
            "Big integer numbers",
            "numbers-bigint",
            Some(crate::numbers_bigint::RECIPES),
            || Box::new(crate::numbers_bigint::BigIntNumbersLib::new())
        );
        $m!(
            "numbers/bool",
            "Boolean numbers",
            "numbers-bool",
            Some(crate::numbers_bool::RECIPES),
            || Box::new(crate::numbers_bool::BoolNumbersLib::new())
        );
        $m!(
            "numbers/f64",
            "F64 numbers",
            "numbers-f64",
            Some(crate::numbers_f64::RECIPES),
            || Box::new(crate::numbers_f64::F64NumbersLib::new())
        );
        $m!(
            "numbers/rational",
            "Rational numbers",
            "numbers-rational",
            Some(crate::numbers_rational::RECIPES),
            || Box::new(crate::numbers_rational::RationalNumbersLib::new())
        );
        $m!(
            "numbers/complex",
            "Complex numbers",
            "numbers-complex",
            Some(crate::numbers_complex::RECIPES),
            || Box::new(crate::numbers_complex::ComplexNumbersLib::new())
        );
        $m!(
            "numbers/func",
            "Function numbers",
            "numbers-func",
            Some(crate::numbers_func::RECIPES),
            || Box::new(crate::numbers_func::FuncNumbersLib::new())
        );
        $m!(
            "numbers/cas",
            "CAS numbers",
            "numbers-cas",
            Some(crate::numbers_cas::RECIPES),
            || Box::new(crate::numbers_cas::CasNumbersLib::new())
        );
        $m!(
            "numbers/cas-diff",
            "CAS diff numbers",
            "numbers-cas-diff",
            Some(crate::numbers_cas_diff::RECIPES),
            || Box::new(crate::numbers_cas_diff::CasDiffLib::new())
        );
        $m!(
            "numbers/cas-eval",
            "CAS eval numbers",
            "numbers-cas-eval",
            Some(crate::numbers_cas_eval::RECIPES),
            || Box::new(crate::numbers_cas_eval::CasEvalLib::new())
        );
        $m!(
            "numbers/cf",
            "Exotic numbers",
            "numbers-exotic",
            Some(crate::numbers_exotic::RECIPES),
            || Box::new(crate::numbers_exotic::ExoticNumbersLib::new())
        );
        $m!(
            "numbers/fixed",
            "Fixed-point numbers",
            "numbers-fixed",
            Some(crate::numbers_fixed::RECIPES),
            || Box::new(crate::numbers_fixed::FixedNumbersLib::new())
        );
        $m!(
            "numbers/float",
            "Float numbers",
            "numbers-float",
            Some(crate::numbers_float::RECIPES),
            || Box::new(crate::numbers_float::F32NumbersLib::new())
        );
        $m!(
            "numbers/numeric",
            "Numeric methods",
            "numbers-numeric",
            Some(crate::numbers_numeric::RECIPES),
            || Box::new(crate::numbers_numeric::NumericNumbersLib::new())
        );
        $m!(
            "numbers/quad",
            "Quadrature numbers",
            "numbers-quad",
            Some(crate::numbers_quad::RECIPES),
            || Box::new(crate::numbers_quad::QuadNumbersLib::new())
        );
        $m!(
            "numbers/rk",
            "Runge-Kutta numbers",
            "numbers-rk",
            Some(crate::numbers_rk::RECIPES),
            || Box::new(crate::numbers_rk::RkNumbersLib::new())
        );
        $m!(
            "numbers/tensor",
            "Tensor numbers",
            "numbers-tensor",
            Some(crate::numbers_tensor::RECIPES),
            || Box::new(crate::numbers_tensor::TensorNumbersLib::new())
        );
        $m!(
            "numbers/tensor-bcast",
            "Tensor broadcast numbers",
            "numbers-tensor-bcast",
            Some(crate::numbers_tensor_bcast::RECIPES),
            || Box::new(crate::numbers_tensor_bcast::TensorBroadcastLib::new())
        );
        $m!(
            "numbers/tensor-bit",
            "Bit tensor numbers",
            "numbers-tensor-bit",
            Some(crate::numbers_tensor_bit::RECIPES),
            || Box::new(crate::numbers_tensor_bit::BitTensorLib::new())
        );
        $m!(
            "numbers/tensor-cmplxf",
            "Complex tensor numbers",
            "numbers-tensor-cmplxf",
            Some(crate::numbers_tensor_cmplxf::RECIPES),
            || Box::new(crate::numbers_tensor_cmplxf::ComplexFTensorLib::new())
        );
        $m!(
            "numbers/tensor-f64",
            "F64 tensor numbers",
            "numbers-tensor-f64",
            Some(crate::numbers_tensor_f64::RECIPES),
            || Box::new(crate::numbers_tensor_f64::F64TensorLib::new())
        );
        $m!(
            "numbers/tensor-i64",
            "I64 tensor numbers",
            "numbers-tensor-i64",
            Some(crate::numbers_tensor_i64::RECIPES),
            || Box::new(crate::numbers_tensor_i64::I64TensorLib::new())
        );
        $m!(
            "numbers/tensor-linalg",
            "Tensor linear algebra",
            "numbers-tensor-linalg",
            Some(crate::numbers_tensor_linalg::RECIPES),
            || Box::new(crate::numbers_tensor_linalg::TensorLinalgLib::new())
        );
        $m!(
            "numbers/tensor-rat64",
            "Rational tensor numbers",
            "numbers-tensor-rat64",
            Some(crate::numbers_tensor_rat64::RECIPES),
            || Box::new(crate::numbers_tensor_rat64::Rat64TensorLib::new())
        );
    };
}
