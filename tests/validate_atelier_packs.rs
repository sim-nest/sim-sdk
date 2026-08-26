// conformance: Atelier room packs remain independent, bounded, and networkless.

#[path = "support/python_conformance.rs"]
mod python_conformance;

#[test]
fn validate_atelier_packs() {
    python_conformance::run("tests/validate_atelier_packs.py");
}
