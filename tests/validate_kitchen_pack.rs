// conformance: kitchen plans preserve units, freshness, and source-room isolation.

#[path = "support/python_conformance.rs"]
mod python_conformance;

#[test]
fn validate_kitchen_pack() {
    python_conformance::run("tests/validate_kitchen_pack.py");
}
