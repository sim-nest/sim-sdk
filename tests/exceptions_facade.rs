#![cfg(feature = "control")]

// conformance: the SDK exports the canonical raised envelope, matcher, and managed relation adapter.

#[allow(unused_imports)]
use sim::exceptions::match_raised_class;
use sim::exceptions::{ManagedException, Raised};

#[test]
fn exports_envelope_matcher_and_managed_adapter() {
    let _ = std::any::type_name::<Raised>();
    let _ = std::any::type_name::<ManagedException<(), ()>>();
}
