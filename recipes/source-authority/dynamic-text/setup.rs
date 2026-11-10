// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use sim::kernel::{
    CapabilityName, CapabilitySet, Cx, DefaultFactory, EagerPolicy, Error, ReadPolicy, Symbol,
    TrustLevel, macro_expand_eval_capability, read_eval_capability,
};
use sim::shape::AnyShape;
use sim::source_authority::{DynamicSourcePolicy, RequestOrigin, SourceAuthority};

pub fn dynamic_text_source_authority() -> Result<(), Box<dyn std::error::Error>> {
    let (mut cx, seat) = Cx::new_seated(Arc::new(EagerPolicy), Arc::new(DefaultFactory), sim_kernel::HandleSeed::new(0x0afe_dfec_e818_84d4));
    sim::runtime::install_core_runtime(&mut cx);
    let codec_id = cx.registry_mut().fresh_codec_id();
    let lisp = sim::codec_lisp::LispCodecLib::new(codec_id)?;
    cx.load_lib(&lisp)?;

    let read_eval = read_eval_capability();
    let macro_expand = macro_expand_eval_capability();
    let source_run = CapabilityName::new("example.source.run");
    seat.grant(&mut cx, read_eval.clone())?;
    seat.grant(&mut cx, macro_expand.clone())?;
    let authority = || {
        SourceAuthority::new(
            ReadPolicy {
                trust: TrustLevel::TrustedSource,
                capabilities: CapabilitySet::new().grant(read_eval.clone()),
            },
            vec![source_run.clone()],
            CapabilitySet::new().grant(macro_expand.clone()),
        )
    };
    let policy = DynamicSourcePolicy::new(
        Symbol::qualified("codec", "lisp"),
        RequestOrigin::new(Symbol::qualified("example", "dynamic-text")),
    );

    let denied = policy.evaluate_text(&mut cx, "42", authority()?, Arc::new(AnyShape));
    assert!(matches!(denied, Err(Error::CapabilityDenied { .. })));

    seat.grant(&mut cx, source_run.clone())?;
    let value = policy.evaluate_text(&mut cx, "42", authority()?, Arc::new(AnyShape))?;
    assert_eq!(value.object().display(&mut cx)?, "42");
    Ok(())
}
