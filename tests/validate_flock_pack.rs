// conformance: flock care preserves unknowns and refuses diagnosis or automated care.

#[path = "support/python_conformance.rs"]
mod python_conformance;

#[test]
fn validate_flock_pack() {
    python_conformance::run("tests/validate_flock_pack.py");
}
