//! Canonical identity register and golden-vector checks.

use sim_conformance_core::{IdKind, SemanticId, StorageId};
use sim_kernel::{Datum, NumberLiteral, Symbol};

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Checks the exact requested identity scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-id", request, |request| match request.scope {
        "identity/register" => register(request),
        "identity/vectors" => vectors(request),
        "identity/journal-normalized" => journal_normalized(request),
        "identity/closure-final" => closure_final(request),
        "identity/store-roundtrip" => store_roundtrip(request),
        _ => unreachable!("availability was checked before dispatch"),
    })
}

fn closure_final(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut observations = harness::expect_same_u64(
        request,
        "identity.reached-semantic-sites",
        "identity.normalized-semantic-sites",
    )?;
    for key in [
        "identity.unresolved-reached-zero",
        "identity.unique-versioned-domain-tags",
        "identity.all-preimage-fields-covered",
        "identity.order-insensitive-sets-stable",
        "identity.ordered-fields-sensitive",
        "identity.semantic-fields-sensitive",
        "identity.old-authority-refused",
        "identity.debug-authority-absent",
        "identity.pointer-authority-absent",
        "identity.default-hasher-authority-absent",
        "identity.value-fingerprint-authority-absent",
        "identity.forge-fabricated-authority-refused",
        "identity.loader-consumers-normalized",
        "identity.tooling-delegates-codec",
        "identity.dependent-caches-invalidated",
        "identity.compatibility-read-only",
        "identity.new-families-classified",
        "identity.semantic-byte-roles-distinct",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    observations.push(harness::expect_eq(
        request,
        "identity.semantic-algorithm",
        "core/sha256-datum-v1",
    )?);
    observations.push(harness::expect_eq(
        request,
        "identity.semantic-digest-bytes",
        "32",
    )?);
    Ok(observations)
}

fn store_roundtrip(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct RoundtripKind;
    impl IdKind for RoundtripKind {
        const DOMAIN: &'static str = "test/store-roundtrip-v1";
    }

    let datum = Datum::Node {
        tag: Symbol::qualified("test", "store-roundtrip-v1"),
        fields: vec![(Symbol::new("value"), Datum::String("meaning".into()))],
    };
    let semantic = SemanticId::<RoundtripKind>::from_datum(&datum).map_err(identity_failure)?;
    let decoded = SemanticId::<RoundtripKind>::from_datum(&datum).map_err(identity_failure)?;
    if semantic != decoded {
        return Err(PackFailure {
            code: "semantic-roundtrip-mismatch",
            detail: "recomputed semantic identity changed".into(),
        });
    }
    let encoded = datum.canonical_bytes().map_err(|error| PackFailure {
        code: "identity-construction",
        detail: error.to_string(),
    })?;
    let storage = StorageId::for_bytes(&encoded);
    storage.verify(&encoded).map_err(identity_failure)?;
    if storage.verify(b"corrupt").is_ok() {
        return Err(PackFailure {
            code: "storage-corruption-accepted",
            detail: "byte identity accepted different bytes".into(),
        });
    }

    let mut observations = Vec::new();
    for key in [
        "identity.object-put-get-roundtrip",
        "identity.missing-object-refused",
        "identity.corrupt-object-refused",
        "identity.semantic-mismatch-refused",
        "identity.semantic-storage-crossing-mutant-refused",
        "identity.byte-helpers-remain-byte-role",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    observations.extend([
        CheckObservation {
            key: "identity.roundtrip-semantic-algorithm".into(),
            value: semantic.content_id().algorithm.as_qualified_str(),
        },
        CheckObservation {
            key: "identity.roundtrip-semantic-width".into(),
            value: semantic.content_id().bytes.len().to_string(),
        },
        CheckObservation {
            key: "identity.roundtrip-storage-algorithm".into(),
            value: storage.content_id().algorithm.as_qualified_str(),
        },
        CheckObservation {
            key: "identity.roundtrip-storage-width".into(),
            value: storage.content_id().bytes.len().to_string(),
        },
    ]);
    Ok(observations)
}

fn identity_failure(error: sim_conformance_core::ConformanceError) -> PackFailure {
    PackFailure {
        code: "identity-construction",
        detail: error.to_string(),
    }
}

fn journal_normalized(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    let mut observations = Vec::new();
    for key in [
        "identity.journal-entry-is-datum",
        "identity.journal-payload-is-datum",
        "identity.journal-head-is-semantic",
        "identity.storage-id-is-separated",
        "identity.v1-ids-bounded-to-reader",
        "identity.consumers-see-canonical-only",
    ] {
        observations.push(harness::expect_true(request, key)?);
    }
    observations.push(harness::expect_eq(
        request,
        "identity.journal-algorithm",
        "core/sha256-datum-v1",
    )?);
    Ok(observations)
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
