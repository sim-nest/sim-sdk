// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Claim 4 for the private neutral-input slot.
//!
//! The owner commit holds the selected source and the slot at fixed paths.
//! Admission goes through the neutral-input owner and the `codec/lisp`
//! adapter. The trusted commission is fixed here. A commission read out of
//! the slot is not that authority. The slot bytes stay in the owner commit.
//! The result is `completion/claim-4` only when the recorded digests, the
//! canonical codec, the source, and the commission agree, and the slot
//! declares no runtime consumption.

use std::cell::RefCell;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use sim_codec::{Input, decode_datum_with_codec, encode_datum_with_codec};
use sim_codec_lisp::LispCodecLib;
use sim_conformance_neutral::{
    NeutralInputGap, NeutralQualificationInput, SlotCodec, TrustedCommission,
};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_kernel::{
    Cx, Datum, DefaultFactory, EagerPolicy, EncodeOptions, HandleSeed, ReadPolicy, Symbol,
};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use super::selection::{self, SelectedSource};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/neutral-input";
const DOMAIN: &str = "completion/neutral-input";
const FACT: &str = "completion.neutral-input";
const RESULT: &str = "completion/claim-4";
const PROOF: &str = "neutral-input";
const SOURCE_PATH: &str = "docs/workbench/NO_V3_18.md";
const HANDOFF_PATH: &str = "docs/workbench/NV12_FINISH_1.md";
const SLOT_PATH: &str = "docs/private/no-v3/nv18/neutral-input.sx";
const SOURCE_DIGEST: &str = "4c0b9ad283be893cf4ccbe87e49bf2adc3e206a923c78b05e1dcaaf554fb0052";
const SLOT_DIGEST: &str = "d8a45f8656979dcd1589bcbc91f3230e69d43677e205f79a522fc1599784c2fb";
const CONSUMPTION: &str = "no-runtime-consumption";

/// Receipt admitted after the owner slot verifies.
pub struct ObservedNeutralInput {
    /// Subject the receipt names.
    pub subject: String,
    /// `completion/claim-4`.
    pub result: String,
    /// SHA-256 of the authorized source blob.
    pub source_digest: String,
    /// SHA-256 of the authorized slot blob.
    pub slot_digest: String,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/neutral-input`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Reads the owner slot and admits claim 4.
///
/// # Errors
///
/// Returns a refusal when the paths are not the owner records, git cannot
/// show those blobs, a recorded digest differs, the codec refuses the
/// bytes, or the neutral-input owner refuses the slot.
pub fn observe_neutral_input(
    subject: &str,
    source: &SelectedSource,
) -> Result<ObservedNeutralInput, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(FACT, encode_coordinates(subject, source)?);
    let verdict = super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &receipt::render(binding.id().content_id()),
        subject: &receipt::render(checked.content_id()),
        scope: SCOPE,
        evidence: &evidence,
    });
    let observations = match verdict {
        PackVerdict::Pass { observations, .. } => observations,
        PackVerdict::Refused(failure) => return Err(failure),
        PackVerdict::UnimplementedPack { .. } => return Err(fail("unimplemented", SCOPE)),
    };
    admit_rows(subject, source, &observations)
}

/// Re-checks a claim-4 receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope is no longer claim 4.
pub fn bind_neutral_input(observation: &ObservedNeutralInput) -> Result<(), PackFailure> {
    if observation.result != RESULT || observation.scope != scope_id()? {
        return Err(fail("claim-relabelled", SCOPE));
    }
    receipt::bind_executed(
        &observation.subject,
        &observation.result,
        &observation.scope,
        &observation.checker,
        &observation.catalog,
        &observation.admitted,
        observation.revocation.clone(),
    )
}

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let body = one_fact(request)?;
    if body.contains("completion/claim-") {
        return Err(fail("claim-relabelled", FACT));
    }
    let (subject, selected) = decode_coordinates(body)?;
    let bound =
        CheckedSubjectId::from_text(&subject).map_err(|_| fail("subject-mismatch", FACT))?;
    if receipt::render(bound.content_id()) != request.subject {
        return Err(fail("subject-mismatch", FACT));
    }
    let rev = selection::verify_commit(&selected)?;
    let verified = verify_slot(&selected, &rev)?;
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, &rev);
    receipt::push_field(&mut encoded, &verified.source_digest);
    receipt::push_field(&mut encoded, &verified.slot_digest);
    receipt::push_field(&mut encoded, CONSUMPTION);
    Ok(vec![observation("completion.execution", encoded)])
}

struct VerifiedSlot {
    source_digest: String,
    slot_digest: String,
}

fn verify_slot(selected: &SelectedSource, rev: &str) -> Result<VerifiedSlot, PackFailure> {
    if selected.source != SOURCE_PATH {
        return Err(fail("bad-record", "source"));
    }
    if selected.selection != HANDOFF_PATH {
        return Err(fail("bad-record", "human-selection"));
    }
    let source = selection::git_show(&selected.repository, rev, SOURCE_PATH)?;
    let handoff = selection::git_show(&selected.repository, rev, HANDOFF_PATH)?;
    let slot = selection::git_show(&selected.repository, rev, SLOT_PATH)?;
    let source_digest = sha256_hex(source.as_bytes());
    if source_digest != SOURCE_DIGEST || !handoff.contains(SOURCE_DIGEST) {
        return Err(fail("bad-record", "source-digest"));
    }
    let slot_digest = sha256_hex(slot.as_bytes());
    if slot_digest != SLOT_DIGEST || !handoff.contains(SLOT_DIGEST) {
        return Err(fail("bad-record", "slot-digest"));
    }
    admit_slot(slot.as_bytes(), source.as_bytes())?;
    Ok(VerifiedSlot {
        source_digest,
        slot_digest,
    })
}

fn admit_slot(slot: &[u8], source: &[u8]) -> Result<(), PackFailure> {
    let codec = LispSlotCodec::boot().map_err(|_| fail("codec", "lisp"))?;
    let trusted = commission();
    let admitted = NeutralQualificationInput::admit(slot, &codec, source, &trusted)
        .map_err(|error| fail("neutral-input", gap_name(error.gap)))?;
    if admitted.derivation.producer != trusted.producer
        || admitted.derivation.authority != trusted.authority
        || admitted.derivation.review != trusted.review
        || !admitted.derivation.no_runtime_consumption
    {
        return Err(fail("neutral-input", "commission"));
    }
    Ok(())
}

fn commission() -> TrustedCommission {
    TrustedCommission {
        producer: Symbol::qualified("authoring", "commissioned-neutral-input-author-nv18"),
        authority: Symbol::qualified("authoring", "no-v3-18-maintainer-selected-contract"),
        review: Symbol::qualified("authoring", "nv18-neutral-input-independent-review"),
    }
}

fn gap_name(gap: NeutralInputGap) -> &'static str {
    match gap {
        NeutralInputGap::Malformed => "malformed",
        NeutralInputGap::MissingField => "missing-field",
        NeutralInputGap::ExtraField => "extra-field",
        NeutralInputGap::UnrecognizedTag => "unrecognized-tag",
        NeutralInputGap::ForbiddenText => "forbidden-text",
        NeutralInputGap::UnsupportedVersion => "unsupported-version",
        NeutralInputGap::Oversize => "oversize",
        NeutralInputGap::Arithmetic => "arithmetic",
        NeutralInputGap::Identity => "identity",
        NeutralInputGap::SourceMismatch => "source-mismatch",
        NeutralInputGap::FactMismatch => "fact-mismatch",
    }
}

struct LispSlotCodec {
    cx: RefCell<Cx>,
    symbol: Symbol,
}

impl LispSlotCodec {
    fn boot() -> Result<Self, String> {
        let mut cx = Cx::new(
            Arc::new(EagerPolicy),
            Arc::new(DefaultFactory),
            HandleSeed::new(1),
        );
        register_core_classes(&mut cx)?;
        let lib =
            LispCodecLib::new(cx.registry_mut().fresh_codec_id()).map_err(|e| e.to_string())?;
        cx.load_lib(&lib).map_err(|e| e.to_string())?;
        Ok(Self {
            cx: RefCell::new(cx),
            symbol: Symbol::qualified("codec", "lisp"),
        })
    }
}

impl SlotCodec for LispSlotCodec {
    fn decode(&self, bytes: &[u8]) -> Result<Datum, String> {
        decode_datum_with_codec(
            &mut self.cx.borrow_mut(),
            &self.symbol,
            Input::Bytes(bytes.to_vec()),
            ReadPolicy::default(),
        )
        .map_err(|error| error.to_string())
    }

    fn encode(&self, datum: &Datum) -> Result<Vec<u8>, String> {
        encode_datum_with_codec(
            &mut self.cx.borrow_mut(),
            &self.symbol,
            datum,
            EncodeOptions::default(),
        )
        .and_then(|output| output.into_text())
        .map(String::into_bytes)
        .map_err(|error| error.to_string())
    }
}

fn register_core_classes(cx: &mut Cx) -> Result<(), String> {
    const CLASSES: &[(sim_kernel::ClassId, &str)] = &[
        (sim_kernel::CORE_CLASS_CLASS_ID, "Class"),
        (sim_kernel::CORE_CODEC_CLASS_ID, "Codec"),
        (sim_kernel::CORE_NUMBER_CLASS_ID, "Number"),
        (sim_kernel::CORE_SYMBOL_CLASS_ID, "Symbol"),
        (sim_kernel::CORE_STRING_CLASS_ID, "String"),
        (sim_kernel::CORE_EXPR_CLASS_ID, "Expr"),
        (sim_kernel::CORE_SHAPE_CLASS_ID, "Shape"),
        (sim_kernel::CORE_BOOL_CLASS_ID, "Bool"),
        (sim_kernel::CORE_LIST_CLASS_ID, "List"),
        (sim_kernel::CORE_BYTES_CLASS_ID, "Bytes"),
        (sim_kernel::CORE_TABLE_CLASS_ID, "Table"),
        (sim_kernel::CORE_FUNCTION_CLASS_ID, "Function"),
        (sim_kernel::CORE_CARD_CLASS_ID, "Card"),
        (sim_kernel::CORE_NUMBER_DOMAIN_CLASS_ID, "NumberDomain"),
    ];
    for (id, name) in CLASSES {
        let symbol = Symbol::qualified("core", *name);
        let value = cx
            .factory()
            .class_stub(*id, symbol.clone())
            .map_err(|error| error.to_string())?;
        cx.registry_mut()
            .register_class_value(symbol, value)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn admit_rows(
    subject: &str,
    source: &SelectedSource,
    observations: &[CheckObservation],
) -> Result<ObservedNeutralInput, PackFailure> {
    let encoded = execution(observations)?;
    let decoded = decode_execution(encoded)?;
    let rev = selection::verify_commit(source)?;
    let again = verify_slot(source, &rev)?;
    if decoded.rev != rev
        || decoded.source_digest != again.source_digest
        || decoded.slot_digest != again.slot_digest
        || decoded.source_digest != SOURCE_DIGEST
        || decoded.slot_digest != SLOT_DIGEST
        || decoded.consumption != CONSUMPTION
        || !is_rev(&decoded.rev)
    {
        return Err(fail("evidence-mismatch", "completion.execution"));
    }
    let rows = vec![PackageRow {
        name: PROOF.to_owned(),
        version: "0".to_owned(),
        url: format!("git://{PROOF}"),
        rev: decoded.rev,
    }];
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: RESULT,
        scope: SCOPE,
        proof: PROOF,
        domain: DOMAIN,
        policy: "policy/neutral-input",
        provenance: "provenance/neutral-input",
        support: "support/neutral-input",
        projection: "projection/neutral-input",
        environment: "environment/neutral-input",
        leaf: encoded.as_bytes(),
        source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
        definitions: &receipt::package_definitions(DOMAIN, &rows),
    })?;
    Ok(ObservedNeutralInput {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        source_digest: decoded.source_digest,
        slot_digest: decoded.slot_digest,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn encode_coordinates(subject: &str, source: &SelectedSource) -> Result<String, PackFailure> {
    if subject.is_empty()
        || subject.contains('\0')
        || subject.contains('\n')
        || subject.contains("completion/claim-")
    {
        return Err(fail("bad-record", "subject"));
    }
    let repo = source
        .repository
        .to_str()
        .ok_or_else(|| fail("bad-record", "repository"))?;
    if repo.is_empty() || repo.contains('\0') || repo.contains('\n') {
        return Err(fail("bad-record", "repository"));
    }
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, subject);
    receipt::push_field(&mut encoded, repo);
    receipt::push_field(&mut encoded, &source.commit);
    receipt::push_field(&mut encoded, &source.source);
    receipt::push_field(&mut encoded, &source.selection);
    Ok(encoded)
}

fn decode_coordinates(body: &str) -> Result<(String, SelectedSource), PackFailure> {
    let (subject, rest) = receipt::take_field(body)?;
    let (repository, rest) = receipt::take_field(rest)?;
    let (commit, rest) = receipt::take_field(rest)?;
    let (source, rest) = receipt::take_field(rest)?;
    let (selection, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", FACT));
    }
    Ok((
        subject.to_owned(),
        SelectedSource {
            repository: std::path::PathBuf::from(repository),
            commit: commit.to_owned(),
            source: source.to_owned(),
            selection: selection.to_owned(),
        },
    ))
}

struct SlotExecution {
    rev: String,
    source_digest: String,
    slot_digest: String,
    consumption: String,
}

fn decode_execution(body: &str) -> Result<SlotExecution, PackFailure> {
    let (rev, rest) = receipt::take_field(body)?;
    let (source_digest, rest) = receipt::take_field(rest)?;
    let (slot_digest, rest) = receipt::take_field(rest)?;
    let (consumption, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() || source_digest.len() != 64 || slot_digest.len() != 64 {
        return Err(fail("bad-record", "completion.execution"));
    }
    Ok(SlotExecution {
        rev: rev.to_owned(),
        source_digest: source_digest.to_owned(),
        slot_digest: slot_digest.to_owned(),
        consumption: consumption.to_owned(),
    })
}

fn one_fact<'a>(request: &'a PackRequest<'a>) -> Result<&'a str, PackFailure> {
    let mut body = None;
    for key in request.evidence.fact_keys() {
        match key {
            FACT => body = request.evidence.fact(key),
            other => return Err(fail("unexpected-fact", other)),
        }
    }
    body.ok_or_else(|| fail("missing-source", FACT))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "completion.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-source", "completion.execution"))
}

fn observation(key: &str, value: impl Into<String>) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: value.into(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_bytes(Sha256::digest(bytes).as_slice())
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_rev(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn scope_id() -> Result<CheckScopeId, PackFailure> {
    CheckScopeId::from_text(SCOPE).map_err(|error| fail("receipt-refused", &error.to_string()))
}

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}

#[cfg(test)]
#[path = "release_neutral_tests.rs"]
mod tests;
