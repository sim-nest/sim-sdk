use std::{collections::BTreeMap, sync::Arc};

use sim::kernel::{
    CapabilitySet, Cx, DefaultFactory, EagerPolicy, Error, Expr, ReadPolicy, Symbol, TrustLevel,
    macro_expand_eval_capability, read_eval_capability,
};
use sim::lib_lang_python::{
    DynamicAdmission, DynamicPython, PYTHON_FIDELITY, PythonEvalPolicy, PythonValue,
};
use sim::shape::AnyShape;

pub fn capability_scoped_python() -> Result<(), Box<dyn std::error::Error>> {
    let (mut cx, seat) = Cx::new_seated(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    sim::runtime::install_core_runtime(&mut cx);
    let python_codec_id = cx.registry_mut().fresh_codec_id();
    cx.load_lib(&sim::codec_python::PythonCodecLib::new(python_codec_id))?;
    let dynamic = DynamicPython::default();

    let denied = dynamic.eval(
        &mut cx,
        "40 + 2",
        DynamicAdmission::new(
            ReadPolicy {
                trust: TrustLevel::Untrusted,
                capabilities: CapabilitySet::new(),
            },
            CapabilitySet::new(),
        ),
    );
    assert!(matches!(denied, Err(Error::TrustDenied { .. } | Error::CapabilityDenied { .. })));

    let read_eval = read_eval_capability();
    let expand_eval = macro_expand_eval_capability();
    seat.grant(&mut cx, read_eval.clone())?;
    seat.grant(&mut cx, expand_eval.clone())?;
    let trusted = || ReadPolicy {
        trust: TrustLevel::TrustedSource,
        capabilities: CapabilitySet::new().grant(read_eval.clone()),
    };
    let admitted = || DynamicAdmission {
        read_policy: trusted(),
        requires: vec![read_eval.clone(), expand_eval.clone()],
        allow: CapabilitySet::new().grant(expand_eval.clone()),
        expected_shape: Arc::new(AnyShape),
    };
    let evaluated = dynamic.eval(&mut cx, "40 + 2", admitted())?;
    assert!(matches!(
        evaluated.object().as_expr(&mut cx)?,
        Expr::Call { operator, .. }
            if *operator == Expr::Symbol(Symbol::qualified("python", "module"))
    ));
    let executed = dynamic.exec(&mut cx, "answer = 40 + 2\nanswer", admitted())?;
    assert!(matches!(
        executed.object().as_expr(&mut cx)?,
        Expr::Call { operator, .. }
            if *operator == Expr::Symbol(Symbol::qualified("python", "module"))
    ));

    let tree = sim::codec_python::parse_module("answer = 40 + 2\nanswer")?;
    let lowered = sim::codec_python::lower_python(&tree);
    let value = PythonEvalPolicy::new(128)?.eval_lowered(&lowered, &mut BTreeMap::new())?;
    assert_eq!(value, PythonValue::Int(42));
    assert!(PYTHON_FIDELITY.expected_gaps.iter().any(|gap| gap.contains("CPython")));
    assert!(PYTHON_FIDELITY.expected_gaps.iter().any(|gap| gap.contains("compiler")));
    Ok(())
}
