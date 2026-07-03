#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
use std::sync::Arc;

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
use sim_kernel::{ClassRef, Expr, NumberDomain, NumberLiteral, NumberValue, Object, Symbol, Value};

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) const TEST_NUMBER_DOMAIN_CLASS_ID: sim_kernel::ClassId =
    sim_kernel::CORE_NUMBER_DOMAIN_CLASS_ID;

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
#[derive(Clone)]
struct TestDomain {
    symbol: Symbol,
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
impl NumberDomain for TestDomain {
    fn symbol(&self) -> Symbol {
        self.symbol.clone()
    }

    fn parse_literal(
        &self,
        _cx: &mut sim_kernel::Cx,
        _text: &str,
    ) -> sim_kernel::Result<Option<Value>> {
        Ok(None)
    }

    fn encode_literal(
        &self,
        cx: &mut sim_kernel::Cx,
        value: Value,
    ) -> sim_kernel::Result<Option<NumberLiteral>> {
        let expr = value.object().as_expr(cx)?;
        match expr {
            Expr::Number(number) if number.domain == self.symbol => Ok(Some(number)),
            _ => Ok(None),
        }
    }
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
impl Object for TestDomain {
    fn display(&self, _cx: &mut sim_kernel::Cx) -> sim_kernel::Result<String> {
        Ok(format!("#<number-domain {}>", self.symbol))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
impl sim_kernel::ObjectCompat for TestDomain {
    fn class(&self, cx: &mut sim_kernel::Cx) -> sim_kernel::Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&Symbol::qualified("core", "NumberDomain"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            TEST_NUMBER_DOMAIN_CLASS_ID,
            Symbol::qualified("core", "NumberDomain"),
        )
    }
    fn as_number_domain(&self) -> Option<&dyn NumberDomain> {
        Some(self)
    }
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
#[derive(Clone)]
pub(super) struct OpaqueNumber {
    pub(super) domain: Symbol,
    pub(super) value: f64,
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
impl NumberValue for OpaqueNumber {
    fn number_domain(&self, _cx: &mut sim_kernel::Cx) -> sim_kernel::Result<Symbol> {
        Ok(self.domain.clone())
    }
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
impl Object for OpaqueNumber {
    fn display(&self, _cx: &mut sim_kernel::Cx) -> sim_kernel::Result<String> {
        Ok(format!("{}:{}", self.domain, self.value))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
impl sim_kernel::ObjectCompat for OpaqueNumber {
    fn class(&self, cx: &mut sim_kernel::Cx) -> sim_kernel::Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&Symbol::qualified("core", "Number"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            sim_kernel::CORE_NUMBER_CLASS_ID,
            Symbol::qualified("core", "Number"),
        )
    }
    fn as_expr(&self, _cx: &mut sim_kernel::Cx) -> sim_kernel::Result<Expr> {
        Ok(Expr::Extension {
            tag: Symbol::qualified("test", "opaque-number"),
            payload: Box::new(Expr::String(format!("{}:{}", self.domain, self.value))),
        })
    }
    fn as_number_value(&self) -> Option<&dyn NumberValue> {
        Some(self)
    }
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn install_test_domain(cx: &mut sim_kernel::Cx, symbol: Symbol) {
    let domain = cx
        .factory()
        .opaque(Arc::new(TestDomain {
            symbol: symbol.clone(),
        }))
        .unwrap();
    cx.registry_mut()
        .register_number_domain_value(symbol, domain)
        .unwrap();
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn opaque_number_value(cx: &sim_kernel::Cx, domain: Symbol, value: f64) -> Value {
    cx.factory()
        .opaque(Arc::new(OpaqueNumber { domain, value }))
        .unwrap()
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn read_opaque_number(value: &Value) -> &OpaqueNumber {
    value.object().downcast_ref::<OpaqueNumber>().unwrap()
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn promote_f64_to_decimal(
    _cx: &mut sim_kernel::Cx,
    number: NumberLiteral,
) -> sim_kernel::Result<NumberLiteral> {
    Ok(NumberLiteral {
        domain: Symbol::qualified("numbers", "decimal-test"),
        canonical: number.canonical,
    })
}

#[cfg(feature = "numbers-rational")]
pub(super) fn promote_rational_to_decimal(
    _cx: &mut sim_kernel::Cx,
    number: NumberLiteral,
) -> sim_kernel::Result<NumberLiteral> {
    Ok(NumberLiteral {
        domain: Symbol::qualified("numbers", "decimal-test"),
        canonical: number.canonical,
    })
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn decimal_add_rule(
    cx: &mut sim_kernel::Cx,
    left: NumberLiteral,
    right: NumberLiteral,
) -> sim_kernel::Result<Value> {
    let left = left.canonical.parse::<f64>().unwrap();
    let right = right.canonical.parse::<f64>().unwrap();
    cx.factory().number_literal(
        Symbol::qualified("numbers", "decimal-test"),
        (left + right).to_string(),
    )
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn promote_opaque_start_to_middle(
    cx: &mut sim_kernel::Cx,
    value: Value,
) -> sim_kernel::Result<Value> {
    let source = read_opaque_number(&value);
    Ok(opaque_number_value(
        cx,
        Symbol::qualified("numbers", "opaque-middle-test"),
        source.value + 0.0,
    ))
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn promote_opaque_middle_to_target(
    cx: &mut sim_kernel::Cx,
    value: Value,
) -> sim_kernel::Result<Value> {
    let source = read_opaque_number(&value);
    Ok(opaque_number_value(
        cx,
        Symbol::qualified("numbers", "opaque-target-test"),
        source.value,
    ))
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn promote_opaque_start_to_alt_target(
    cx: &mut sim_kernel::Cx,
    value: Value,
) -> sim_kernel::Result<Value> {
    let source = read_opaque_number(&value);
    Ok(opaque_number_value(
        cx,
        Symbol::qualified("numbers", "opaque-alt-target-test"),
        source.value,
    ))
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn opaque_add_rule(
    cx: &mut sim_kernel::Cx,
    left: Value,
    right: Value,
) -> sim_kernel::Result<Value> {
    let left = read_opaque_number(&left).value;
    let right = read_opaque_number(&right).value;
    Ok(opaque_number_value(
        cx,
        Symbol::qualified("numbers", "opaque-target-test"),
        left + right,
    ))
}

#[cfg(any(feature = "numbers-rational", feature = "numbers-i64"))]
pub(super) fn opaque_add_alt_rule(
    cx: &mut sim_kernel::Cx,
    left: Value,
    right: Value,
) -> sim_kernel::Result<Value> {
    let left = read_opaque_number(&left).value;
    let right = read_opaque_number(&right).value;
    Ok(opaque_number_value(
        cx,
        Symbol::qualified("numbers", "opaque-alt-target-test"),
        left + right,
    ))
}
