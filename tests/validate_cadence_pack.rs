// conformance: stewardship cadence preserves source isolation and interaction ceilings.

#[path = "support/python_conformance.rs"]
mod python_conformance;

#[test]
fn validate_cadence_pack() {
    python_conformance::run("tests/validate_cadence_pack.py");
}
