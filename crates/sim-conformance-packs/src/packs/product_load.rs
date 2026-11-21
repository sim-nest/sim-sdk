// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Joined load of world, prove, deliver, and roadmap.
//!
//! The check runs those four commands and their four index routes through
//! the program named by `product.command`. It reads each route specimen and
//! that crate's brochure under `product.root`. The same command must print
//! the same bytes on the second run. Boot, recipe, brochure, index, and
//! route guards stay their own results. A caller label is not this result.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};

use super::COMMAND_FACT;
use super::command;

#[path = "product_load_parse.rs"]
mod parse;

use crate::packs::release::receipt::{self, AdmitExecuted, PackageRow};
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "completion/product-load";
const RESULT: &str = "completion/claim-18";
const ROOT_FACT: &str = "product.root";
const PROOF: &str = "product-load";
const DOMAIN: &str = "completion/product-load";

const COMMANDS: &[(&str, &[&str])] = &[
    (
        "product.load.world",
        &[
            "world",
            "why",
            "conclusion/public-release",
            "policy/no-v3-disclosure",
        ],
    ),
    ("product.load.prove", &["prove", "plan"]),
    ("product.load.deliver", &["world", "deliver", "model"]),
    (
        "product.load.roadmap",
        &[
            "roadmap",
            "status",
            "--conduct",
            "conduct/local",
            "--model-pick",
            "model/local",
            "--proof-catalog",
            "proof/local",
            "--runner-generation",
            "generation/local",
        ],
    ),
];

const ROUTES: &[(&str, &str, &str)] = &[
    (
        "product.load.route.world",
        "route/run-world-command",
        "Project supplied semantic facts and explain their exact affected closure",
    ),
    (
        "product.load.route.prove",
        "route/run-prove-command",
        "Run a cold proof and read its frontier",
    ),
    (
        "product.load.route.deliver",
        "route/run-deliver-command",
        "Read a delivery record through the installed world command",
    ),
    (
        "product.load.route.roadmap",
        "route/run-roadmap-command",
        "Read the local roadmap status record",
    ),
];

/// Receipt the joined product load admits.
#[derive(Debug)]
pub struct ObservedProductLoad {
    /// Subject the checker invocation and the receipt both name.
    pub subject: String,
    /// `completion/claim-18` for this subject.
    pub result: String,
    /// Joined command and route stdout. This string is not the claim.
    pub execution: String,
    /// Canonical program the check ran.
    pub program: String,
    /// Canonical constellation root whose specimens were read.
    pub root: String,
    /// Catalog that admits the evidence.
    pub catalog: sim_lib_roadmap_runner::ProofCatalog,
    /// Evidence the catalog admits.
    pub admitted: sim_lib_roadmap_runner::AdmittedEvidence,
    /// Checker receipt that binds the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `completion/product-load`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Runs the joined load and admits its result.
///
/// # Errors
///
/// Returns a refusal when the subject is not a token, a fact carries a
/// completion label, a command is unstable, or a route specimen is absent.
pub fn observe_product_load(
    subject: &str,
    program: &Path,
    root: &Path,
) -> Result<ObservedProductLoad, PackFailure> {
    if !token(subject) {
        return Err(command::fail("bad-record", "subject"));
    }
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| command::fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-product")
        .ok_or_else(|| command::fail("unknown-checker", "checker/c-product"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| command::fail("invalid-static-binding", &error.to_string()))?;
    let evidence = MemorySubject::default()
        .with(COMMAND_FACT, program.display().to_string())
        .with(ROOT_FACT, root.display().to_string());
    let verdict = super::check(&PackRequest {
        checker: "checker/c-product",
        binding: &receipt::render(binding.id().content_id()),
        subject: &receipt::render(checked.content_id()),
        scope: SCOPE,
        evidence: &evidence,
    });
    let observations = match verdict {
        PackVerdict::Pass { observations, .. } => observations,
        PackVerdict::Refused(failure) => return Err(failure),
        PackVerdict::UnimplementedPack { .. } => return Err(command::fail("unimplemented", SCOPE)),
    };
    admit(subject, program, root, &observations)
}

/// Re-checks a product-load receipt against the catalog that admitted it.
///
/// The execution string has to be the joined stdout. A completion label in
/// that string is refused before the commands run again.
///
/// # Errors
///
/// Returns a refusal when the result, scope, or execution is no longer the
/// joined load.
pub fn bind_product_load(observation: &ObservedProductLoad) -> Result<(), PackFailure> {
    if observation.result != RESULT
        || observation.scope != scope_id()?
        || observation.execution.contains("completion/claim-")
    {
        return Err(command::fail("claim-relabelled", SCOPE));
    }
    let again = load(
        Path::new(&observation.program),
        Path::new(&observation.root),
    )?;
    if observation.execution != again.execution {
        return Err(command::fail("evidence-mismatch", ROOT_FACT));
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
    let (program, root) = facts(request)?;
    let loaded = load(&program, &root)?;
    if loaded.execution.contains("completion/claim-") {
        return Err(command::fail("claim-relabelled", SCOPE));
    }
    Ok(KEYS
        .iter()
        .zip(loaded.parts.iter())
        .map(|(key, value)| command::observation(key, value))
        .collect())
}

struct Loaded {
    parts: Vec<String>,
    execution: String,
    digest: Vec<u8>,
    seal: String,
}

fn load(program: &Path, root: &Path) -> Result<Loaded, PackFailure> {
    let mut parts = Vec::with_capacity(COMMANDS.len() + ROUTES.len());
    for (key, args) in COMMANDS {
        let stdout = parse::stable(program, args)?;
        parse::require_command(key, &stdout)?;
        parts.push(stdout);
    }
    let mut routes = Vec::with_capacity(ROUTES.len());
    for (_, id, task) in ROUTES {
        let stdout = parse::stable(program, &["index", "route", task])?;
        routes.push((*id, stdout));
    }
    for (id, stdout) in &routes {
        let expected = parse::specimen_for(id)?;
        parse::require_route(stdout, id, expected)?;
        parse::read_specimen(root, expected)?;
        parts.push(stdout.clone());
    }
    let execution = join(&parts);
    let digest = Sha256::digest(execution.as_bytes());
    if digest.iter().all(|byte| *byte == 0) {
        return Err(command::fail("unstable", "seal"));
    }
    Ok(Loaded {
        parts,
        execution,
        seal: hex(digest.as_slice()),
        digest: digest.to_vec(),
    })
}

const KEYS: &[&str] = &[
    "product.load.world",
    "product.load.prove",
    "product.load.deliver",
    "product.load.roadmap",
    "product.load.route.world",
    "product.load.route.prove",
    "product.load.route.deliver",
    "product.load.route.roadmap",
];

fn admit(
    subject: &str,
    program: &Path,
    root: &Path,
    observations: &[CheckObservation],
) -> Result<ObservedProductLoad, PackFailure> {
    let program = file_path(program, COMMAND_FACT)?;
    let root = directory(
        root.to_str()
            .ok_or_else(|| command::fail("bad-record", ROOT_FACT))?,
    )?;
    let executed = execution(observations)?;
    let again = load(&program, &root)?;
    if executed != again.execution {
        return Err(command::fail("evidence-mismatch", ROOT_FACT));
    }
    let rows = vec![PackageRow {
        name: PROOF.to_owned(),
        version: "0".to_owned(),
        url: format!("git://{PROOF}"),
        rev: again.seal.clone(),
    }];
    let admitted = receipt::admit_for(
        "checker/c-product",
        AdmitExecuted {
            subject,
            result: RESULT,
            scope: SCOPE,
            proof: PROOF,
            domain: DOMAIN,
            policy: "policy/product-load",
            provenance: "provenance/product-load",
            support: "support/product-load",
            projection: "projection/product-load",
            environment: "environment/product-load",
            leaf: &again.digest,
            source_text: &receipt::package_source(DOMAIN, PROOF, &rows),
            definitions: &receipt::package_definitions(DOMAIN, &rows),
        },
    )?;
    Ok(ObservedProductLoad {
        subject: subject.to_owned(),
        result: RESULT.to_owned(),
        execution: executed,
        program: program.display().to_string(),
        root: root.display().to_string(),
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn facts(request: &PackRequest<'_>) -> Result<(PathBuf, PathBuf), PackFailure> {
    let mut program = None;
    let mut root = None;
    for key in request.evidence.fact_keys() {
        match key {
            COMMAND_FACT => program = request.evidence.fact(key),
            ROOT_FACT => root = request.evidence.fact(key),
            other => return Err(command::fail("unexpected-fact", other)),
        }
    }
    let program = program.ok_or_else(|| command::fail("missing-source", COMMAND_FACT))?;
    let root = root.ok_or_else(|| command::fail("missing-source", ROOT_FACT))?;
    if program.contains("completion/claim-")
        || root.contains("completion/claim-")
        || request.subject.contains("completion/claim-")
    {
        return Err(command::fail("claim-relabelled", SCOPE));
    }
    Ok((
        file_path(Path::new(program), COMMAND_FACT)?,
        directory(root)?,
    ))
}

fn file_path(path: &Path, fact: &str) -> Result<PathBuf, PackFailure> {
    let text = path
        .to_str()
        .ok_or_else(|| command::fail("bad-record", fact))?;
    if text.is_empty() || text.contains('\0') || text.contains('\n') || text == "true" {
        return Err(command::fail("bad-record", fact));
    }
    let canonical =
        std::fs::canonicalize(path).map_err(|_| command::fail("missing-source", fact))?;
    if canonical.is_file() {
        Ok(canonical)
    } else {
        Err(command::fail("missing-source", fact))
    }
}

fn directory(text: &str) -> Result<PathBuf, PackFailure> {
    if text.is_empty() || text.contains('\0') || text.contains('\n') || text == "true" {
        return Err(command::fail("bad-record", ROOT_FACT));
    }
    let canonical =
        std::fs::canonicalize(text).map_err(|_| command::fail("missing-source", ROOT_FACT))?;
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        Err(command::fail("missing-source", ROOT_FACT))
    }
}

fn execution(observations: &[CheckObservation]) -> Result<String, PackFailure> {
    let mut parts = Vec::with_capacity(KEYS.len());
    for key in KEYS {
        let value = observations
            .iter()
            .find(|item| item.key == *key)
            .map(|item| item.value.as_str())
            .ok_or_else(|| command::fail("missing-source", key))?;
        if value.contains("completion/claim-") {
            return Err(command::fail("claim-relabelled", key));
        }
        parts.push(value.to_owned());
    }
    Ok(join(&parts))
}

fn join(parts: &[String]) -> String {
    let mut out = String::new();
    for part in parts {
        receipt::push_field(&mut out, part);
    }
    out
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.contains("completion/claim-")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b' ')
}

fn scope_id() -> Result<CheckScopeId, PackFailure> {
    CheckScopeId::from_text(SCOPE)
        .map_err(|error| command::fail("receipt-refused", &error.to_string()))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0xf) as usize] as char);
    }
    out
}
