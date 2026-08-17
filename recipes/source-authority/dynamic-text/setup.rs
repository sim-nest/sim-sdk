use std::sync::Arc;

use sim::kernel::{
    CapabilityName, CapabilitySet, Cx, DefaultFactory, EagerPolicy, Error, ReadPolicy, Symbol,
    TrustLevel, read_eval_capability,
};
use sim::shape::AnyShape;
use sim::source_authority::{DynamicSourcePolicy, RequestOrigin, SourceAuthority};

pub fn dynamic_text_source_authority() -> Result<(), Box<dyn std::error::Error>> {
    let (mut cx, seat) = Cx::new_seated(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    sim::runtime::install_core_runtime(&mut cx);
    let codec_id = cx.registry_mut().fresh_codec_id();
    cx.load_lib(&sim::codec_lisp::LispCodecLib::new(codec_id))?;

    let read_eval = read_eval_capability();
    let source_run = CapabilityName::new("example.source.run");
    seat.grant(&mut cx, read_eval.clone())?;
    let authority = || {
        SourceAuthority::new(
            ReadPolicy {
                trust: TrustLevel::TrustedSource,
                capabilities: CapabilitySet::new().grant(read_eval.clone()),
            },
            vec![source_run.clone()],
            CapabilitySet::new(),
        )
    };
    let policy = DynamicSourcePolicy::new(
        Symbol::qualified("codec", "lisp"),
        RequestOrigin::new(Symbol::qualified("example", "dynamic-text")),
    );

    let denied = policy.evaluate_text(&mut cx, "42", authority()?, Arc::new(AnyShape));
    assert!(matches!(denied, Err(Error::CapabilityDenied { .. })));

    seat.grant(&mut cx, source_run)?;
    let value = policy.evaluate_text(&mut cx, "42", authority()?, Arc::new(AnyShape))?;
    assert_eq!(value.object().display(&mut cx)?, "42");
    Ok(())
}
