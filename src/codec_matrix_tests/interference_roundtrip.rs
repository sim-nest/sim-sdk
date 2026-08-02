use sim_codec::{Input, Output, decode_with_codec};
use sim_citizen::CitizenRuntime;
use sim_kernel::{
    CapabilitySet, ObjectCompat, ReadPolicy, TrustLevel, read_construct_capability,
};

use super::support::{codec_symbols, cx, encode_once};

#[test]
fn interference_study_roundtrips_through_every_general_codec() {
    let mut cx = cx();
    crate::interference_runtime::install_interference_records(&mut cx).unwrap();
    let study = crate::interference_runtime::StudyDescriptor::example();
    let expr = study.as_expr(&mut cx).unwrap();

    for codec in codec_symbols() {
        let encoded = encode_once(&mut cx, &codec, &expr);
        let input = match encoded {
            Output::Text(text) => Input::Text(text),
            Output::Bytes(bytes) => Input::Bytes(bytes),
        };
        let decoded = decode_with_codec(
            &mut cx,
            &codec,
            input,
            ReadPolicy {
                trust: TrustLevel::TrustedSource,
                capabilities: CapabilitySet::new().grant(read_construct_capability()),
            },
        )
        .unwrap_or_else(|error| panic!("codec {codec} failed Study decode: {error:?}"));
        assert!(
            decoded.canonical_eq(&expr),
            "codec {codec} changed the canonical interference Study"
        );
    }
}
