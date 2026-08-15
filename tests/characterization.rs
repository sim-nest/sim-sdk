#![cfg(feature = "standard-core")]

use sim::{
    characterization::{
        BoundedLane, CanonicalFailure, CanonicalObservation, CanonicalOutcome,
        CaptureComparisonProjection, CharacterizationCapture, FailureLocation, ScenarioLimits,
        ScenarioObservationLane, ScenarioSpec, compare_characterization_captures,
    },
    kernel::{Datum, Symbol},
};

fn contract() -> ScenarioSpec {
    ScenarioSpec::new(
        Symbol::qualified("example", "parser-contract"),
        Symbol::qualified("example", "parser-setup/v1"),
    )
    .with_limits(ScenarioLimits::new(1, 1))
    .observing(ScenarioObservationLane::ValueOrFailure)
}

fn captured(detail: &str, start: usize) -> CharacterizationCapture {
    CharacterizationCapture::new(
        Symbol::qualified("example", "stable-fields/v1"),
        CanonicalObservation {
            outcome: Some(CanonicalOutcome::Failure(CanonicalFailure {
                class: Symbol::qualified("example", "parse-error"),
                detail: Datum::String(detail.to_owned()),
                location: Some(FailureLocation {
                    source: Symbol::qualified("fixture", "invalid-input"),
                    start,
                    end: start + 1,
                }),
            })),
            events: BoundedLane::Absent,
            receipts: BoundedLane::Absent,
            browse: BoundedLane::Absent,
        },
    )
}

#[test]
fn downstream_migration_compares_only_public_contracts() {
    let scenario = contract();
    let projection =
        CaptureComparisonProjection::new(Symbol::qualified("example", "stable-fields/v1"));
    let before = captured("unexpected-token", 4);

    let unchanged = compare_characterization_captures(
        &scenario,
        &before,
        &scenario,
        &captured("unexpected-token", 4),
        &projection,
    )
    .expect("the public contract is comparable");
    assert!(unchanged.is_same());

    let changed = compare_characterization_captures(
        &scenario,
        &before,
        &scenario,
        &captured("unexpected-token", 9),
        &projection,
    )
    .expect("a mismatch is returned as data");
    assert!(!changed.is_same());
    assert!(
        changed
            .differences
            .iter()
            .any(|difference| difference.path.ends_with(".location.start"))
    );
    assert!(changed.differences.iter().all(|difference| {
        difference.left != difference.right && difference.path.starts_with('$')
    }));
}
