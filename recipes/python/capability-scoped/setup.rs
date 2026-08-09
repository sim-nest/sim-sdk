use std::{collections::BTreeMap, sync::Arc};

use sim::kernel::{
    CapabilitySet, Cx, DefaultFactory, EagerPolicy, Error, ReadPolicy, TrustLevel,
    read_eval_capability,
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

    seat.grant(&mut cx, read_eval_capability())?;
    let trusted = || ReadPolicy {
        trust: TrustLevel::TrustedSource,
        capabilities: CapabilitySet::new().grant(read_eval_capability()),
    };
    let admitted = || DynamicAdmission {
        read_policy: trusted(),
        requires: Vec::new(),
        allow: CapabilitySet::new(),
        expected_shape: Arc::new(AnyShape),
    };
    for authorized in [
        dynamic.eval(&mut cx, "40 + 2", admitted()),
        dynamic.exec(&mut cx, "answer = 40 + 2\nanswer", admitted()),
    ] {
        assert!(matches!(authorized, Err(Error::TypeMismatch { .. })));
    }

    let tree = sim::codec_python::parse_module("answer = 40 + 2\nanswer")?;
    let lowered = sim::codec_python::lower_python(&tree);
    let value = PythonEvalPolicy::new(128)?.eval_lowered(&lowered, &mut BTreeMap::new())?;
    assert_eq!(value, PythonValue::Int(42));
    assert!(PYTHON_FIDELITY.expected_gaps.iter().any(|gap| gap.contains("CPython")));
    assert!(PYTHON_FIDELITY.expected_gaps.iter().any(|gap| gap.contains("compiler")));
    Ok(())
}
