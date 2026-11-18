// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! One fixed repository command, executed in the root that contains it.
//!
//! A guard names the script, its arguments, and the receipt result. This
//! module does not accept a command string or a passed count. The result
//! is the guard's own name.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, CheckerReceiptId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

pub(super) struct GuardSpec {
    pub scope: &'static str,
    pub domain: &'static str,
    pub fact: &'static str,
    pub result: &'static str,
    pub script: &'static str,
    pub args: &'static [&'static str],
}

pub(super) struct ObservedCommand {
    pub subject: String,
    pub result: String,
    pub root: String,
    pub catalog: ProofCatalog,
    pub admitted: AdmittedEvidence,
    pub checker: CheckerReceiptId,
    pub scope: CheckScopeId,
    pub revocation: RevocationStatus,
}

pub(super) fn observe(
    spec: &GuardSpec,
    subject: &str,
    root: &Path,
) -> Result<ObservedCommand, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let pack = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = pack
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default().with(spec.fact, encode_root(root)?);
    let verdict = super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &receipt::render(binding.id().content_id()),
        subject: &receipt::render(checked.content_id()),
        scope: spec.scope,
        evidence: &evidence,
    });
    let observations = match verdict {
        PackVerdict::Pass { observations, .. } => observations,
        PackVerdict::Refused(failure) => return Err(failure),
        PackVerdict::UnimplementedPack { .. } => return Err(fail("unimplemented", spec.scope)),
    };
    admit_rows(spec, subject, &observations)
}

pub(super) struct BoundCommand<'a> {
    pub subject: &'a str,
    pub result: &'a str,
    pub scope: &'a CheckScopeId,
    pub checker: &'a CheckerReceiptId,
    pub catalog: &'a ProofCatalog,
    pub admitted: &'a AdmittedEvidence,
    pub revocation: &'a RevocationStatus,
}

pub(super) fn bind(spec: &GuardSpec, observed: BoundCommand<'_>) -> Result<(), PackFailure> {
    if observed.result != spec.result || observed.scope != &scope_id(spec)? {
        return Err(fail("claim-relabelled", spec.scope));
    }
    receipt::bind_executed(
        observed.subject,
        observed.result,
        observed.scope,
        observed.checker,
        observed.catalog,
        observed.admitted,
        observed.revocation.clone(),
    )
}

pub(super) fn observations(
    spec: &GuardSpec,
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let root = decode_root(spec, one_fact(spec, request)?)?;
    let script = script_in_root(spec, &root)?;
    let (stdout, stderr) = run_in_root(spec, &root, &script)?;
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, &root.to_string_lossy());
    receipt::push_field(
        &mut encoded,
        std::str::from_utf8(&stdout).map_err(|_| fail("bad-record", "stdout"))?,
    );
    receipt::push_field(
        &mut encoded,
        std::str::from_utf8(&stderr).map_err(|_| fail("bad-record", "stderr"))?,
    );
    Ok(vec![observation("release.execution", encoded)])
}

fn admit_rows(
    spec: &GuardSpec,
    subject: &str,
    observations: &[CheckObservation],
) -> Result<ObservedCommand, PackFailure> {
    let encoded = execution(observations)?;
    let (root, stdout, stderr) = decode_execution(encoded)?;
    let mut hasher = Sha256::new();
    hasher.update(stdout.as_bytes());
    hasher.update(stderr.as_bytes());
    let rows = vec![PackageRow {
        name: spec.result.to_owned(),
        version: "0".to_owned(),
        url: format!("git://{}", spec.result),
        rev: hex_bytes(hasher.finalize().as_slice()),
    }];
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: spec.result,
        scope: spec.scope,
        proof: spec.result,
        domain: spec.domain,
        policy: &format!("policy/{}", spec.result),
        provenance: &format!("provenance/{}", spec.result),
        support: &format!("support/{}", spec.result),
        projection: &format!("projection/{}", spec.result),
        environment: &format!("environment/{}", spec.result),
        leaf: encoded.as_bytes(),
        source_text: &receipt::package_source(spec.domain, spec.result, &rows),
        definitions: &receipt::package_definitions(spec.domain, &rows),
    })?;
    Ok(ObservedCommand {
        subject: subject.to_owned(),
        result: spec.result.to_owned(),
        root,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn run_in_root(
    spec: &GuardSpec,
    root: &Path,
    script: &Path,
) -> Result<(Vec<u8>, Vec<u8>), PackFailure> {
    let program = "/bin/sh";
    let script_text = script
        .to_str()
        .ok_or_else(|| fail("bad-record", "script"))?;
    if script_text.contains('\0') {
        return Err(fail("process-unrun", "argv"));
    }
    let mut command = Command::new(program);
    command.arg(script_text).current_dir(root);
    for arg in spec.args {
        if arg.contains('\0') {
            return Err(fail("process-unrun", "argv"));
        }
        command.arg(arg);
    }
    let output = command
        .output()
        .map_err(|_| fail("process-unrun", program))?;
    if output.stdout.len() > 1_048_576 || output.stderr.len() > 1_048_576 {
        return Err(fail("process-output", program));
    }
    let status = output
        .status
        .code()
        .ok_or_else(|| fail("process-unrun", "signal"))?;
    if status != 0 {
        return Err(fail("process-failed", &status.to_string()));
    }
    Ok((output.stdout, output.stderr))
}

fn script_in_root(spec: &GuardSpec, root: &Path) -> Result<PathBuf, PackFailure> {
    let script = root.join(spec.script);
    let canonical =
        std::fs::canonicalize(&script).map_err(|_| fail("missing-source", spec.script))?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(fail("bad-record", spec.script));
    }
    Ok(canonical)
}

fn encode_root(root: &Path) -> Result<String, PackFailure> {
    let text = root.to_str().ok_or_else(|| fail("bad-record", "root"))?;
    if text.is_empty()
        || text.contains('\0')
        || text.contains('\n')
        || text.contains("completion/claim-")
    {
        return Err(fail("bad-record", "root"));
    }
    let mut encoded = String::new();
    receipt::push_field(&mut encoded, text);
    Ok(encoded)
}

fn decode_root(spec: &GuardSpec, body: &str) -> Result<PathBuf, PackFailure> {
    if body.contains("completion/claim-") {
        return Err(fail("claim-relabelled", spec.fact));
    }
    let (text, rest) = receipt::take_field(body)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", spec.fact));
    }
    let root = std::fs::canonicalize(text).map_err(|_| fail("missing-source", "root"))?;
    if !root.is_dir() {
        return Err(fail("bad-record", "root"));
    }
    Ok(root)
}

fn decode_execution(body: &str) -> Result<(String, String, String), PackFailure> {
    let (root, rest) = receipt::take_field(body)?;
    let (stdout, rest) = receipt::take_field(rest)?;
    let (stderr, rest) = receipt::take_field(rest)?;
    if !rest.is_empty() {
        return Err(fail("bad-record", "release.execution"));
    }
    Ok((root.to_owned(), stdout.to_owned(), stderr.to_owned()))
}

fn one_fact<'a>(spec: &GuardSpec, request: &'a PackRequest<'a>) -> Result<&'a str, PackFailure> {
    let mut body = None;
    for key in request.evidence.fact_keys() {
        if key == spec.fact {
            body = request.evidence.fact(key);
        } else {
            return Err(fail("unexpected-fact", key));
        }
    }
    body.ok_or_else(|| fail("missing-source", spec.fact))
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "release.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-source", "release.execution"))
}

fn scope_id(spec: &GuardSpec) -> Result<CheckScopeId, PackFailure> {
    CheckScopeId::from_text(spec.scope).map_err(|error| fail("receipt-refused", &error.to_string()))
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
