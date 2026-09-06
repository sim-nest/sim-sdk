//! Canonical identity register and golden-vector checks.

use sim_conformance_core::{IdKind, SemanticId};
use sim_kernel::{Datum, NumberLiteral, Symbol};

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Checks the exact requested identity scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-id", request, |request| match request.scope {
        "identity/register" => register(request),
        "identity/vectors" => vectors(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}

fn register(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut observations = harness::expect_same_u64(
        request,
        "identity.registered-constructions",
        "identity.expected-constructions",
    )?;
    for key in [
        "identity.all-constructions-funded",
        "identity.semantic-digests-256-bit",
        "identity.domain-tags-unique",
        "identity.semantic-storage-separated",
        "identity.ephemeral-authority-absent",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    Ok(observations)
}

fn vectors(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct VectorKind;
    impl IdKind for VectorKind {
        const DOMAIN: &'static str = "test/vector-v1";
    }
    let id = SemanticId::<VectorKind>::from_fields(vec![(
        Symbol::qualified("vector", "answer"),
        Datum::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "u64"),
            canonical: "42".into(),
        }),
    )])
    .map_err(|error| PackFailure {
        code: "identity-construction",
        detail: error.to_string(),
    })?;
    let actual = id
        .content_id()
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let expected = "c13d17c133a40252864f27f34aeadaeb273a83dadd2f9726754d31a2e02d6e68";
    if actual != expected {
        return Err(PackFailure {
            code: "golden-vector-mismatch",
            detail: actual,
        });
    }
    Ok(vec![
        CheckObservation {
            key: "identity.datum-vector".into(),
            value: expected.into(),
        },
        harness::expect_true(request, "identity.cross-toolchain-confirmed")?,
        harness::expect_true(request, "identity.cross-architecture-confirmed")?,
    ])
}
