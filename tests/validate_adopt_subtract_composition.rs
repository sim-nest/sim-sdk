// conformance: stewardship adoption rebuilds without ephemeral state and subtracts retired parts.

#[path = "support/python_conformance.rs"]
mod python_conformance;

#[test]
fn validate_adopt_subtract_composition() {
    python_conformance::run("tests/validate_adopt_subtract_composition.py");
}
