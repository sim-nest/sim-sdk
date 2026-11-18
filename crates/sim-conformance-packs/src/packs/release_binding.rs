// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Executed checker-catalog and identity observations.
//!
//! `binding-catalog` rebuilds every static checker binding and its code
//! id from this crate. The caller supplies the verb `execute`, not the
//! ids. `identity-roundtrip` recomputes one semantic id, checks the
//! golden vector, and verifies a storage id against the datum's canonical
//! bytes. Neither result is a completion claim: the catalog walk does not
//! serialize a support DAG, and the round-trip is one datum, not every
//! reached constructor.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use sim_conformance_core::{IdKind, SemanticId, StorageId};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_kernel::{Datum, NumberLiteral, Symbol};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::{
    CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, all_packs, find_pack,
};

const BINDING_SCOPE: &str = "release/binding-catalog";
const BINDING_FACT: &str = "release.bindings";
const BINDING_RESULT: &str = "binding-catalog";
const IDENTITY_SCOPE: &str = "release/identity-roundtrip";
const IDENTITY_FACT: &str = "release.identity";
const IDENTITY_RESULT: &str = "identity-roundtrip";
const VECTOR: &str = "c13d17c133a40252864f27f34aeadaeb273a83dadd2f9726754d31a2e02d6e68";

/// Receipt produced by rebuilding the static checker catalog.
pub struct ObservedBindingCatalog {
    /// Subject the receipt names.
    pub subject: String,
    /// `binding-catalog`. This is not `completion/claim-25`.
    pub result: String,
    /// Checkers whose bindings were rebuilt, in order.
    pub checkers: Vec<String>,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `release/binding-catalog`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Receipt produced by the semantic and storage round-trip.
pub struct ObservedIdentityRoundtrip {
    /// Subject the receipt names.
    pub subject: String,
    /// `identity-roundtrip`. This is not `completion/claim-5`.
    pub result: String,
    /// Golden vector the semantic id produced.
    pub vector: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `release/identity-roundtrip`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Rebuilds the static bindings and admits `binding-catalog`.
///
/// # Errors
///
/// Returns a refusal when a binding does not rebuild to the same id, or
/// an implemented scope is outside the binding.
pub fn observe_binding_catalog(subject: &str) -> Result<ObservedBindingCatalog, PackFailure> {
    admit_binding(subject, &run(subject, BINDING_SCOPE, BINDING_FACT)?)
}

/// Recomputes the identity round-trip and admits `identity-roundtrip`.
///
/// # Errors
///
/// Returns a refusal when the golden vector or the storage check fails.
pub fn observe_identity_roundtrip(subject: &str) -> Result<ObservedIdentityRoundtrip, PackFailure> {
    admit_identity(subject, &run(subject, IDENTITY_SCOPE, IDENTITY_FACT)?)
}

/// Re-checks a binding receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer `binding-catalog`.
pub fn bind_binding_catalog(observation: &ObservedBindingCatalog) -> Result<(), PackFailure> {
    if observation.result != BINDING_RESULT || observation.scope != scope_id(BINDING_SCOPE)? {
        return Err(fail("claim-relabelled", BINDING_SCOPE));
    }
    bind(
        observation.subject.as_str(),
        observation.result.as_str(),
        &observation.scope,
        &observation.checker,
        &observation.catalog,
        &observation.admitted,
        observation.revocation.clone(),
    )
}

/// Re-checks an identity receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer `identity-roundtrip`.
pub fn bind_identity_roundtrip(observation: &ObservedIdentityRoundtrip) -> Result<(), PackFailure> {
    if observation.result != IDENTITY_RESULT || observation.scope != scope_id(IDENTITY_SCOPE)? {
        return Err(fail("claim-relabelled", IDENTITY_SCOPE));
    }
    bind(
        observation.subject.as_str(),
        observation.result.as_str(),
        &observation.scope,
        &observation.checker,
        &observation.catalog,
        &observation.admitted,
        observation.revocation.clone(),
    )
}

pub(super) fn binding_observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    require_execute(request, BINDING_FACT)?;
    let (encoded, _) = binding_body()?;
    Ok(vec![observation("release.execution", encoded)])
}

pub(super) fn identity_observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    require_execute(request, IDENTITY_FACT)?;
    Ok(vec![observation("release.execution", identity_body()?)])
}

fn admit_binding(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedBindingCatalog, PackFailure> {
    let encoded = execution(observations)?;
    let (fresh, checkers) = binding_body()?;
    if encoded != fresh {
        return Err(fail("evidence-mismatch", "release.execution"));
    }
    let admitted = admit(
        subject,
        BINDING_RESULT,
        BINDING_SCOPE,
        "binding-catalog",
        "binding/catalog",
        encoded,
    )?;
    Ok(ObservedBindingCatalog {
        subject: subject.to_owned(),
        result: BINDING_RESULT.to_owned(),
        checkers,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn admit_identity(
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedIdentityRoundtrip, PackFailure> {
    let encoded = execution(observations)?;
    let fresh = identity_body()?;
    if encoded != fresh || !encoded.contains(VECTOR) {
        return Err(fail("evidence-mismatch", "release.execution"));
    }
    let admitted = admit(
        subject,
        IDENTITY_RESULT,
        IDENTITY_SCOPE,
        "identity-roundtrip",
        "identity/roundtrip",
        encoded,
    )?;
    Ok(ObservedIdentityRoundtrip {
        subject: subject.to_owned(),
        result: IDENTITY_RESULT.to_owned(),
        vector: VECTOR.to_owned(),
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn binding_body() -> Result<(String, Vec<String>), PackFailure> {
    let mut specs = all_packs().to_vec();
    specs.sort_by_key(|spec| spec.checker);
    let mut seen = BTreeSet::new();
    let mut encoded = String::new();
    let mut checkers = Vec::new();
    for spec in specs {
        if !seen.insert(spec.checker) {
            return Err(fail("duplicate-crate", spec.checker));
        }
        let left = spec
            .checker_binding()
            .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
        let right = spec
            .checker_binding()
            .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
        if left.id() != right.id() {
            return Err(fail("evidence-mismatch", spec.checker));
        }
        let allowed: BTreeSet<&str> = spec.allowed_scopes.iter().copied().collect();
        if spec
            .implemented_scopes
            .iter()
            .any(|scope| !allowed.contains(scope))
        {
            return Err(fail("scope-outside-binding", spec.checker));
        }
        let code = spec
            .checker_code_id()
            .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
        receipt::push_field(&mut encoded, spec.checker);
        receipt::push_field(&mut encoded, &receipt::render(left.id().content_id()));
        receipt::push_field(&mut encoded, &receipt::render(code.content_id()));
        receipt::push_field(&mut encoded, spec.build_inputs_sha256());
        checkers.push(spec.checker.to_owned());
    }
    if checkers.is_empty() {
        return Err(fail("missing-source", BINDING_FACT));
    }
    Ok((encoded, checkers))
}

fn identity_body() -> Result<String, PackFailure> {
    let vector = golden_vector()?;
    if vector != VECTOR {
        return Err(fail("golden-vector-mismatch", &vector));
    }
    let datum = Datum::Node {
        tag: Symbol::qualified("test", "store-roundtrip-v1"),
        fields: vec![(Symbol::new("value"), Datum::String("meaning".into()))],
    };
    let semantic = SemanticId::<RoundtripKind>::from_datum(&datum)
        .map_err(|error| fail("identity-construction", &error.to_string()))?;
    let again = SemanticId::<RoundtripKind>::from_datum(&datum)
        .map_err(|error| fail("identity-construction", &error.to_string()))?;
    if semantic != again {
        return Err(fail("semantic-roundtrip-mismatch", "identity"));
    }
    let encoded = datum
        .canonical_bytes()
        .map_err(|error| fail("identity-construction", &error.to_string()))?;
    let storage = StorageId::for_bytes(&encoded);
    storage
        .verify(&encoded)
        .map_err(|error| fail("identity-construction", &error.to_string()))?;
    if storage.verify(b"corrupt").is_ok() {
        return Err(fail("storage-corruption-accepted", "identity"));
    }
    let mut body = String::new();
    receipt::push_field(&mut body, &vector);
    receipt::push_field(
        &mut body,
        &semantic.content_id().algorithm.as_qualified_str(),
    );
    receipt::push_field(
        &mut body,
        &storage.content_id().algorithm.as_qualified_str(),
    );
    receipt::push_field(&mut body, &hex_bytes(Sha256::digest(&encoded).as_slice()));
    Ok(body)
}

fn golden_vector() -> Result<String, PackFailure> {
    let id = SemanticId::<VectorKind>::from_fields(vec![(
        Symbol::qualified("vector", "answer"),
        Datum::Number(NumberLiteral {
            domain: Symbol::qualified("numbers", "u64"),
            canonical: "42".into(),
        }),
    )])
    .map_err(|error| fail("identity-construction", &error.to_string()))?;
    Ok(hex_bytes(&id.content_id().bytes))
}

fn run(subject: &str, scope: &str, fact: &str) -> Result<Vec<CheckObservation>, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(fact, "execute");
    let verdict = super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &receipt::render(binding.id().content_id()),
        subject: &receipt::render(checked.content_id()),
        scope,
        evidence: &evidence,
    });
    match verdict {
        PackVerdict::Pass { observations, .. } => Ok(observations),
        PackVerdict::Refused(failure) => Err(failure),
        PackVerdict::UnimplementedPack { .. } => Err(fail("unimplemented", scope)),
    }
}

fn admit(
    subject: &str,
    result: &str,
    scope: &str,
    proof: &str,
    domain: &str,
    leaf: &str,
) -> Result<super::receipt::ExecutedReceipt, PackFailure> {
    let rows = vec![PackageRow {
        name: proof.to_owned(),
        version: "0".to_owned(),
        url: format!("git://{proof}"),
        rev: hex_bytes(Sha256::digest(leaf.as_bytes()).as_slice()),
    }];
    receipt::admit_executed(AdmitExecuted {
        subject,
        result,
        scope,
        proof,
        domain,
        policy: &format!("policy/{proof}"),
        provenance: &format!("provenance/{proof}"),
        support: &format!("support/{proof}"),
        projection: &format!("projection/{proof}"),
        environment: &format!("environment/{proof}"),
        leaf: leaf.as_bytes(),
        source_text: &receipt::package_source(domain, proof, &rows),
        definitions: &receipt::package_definitions(domain, &rows),
    })
}

fn require_execute(request: &PackRequest<'_>, fact: &str) -> Result<(), PackFailure> {
    let mut body = None;
    for key in request.evidence.fact_keys() {
        match key {
            value if value == fact => body = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    match body {
        Some("execute") => Ok(()),
        Some(value) if value.contains("completion/claim-") => Err(fail("claim-relabelled", fact)),
        _ => Err(fail("missing-source", fact)),
    }
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "release.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-source", "release.execution"))
}

fn bind(
    subject: &str,
    result: &str,
    scope: &CheckScopeId,
    checker: &sim_conformance_v02::CheckerReceiptId,
    catalog: &ProofCatalog,
    admitted: &AdmittedEvidence,
    revocation: RevocationStatus,
) -> Result<(), PackFailure> {
    receipt::bind_executed(
        subject, result, scope, checker, catalog, admitted, revocation,
    )
}

fn scope_id(scope: &str) -> Result<CheckScopeId, PackFailure> {
    CheckScopeId::from_text(scope).map_err(|error| fail("receipt-refused", &error.to_string()))
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn observation(key: &str, value: impl Into<String>) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.into(),
    }
}

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RoundtripKind;
impl IdKind for RoundtripKind {
    const DOMAIN: &'static str = "test/store-roundtrip-v1";
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct VectorKind;
impl IdKind for VectorKind {
    const DOMAIN: &'static str = "test/vector-v1";
}

#[cfg(test)]
#[path = "release_binding_tests.rs"]
mod tests;
