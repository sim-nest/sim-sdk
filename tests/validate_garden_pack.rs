// conformance: garden plans separate observations, guidance, questions, and copied claims.

#[path = "support/python_conformance.rs"]
mod python_conformance;

#[test]
fn validate_garden_pack() {
    python_conformance::run("tests/validate_garden_pack.py");
}
