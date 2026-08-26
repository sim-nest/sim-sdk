// conformance: private observations preserve consent, missingness, erasure, and export review.

#[path = "support/python_conformance.rs"]
mod python_conformance;

#[test]
fn validate_private_observation_pack() {
    python_conformance::run("tests/validate_private_observation_pack.py");
}
