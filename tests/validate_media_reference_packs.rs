// conformance: media references preserve authored meaning, currency, and removal semantics.

#[path = "support/python_conformance.rs"]
mod python_conformance;

#[test]
fn validate_media_reference_packs() {
    python_conformance::run("tests/validate_media_reference_packs.py");
}
