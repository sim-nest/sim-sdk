use std::collections::{BTreeMap, BTreeSet};

const MATRIX: &str = include_str!("../recipes/stewardship/memo-acceptance/matrix.tsv");
const SCENARIO: &str = include_str!("../recipes/stewardship/memo-acceptance/scenario.tsv");
const STATEMENT: &str = include_str!("../recipes/stewardship/memo-acceptance/statement.csv");
const ODB: &str = include_str!("../recipes/stewardship/memo-acceptance/odb.script");
const README: &str = include_str!("../recipes/stewardship/memo-acceptance/README.md");

fn rows(input: &str, columns: usize) -> Vec<Vec<&str>> {
    input
        .lines()
        .skip(1)
        .map(|line| {
            let row: Vec<_> = line.split('\t').collect();
            assert_eq!(row.len(), columns, "malformed row: {line}");
            assert!(
                row.iter().all(|value| !value.trim().is_empty()),
                "empty cell: {line}"
            );
            row
        })
        .collect()
}

#[test]
fn every_memo_promise_is_classified_and_has_two_sided_proof() {
    let matrix = rows(MATRIX, 6);
    let mut counts = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for row in &matrix {
        assert!(ids.insert(row[0]), "duplicate promise id {}", row[0]);
        assert!(["standard", "ai", "support-crew", "vibe"].contains(&row[1]));
        assert!(["specimen", "unsupported", "manual"].contains(&row[2]));
        assert!(row[4].len() > 8 && row[5].len() > 8);
        *counts.entry(row[1]).or_insert(0usize) += 1;
    }
    assert_eq!(
        counts.keys().copied().collect::<Vec<_>>(),
        ["ai", "standard", "support-crew", "vibe"]
    );
    assert!(counts.values().all(|count| *count >= 4));
    assert!(matrix.len() >= 36, "substantive matrix unexpectedly shrank");
}

#[test]
fn synthetic_product_runs_every_required_trace() {
    let events = rows(SCENARIO, 5);
    let kinds: BTreeSet<_> = events.iter().map(|row| row[1]).collect();
    for required in [
        "month.sources",
        "month.certificate",
        "week.kitchen",
        "season.garden",
        "day.flock",
        "log.optional",
        "media.chosen",
        "dialogue.fake-model",
        "dialogue.refusals",
        "dialogue.handoff",
        "workshop.brief",
        "workshop.adopt",
        "workshop.subtract",
        "crew.day",
        "crew.refusal",
        "crew.handoff",
        "lifecycle.stop",
        "lifecycle.offline",
        "lifecycle.backup-restore",
        "lifecycle.rotation",
        "lifecycle.revocation",
        "lifecycle.crypto-erasure",
        "lifecycle.export",
        "lifecycle.uninstall",
        "lifecycle.manual",
    ] {
        assert!(kinds.contains(required), "missing trace {required}");
    }
    assert!(
        events
            .iter()
            .filter(|row| row[2] == "human")
            .all(|row| ["accepted", "declined", "passed"].contains(&row[3]))
    );
    assert!(
        events
            .iter()
            .filter(|row| row[2] == "fake-model" || row[2] == "crew")
            .all(|row| ["prepared", "refused"].contains(&row[3]))
    );
}

#[test]
fn real_format_sources_are_exact_and_read_only_inputs() {
    let amounts: Vec<i64> = STATEMENT
        .lines()
        .skip(1)
        .map(|line| {
            let amount = line.split(';').nth(4).expect("statement amount");
            let (whole, cents) = amount.split_once('.').expect("two-decimal amount");
            whole.parse::<i64>().unwrap() * 100
                + if whole.starts_with('-') {
                    -cents.parse::<i64>().unwrap()
                } else {
                    cents.parse::<i64>().unwrap()
                }
        })
        .collect();
    assert_eq!(amounts, [-43_720, -120_000, 1_235]);
    assert!(ODB.contains("DECIMAL(14,2)"));
    assert!(ODB.contains("-437.20") && ODB.contains("437.20"));
    assert_eq!(ODB.matches("INSERT INTO TRANS").count(), 2);
    assert!(SCENARIO.contains("eight synthetic bird ids"));
}

#[test]
fn crew_prepares_but_never_acts_and_human_decides() {
    let events = rows(SCENARIO, 5);
    assert!(
        events
            .iter()
            .filter(|row| row[2] == "crew")
            .all(|row| row[3] == "prepared" || row[3] == "refused")
    );
    assert!(events.iter().any(|row| row[1] == "crew.refusal"
        && row[4].contains("no post pay diagnose order share or delete effect")));
    for consequential in [
        "month.sources",
        "week.kitchen",
        "season.garden",
        "day.flock",
        "workshop.adopt",
        "workshop.subtract",
        "lifecycle.rotation",
        "lifecycle.crypto-erasure",
        "lifecycle.export",
        "lifecycle.uninstall",
    ] {
        assert!(
            events
                .iter()
                .any(|row| row[1] == consequential && row[2] == "human"),
            "{consequential} lacks human decision"
        );
    }
}

#[test]
fn public_evidence_is_synthetic_and_destruction_claims_are_bounded() {
    let corpus = [MATRIX, SCENARIO, STATEMENT, ODB, README].join("\n");
    for forbidden in [
        "kb-2022",
        "personnummer",
        "account number",
        "Mia's",
        "Mias ",
        "Mias\t",
    ] {
        assert!(
            !corpus.to_lowercase().contains(&forbidden.to_lowercase()),
            "private-value marker found: {forbidden}"
        );
    }
    for overclaim in [
        "all copies destroyed",
        "irrecoverable everywhere",
        "guaranteed deletion",
        "securely wiped",
    ] {
        assert!(
            !corpus.to_lowercase().contains(overclaim),
            "overbroad destruction claim: {overclaim}"
        );
    }
    assert!(README.contains("tracked\nciphertext generations and backups"));
    assert!(MATRIX.contains("claim is limited to managed copies"));
}
// conformance: memo acceptance proves deterministic multi-format evidence review.
