use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};
use sim_incremental_core::projection::{
    ArchitectureFaultReview, AssayError, AssayOutcome, ControlledDelta, ControlledDeltaClass,
    ExpectedClosure, ExpectedClosureSet, FactId, FederatedClosure, OwnerProjectionGraphOwner,
    PredictedClosureAssay, ProjectionRepairItem, ProjectionRepairTracker, RepairDisposition,
};
use sim_kernel::Datum;
use std::ops::Deref;

use super::{MemorySubject, PackVerdict, packs, request};

const DENOMINATOR: usize = 40;
const ORACLE_BYTES: &[u8] = include_bytes!("../fixtures/nv12_07_expected_closure.tsv");
const ORACLE_SHA256: &str = "3d1d3830a7844c0e00d2335a7e32b66c9c37c4441ee03cf60c45c7a2272543b2";

#[derive(Clone, Copy)]
enum Mutation {
    Declaration(usize),
    Edge(usize, usize),
}

struct OwnedClosure {
    closure: FederatedClosure,
    _owners: Vec<OwnerProjectionGraphOwner>,
}

impl Deref for OwnedClosure {
    type Target = FederatedClosure;

    fn deref(&self) -> &Self::Target {
        &self.closure
    }
}

fn fact(value: &str) -> FactId {
    FactId::new(value).unwrap()
}

fn conclusion(index: usize) -> sim_incremental_core::projection::ConclusionId {
    sim_incremental_core::projection::ConclusionId::new(format!("conclusion/{index:02}")).unwrap()
}

fn scenario_fact(index: usize) -> Option<FactId> {
    match index {
        1..=2 => Some(fact("change/private-implementation")),
        3..=10 => Some(fact("change/public-api")),
        11..=13 => Some(fact("change/head-a")),
        14..=16 => Some(fact("change/head-b")),
        17..=19 => Some(fact("change/head-c")),
        20..=22 => Some(fact("change/control-pins")),
        23..=24 => Some(fact("change/proof-code-alpha")),
        25..=26 => Some(fact("change/checker-alpha")),
        _ => None,
    }
}

fn changed_fact_candidates() -> Vec<FactId> {
    [
        "change/private-implementation",
        "change/public-api",
        "change/head-a",
        "change/head-b",
        "change/head-c",
        "change/control-pins",
        "change/proof-code-alpha",
        "change/checker-alpha",
    ]
    .map(fact)
    .to_vec()
}

fn closure(mutation: Option<Mutation>) -> OwnedClosure {
    let mut universe = changed_fact_candidates()
        .into_iter()
        .collect::<BTreeSet<_>>();
    universe.insert(fact("change/dormant"));
    let mut owners = BTreeMap::<String, Vec<_>>::new();
    for index in 1..=DENOMINATOR {
        let base = fact(&format!("stable/{index:02}"));
        universe.insert(base.clone());
        let mut dependencies = BTreeSet::from([base]);
        if let Some(changed) = scenario_fact(index) {
            dependencies.insert(changed);
        }
        match mutation {
            Some(Mutation::Declaration(target)) if target == index => {
                if scenario_fact(index).is_some() {
                    continue;
                }
                dependencies.insert(fact("change/private-implementation"));
            }
            Some(Mutation::Edge(target, edge)) if target == index => {
                let original = dependencies.iter().nth(edge).unwrap().clone();
                dependencies.remove(&original);
                if original.as_str().starts_with("change/") {
                    dependencies.insert(fact("change/dormant"));
                } else {
                    let replacement = changed_fact_candidates()
                        .into_iter()
                        .find(|candidate| !dependencies.contains(candidate))
                        .unwrap();
                    dependencies.insert(replacement);
                }
            }
            _ => {}
        }
        owners
            .entry(format!("owner/{:02}", (index - 1) / 10 + 1))
            .or_default()
            .push((conclusion(index), dependencies));
    }
    let owners = owners
        .into_iter()
        .map(|(owner, rows)| OwnerProjectionGraphOwner::new(owner, rows).unwrap())
        .collect::<Vec<_>>();
    let closure = FederatedClosure::seal(
        universe,
        owners.iter().map(OwnerProjectionGraphOwner::retain),
    )
    .unwrap();
    OwnedClosure {
        closure,
        _owners: owners,
    }
}

fn delta_class(value: &str) -> ControlledDeltaClass {
    match value {
        "no-semantic-change" => ControlledDeltaClass::NoSemanticChange,
        "private-implementation" => ControlledDeltaClass::PrivateImplementation,
        "public-api" => ControlledDeltaClass::PublicApi,
        "three-public-heads-and-control-pins" => ControlledDeltaClass::PublicHeadsAndPins,
        "proof-and-checker-implementation" => ControlledDeltaClass::ProofAndCheckerImplementation,
        "plan-display-order-semantic-no-op" => ControlledDeltaClass::PresentationOnly,
        other => panic!("unknown oracle delta class {other}"),
    }
}

fn oracle() -> ExpectedClosureSet {
    let digest = Sha256::digest(ORACLE_BYTES);
    let rendered = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(rendered, ORACLE_SHA256);
    let text = std::str::from_utf8(ORACLE_BYTES).unwrap();
    let rows = text
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("delta\t"))
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 7);
            let _class = delta_class(fields[1]);
            let affected = if fields[6] == "-" {
                BTreeSet::new()
            } else {
                fields[6]
                    .split(',')
                    .map(|value| {
                        sim_incremental_core::projection::ConclusionId::new(value).unwrap()
                    })
                    .collect()
            };
            ExpectedClosure {
                delta: fields[0].into(),
                denominator: fields[2].parse().unwrap(),
                max_affected: fields[3].parse().unwrap(),
                max_revision_delta: fields[4].parse().unwrap(),
                carried_state: fields[5].parse().unwrap(),
                affected,
            }
        })
        .collect::<Vec<_>>();
    let source = Datum::Bytes(ORACLE_BYTES.to_vec()).content_id().unwrap();
    ExpectedClosureSet::freeze(source, rows).unwrap()
}

fn deltas() -> Vec<ControlledDelta> {
    vec![
        ControlledDelta::new("D1", ControlledDeltaClass::NoSemanticChange, [], 0, false).unwrap(),
        ControlledDelta::new(
            "D2",
            ControlledDeltaClass::PrivateImplementation,
            [fact("change/private-implementation")],
            1,
            false,
        )
        .unwrap(),
        ControlledDelta::new(
            "D3",
            ControlledDeltaClass::PublicApi,
            [fact("change/public-api")],
            1,
            false,
        )
        .unwrap(),
        ControlledDelta::new(
            "D4",
            ControlledDeltaClass::PublicHeadsAndPins,
            [
                fact("change/head-a"),
                fact("change/head-b"),
                fact("change/head-c"),
                fact("change/control-pins"),
            ],
            4,
            false,
        )
        .unwrap(),
        ControlledDelta::new(
            "D5",
            ControlledDeltaClass::ProofAndCheckerImplementation,
            [
                fact("change/proof-code-alpha"),
                fact("change/checker-alpha"),
            ],
            2,
            false,
        )
        .unwrap(),
        ControlledDelta::new("D6", ControlledDeltaClass::PresentationOnly, [], 1, false).unwrap(),
    ]
}

#[test]
fn frozen_oracle_drives_the_exact_six_delta_assay() {
    let closure = closure(None);
    let qualification = PredictedClosureAssay::new(&closure)
        .qualify(&deltas(), &oracle())
        .unwrap();
    let reports = &qualification.reports;
    assert_eq!(reports.len(), 6);
    assert_eq!(reports[0].unaffected_basis_points, 10_000);
    assert_eq!(reports[1].unaffected_basis_points, 9_500);
    assert_eq!(reports[2].affected, (3..=10).map(conclusion).collect());
    assert_eq!(reports[3].unaffected_basis_points, 7_000);
    assert_eq!(reports[4].affected, (23..=26).map(conclusion).collect());
    assert_eq!(
        (
            reports[0].work.planned_starts,
            reports[0].work.journal_records
        ),
        (0, 0)
    );
    assert_eq!(
        (
            reports[5].work.planned_starts,
            reports[5].work.journal_records
        ),
        (0, 0)
    );
    assert!(!reports[5].carried_state);
    assert!(
        reports
            .iter()
            .all(|report| report.denominator == DENOMINATOR)
    );
}

#[test]
fn every_declaration_and_edge_mutant_prevents_qualification() {
    let oracle = oracle();
    let deltas = deltas();
    for declaration in 1..=DENOMINATOR {
        assert!(matches!(
            PredictedClosureAssay::new(&closure(Some(Mutation::Declaration(declaration))))
                .qualify(&deltas, &oracle),
            Err(AssayError::RepairRequired(_))
        ));
        let edge_count = usize::from(scenario_fact(declaration).is_some()) + 1;
        for edge in 0..edge_count {
            assert!(matches!(
                PredictedClosureAssay::new(&closure(Some(Mutation::Edge(declaration, edge))))
                    .qualify(&deltas, &oracle),
                Err(AssayError::RepairRequired(_))
            ));
        }
    }
}

#[test]
fn repair_is_bounded_and_third_unchanged_epoch_stays_at_need_direction() {
    let mutant = closure(Some(Mutation::Declaration(1)));
    let AssayOutcome::Repair { repairs, .. } = PredictedClosureAssay::new(&mutant)
        .predict(&deltas()[1], &oracle())
        .unwrap()
    else {
        panic!("mutant unexpectedly qualified")
    };
    assert!(
        repairs
            .items
            .contains(&ProjectionRepairItem::Declaration(conclusion(1)))
    );
    let mut tracker = ProjectionRepairTracker::default();
    assert_eq!(tracker.record(&repairs), RepairDisposition::RepairEpoch(1));
    assert_eq!(tracker.record(&repairs), RepairDisposition::RepairEpoch(2));
    for _ in 0..2 {
        assert!(matches!(
            tracker.record(&repairs),
            RepairDisposition::NeedDirection(ArchitectureFaultReview { epochs: 3, .. })
        ));
    }
}

#[test]
fn predicted_closure_checker_is_scope_exact_and_fail_closed() {
    let evidence = packs::evidence::PREDICTED_CLOSURE_FACTS
        .into_iter()
        .fold(MemorySubject::default(), |subject, key| {
            subject.with(key, "true")
        });
    assert!(matches!(
        packs::evidence::check(&request(
            "checker/c-evidence",
            "evidence/predicted-closure",
            &evidence,
        )),
        PackVerdict::Pass { .. }
    ));
    assert!(matches!(
        packs::evidence::check(&request(
            "checker/c-evidence",
            "evidence/predicted-closure",
            &evidence.with("evidence.proof-not-executed", "false"),
        )),
        PackVerdict::Refused(_)
    ));
}

#[test]
fn nv12_07_release_scope_requires_the_complete_release_gate() {
    let release = [
        "release.owner-validation-passed",
        "release.owner-docs-passed",
        "release.packages-assembled",
        "release.audit-passed",
        "release.authorship-passed",
        "release.tags-exact",
        "release.pins-exact",
        "release.mirrors-current",
        "release.generated-converged",
        "release.boot-smoke-passed",
        "release.publication-confirmed",
        "release.standalone-ci-green",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    });
    assert!(matches!(
        packs::release::check(&request("checker/c-release", "release/nv12-07", &release,)),
        PackVerdict::Pass { .. }
    ));
    assert!(matches!(
        packs::release::check(&request(
            "checker/c-release",
            "release/nv12-07",
            &release.with("release.packages-assembled", "false"),
        )),
        PackVerdict::Refused(_)
    ));
}
