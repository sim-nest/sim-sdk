//! Causal-journal conformance pack.

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Frozen after five isolated 100,000-record samples on the NV12.02 active
/// reference host. This value is replaced only by a later measured contract.
pub const COLD_REPLAY_100K_CEILING_NS: u64 = 4_742_983_535;

/// Checks native compatibility, measured replay, or causal reducer evidence.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-journal", request, |request| {
        match request.scope {
            "journal/native-compatibility" => native_compatibility(request),
            "journal/performance" => performance(request),
            "journal/causal" => causal(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

fn native_compatibility(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut observations = Vec::new();
    for key in [
        "journal.old-only-replay",
        "journal.new-only-replay",
        "journal.mixed-replay",
        "journal.v1-prefix-byte-identical",
        "journal.corrupt-prefix-refused",
        "journal.corrupt-payload-refused",
        "journal.truncated-transition-refused",
        "journal.head-mismatch-refused",
        "journal.stale-fence-refused",
        "journal.all-upgrade-crashes-recover",
        "journal.all-append-crashes-recover",
        "journal.paused-old-writer-cas-loses",
        "journal.old-residue-inert",
        "journal.old-winner-included",
        "journal.competing-upgrades-one-winner",
        "journal.conflicting-new-writers-one-head",
        "journal.progress-after-every-recovery",
        "journal.correspondence-index-rebuilds",
        "journal.object-index-rebuilds",
        "journal.no-v3-schema-reader",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}

fn performance(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let records = u64_fact(request, "journal.performance.records")?;
    let samples = u64_fact(request, "journal.performance.samples")?;
    let maximum = u64_fact(request, "journal.performance.maximum-ns")?;
    let ceiling = u64_fact(request, "journal.performance.ceiling-ns")?;
    if records != 100_000 || samples != 5 {
        return Err(PackFailure {
            code: "performance-sample-contract",
            detail: format!("records={records}, samples={samples}"),
        });
    }
    if ceiling != COLD_REPLAY_100K_CEILING_NS || maximum > ceiling {
        return Err(PackFailure {
            code: "performance-ceiling",
            detail: format!("maximum={maximum}, ceiling={ceiling}"),
        });
    }
    Ok(vec![
        observation("journal.performance.records", records),
        observation("journal.performance.samples", samples),
        observation("journal.performance.maximum-ns", maximum),
        observation("journal.performance.ceiling-ns", ceiling),
        harness::expect_true(request, "journal.performance.isolated-processes")?,
        harness::expect_true(request, "journal.performance.active-reference")?,
    ])
}

fn causal(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut observations = Vec::new();
    for key in [
        "journal.semantic-deltas-cause-only",
        "journal.evidence-set-roots-persistent",
        "journal.snapshots-verified",
        "journal.snapshot-damage-refused",
        "journal.retention-roots-explicit",
        "journal.telemetry-separate",
        "journal.unchanged-10000-no-records",
        "journal.unchanged-10000-no-linear-evidence-copy",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}

fn u64_fact(request: &PackRequest<'_>, key: &'static str) -> Result<u64, PackFailure> {
    request
        .evidence
        .fact(key)
        .ok_or(PackFailure {
            code: "missing-evidence",
            detail: key.into(),
        })?
        .parse()
        .map_err(|_| PackFailure {
            code: "malformed-evidence",
            detail: key.into(),
        })
}

fn observation(key: &str, value: u64) -> CheckObservation {
    CheckObservation {
        key: key.into(),
        value: value.to_string(),
    }
}
