// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Outward release obligations read from git, crate bytes, and a boot process.
//!
//! Unpublished packages are read with the git-worktree observer. Each
//! published package is the `.crate` file whose SHA-256 equals one unyanked
//! index line. The tag `{name}-v{version}` must peel to the packaged VCS
//! commit. The pin, the checkout `HEAD`, and both mirror `refs/heads/main`
//! values must name one selected repository revision, and that revision
//! must contain the packaged commit. A package released from an earlier
//! commit on that history is current. Dependency order is computed from
//! normal manifest dependencies. The boot program is executed. A caller
//! flag is not publication, and git identities alone are not this receipt.
//!
//! The result is `completion/claim-21` plus the sorted unpublished identities.
//! Registry packages stay out of that tail. CI is not read.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sim_conformance_v02::{CheckScopeId, CheckedSubjectId, RevocationStatus};
use sim_lib_roadmap_runner::{AdmittedEvidence, ProofCatalog};

use super::receipt::{self, AdmitExecuted, PackageRow};
use super::worktree::{GitPackage, observe_git_release};
use crate::packs::completion_claim::execute_process;
use crate::{CheckObservation, MemorySubject, PackFailure, PackRequest, PackVerdict, find_pack};

const SCOPE: &str = "release/observed-obligations";
const DOMAIN: &str = "resolver/release";
const FACT: &str = "release.obligations";
const MAX_PACKAGES: usize = 64;
const MAX_INDEX: usize = 1_048_576;
const MAX_CRATE: usize = 64 * 1_048_576;

#[path = "release_observed_read.rs"]
mod read;

/// One registry package whose outward release facts are read, not supplied.
pub struct PublishedObligation {
    /// Index file whose unyanked line must match the crate hash.
    pub index: PathBuf,
    /// Packaged `.crate` bytes. The checksum is their SHA-256.
    pub krate: PathBuf,
    /// Git repository that contains `refs/tags/{name}-v{version}`.
    pub repository: PathBuf,
    /// Manifest whose `commit` pin is read.
    pub pin_manifest: PathBuf,
    /// Repository name inside that manifest.
    pub pin_name: String,
    /// Checkout whose `HEAD` must equal the pin.
    pub checkout: PathBuf,
    /// First mirror. A local path or a remote URL.
    pub mirror_left: String,
    /// Second mirror. It must not be the same path or URL as the first.
    pub mirror_right: String,
}

/// Unpublished git packages plus the published obligations and the boot program.
pub struct ReleaseObligations {
    /// Packages read from git. Empty when every obligation is published.
    pub unpublished: Vec<GitPackage>,
    /// Published packages. At least one is required.
    pub published: Vec<PublishedObligation>,
    /// Boot program. It is executed; a status word is not accepted.
    pub boot_program: PathBuf,
    /// Arguments passed to the boot program.
    pub boot_args: Vec<String>,
}

/// One published crate after its checksum, tag, mirrors, and pin were read.
pub struct ObservedPublished {
    /// Package name from the index, the crate, and the tagged manifest.
    pub name: String,
    /// Package version from those same three reads.
    pub version: String,
    /// SHA-256 of the `.crate` bytes.
    pub checksum: String,
    /// VCS commit the package bytes and the release tag named.
    pub commit: String,
    /// Repository revision the pin, the checkout, and the mirrors named.
    pub selected: String,
}

/// Claim 21 receipt for one executed obligation set.
pub struct ObservedReleaseObligations {
    /// Subject the receipt names.
    pub subject: String,
    /// `completion/claim-21`, plus each unpublished git identity, sorted.
    pub result: String,
    /// Unpublished identities, in result order.
    pub identities: Vec<String>,
    /// Package names in computed dependency order.
    pub order: Vec<String>,
    /// Published crates whose outward facts were read.
    pub published: Vec<ObservedPublished>,
    /// Catalog that admitted the evidence.
    pub catalog: ProofCatalog,
    /// Evidence the catalog admitted.
    pub admitted: AdmittedEvidence,
    /// Checker receipt that bound the admission.
    pub checker: sim_conformance_v02::CheckerReceiptId,
    /// Scope `release/observed-obligations`.
    pub scope: CheckScopeId,
    /// Revocation state recorded for that checker.
    pub revocation: RevocationStatus,
}

/// Reads the obligation set and admits claim 21 for its git identities.
///
/// # Errors
///
/// Returns a refusal when a checksum, tag, mirror, pin, checkout, order, or
/// boot read does not agree, or when no published package was supplied.
pub fn observe_release_obligations(
    subject: &str,
    input: &ReleaseObligations,
) -> Result<ObservedReleaseObligations, PackFailure> {
    let checked = CheckedSubjectId::from_text(subject)
        .map_err(|error| fail("subject-identity", &error.to_string()))?;
    let spec = find_pack("checker/c-release")
        .ok_or_else(|| fail("unknown-checker", "checker/c-release"))?;
    let binding = spec
        .checker_binding()
        .map_err(|error| fail("invalid-static-binding", &error.to_string()))?;
    let binding_id = receipt::render(binding.id().content_id());
    let evidence = MemorySubject::default().with(FACT, encode(subject, input)?);
    let verdict = super::check(&PackRequest {
        checker: "checker/c-release",
        binding: &binding_id,
        subject: &receipt::render(checked.content_id()),
        scope: SCOPE,
        evidence: &evidence,
    });
    let observations = match verdict {
        PackVerdict::Pass { observations, .. } => observations,
        PackVerdict::Refused(failure) => return Err(failure),
        PackVerdict::UnimplementedPack { .. } => return Err(fail("unimplemented", SCOPE)),
    };
    admit(subject, input, &observations)
}

/// Re-checks an obligation receipt against the catalog that admitted it.
///
/// # Errors
///
/// Returns a refusal when the result or scope no longer matches the git tail.
pub fn bind_release_obligations(
    observation: &ObservedReleaseObligations,
) -> Result<(), PackFailure> {
    let expected = claim_result(&observation.identities);
    if observation.result != expected || observation.scope != scope_id()? {
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
    if body.contains("completion/claim-") || body.contains("git+") {
        return Err(fail("claim-relabelled", FACT));
    }
    let (subject, input) = decode(body)?;
    let executed = execute(&subject, &input)?;
    Ok(vec![observation("release.execution", executed.encoded)])
}

struct Executed {
    identities: Vec<String>,
    order: Vec<String>,
    published: Vec<ObservedPublished>,
    rows: Vec<PackageRow>,
    encoded: String,
}

fn admit(
    subject: &str,
    input: &ReleaseObligations,
    observations: &[CheckObservation],
) -> Result<ObservedReleaseObligations, PackFailure> {
    let encoded = execution(observations)?;
    let fresh = execute(subject, input)?;
    if encoded != fresh.encoded {
        return Err(fail("evidence-mismatch", "release.execution"));
    }
    let result = claim_result(&fresh.identities);
    let source = receipt::package_source(DOMAIN, "release-obligations", &fresh.rows);
    let definitions = receipt::package_definitions(DOMAIN, &fresh.rows);
    let admitted = receipt::admit_executed(AdmitExecuted {
        subject,
        result: &result,
        scope: SCOPE,
        proof: "release-obligations",
        domain: DOMAIN,
        policy: "policy/release-obligations",
        provenance: "provenance/release-obligations",
        support: "support/release-obligations",
        projection: "projection/release-obligations",
        environment: "environment/release-obligations",
        leaf: fresh.encoded.as_bytes(),
        source_text: &source,
        definitions: &definitions,
    })?;
    Ok(ObservedReleaseObligations {
        subject: subject.to_owned(),
        result,
        identities: fresh.identities,
        order: fresh.order,
        published: fresh.published,
        catalog: admitted.catalog,
        admitted: admitted.admitted,
        checker: admitted.checker,
        scope: admitted.scope,
        revocation: admitted.revocation,
    })
}

fn execute(subject: &str, input: &ReleaseObligations) -> Result<Executed, PackFailure> {
    if input.published.is_empty() || input.unpublished.len() + input.published.len() > MAX_PACKAGES
    {
        return Err(fail("missing-source", FACT));
    }
    let git = read_git(subject, input)?;
    let mut names = BTreeMap::<String, BTreeSet<String>>::new();
    let mut rows = Vec::new();
    for identity in &git {
        let row = parse_identity(identity)?;
        if names.insert(row.name.clone(), BTreeSet::new()).is_some() {
            return Err(fail("duplicate-crate", &row.name));
        }
        rows.push(row);
    }
    for package in &input.unpublished {
        let manifest = read::git_show(&package.repository, &package.commit, &package.manifest)?;
        let parsed = read::parse_manifest(&manifest, false)?;
        let identity = format!("git+{}:{}@{}", package.commit, parsed.name, parsed.version);
        if !git.contains(&identity) {
            return Err(fail("evidence-mismatch", &parsed.name));
        }
        let deps = names
            .get_mut(&parsed.name)
            .ok_or_else(|| fail("evidence-mismatch", &parsed.name))?;
        *deps = parsed.deps;
    }
    let mut published = Vec::new();
    for package in &input.published {
        let record = read::read_published(package)?;
        if names
            .insert(record.parsed.name.clone(), record.parsed.deps.clone())
            .is_some()
        {
            return Err(fail("duplicate-crate", &record.parsed.name));
        }
        rows.push(PackageRow {
            name: record.parsed.name.clone(),
            version: record.parsed.version.clone(),
            url: format!("registry://{}", record.observed.checksum),
            rev: record.observed.commit.clone(),
        });
        published.push(record.observed);
    }
    let known = names.keys().cloned().collect::<BTreeSet<_>>();
    for deps in names.values_mut() {
        deps.retain(|dep| known.contains(dep));
    }
    let order = release_order(&names)?;
    let boot = run_boot(input)?;
    published.sort_by(|left, right| left.name.cmp(&right.name));
    rows.sort_by(|left, right| left.name.cmp(&right.name));
    let encoded = encode_execution(&git, &order, &published, &boot);
    Ok(Executed {
        identities: git,
        order,
        published,
        rows,
        encoded,
    })
}

fn read_git(subject: &str, input: &ReleaseObligations) -> Result<Vec<String>, PackFailure> {
    if input.unpublished.is_empty() {
        return Ok(Vec::new());
    }
    let observed = observe_git_release(subject, &input.unpublished)?;
    if observed.identities.len() != input.unpublished.len() {
        return Err(fail("evidence-mismatch", "git"));
    }
    Ok(observed.identities)
}

fn release_order(nodes: &BTreeMap<String, BTreeSet<String>>) -> Result<Vec<String>, PackFailure> {
    let mut pending = nodes.keys().cloned().collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let Some(next) = pending
            .iter()
            .find(|name| {
                nodes
                    .get(*name)
                    .is_some_and(|deps| deps.iter().all(|dep| !pending.contains(dep)))
            })
            .cloned()
        else {
            return Err(fail("dependency-cycle", "order"));
        };
        pending.remove(&next);
        order.push(next);
    }
    Ok(order)
}

fn run_boot(input: &ReleaseObligations) -> Result<String, PackFailure> {
    if input.boot_args.len() > 16 {
        return Err(fail("bound", "boot"));
    }
    for arg in &input.boot_args {
        token(arg, "boot")?;
    }
    let program =
        std::fs::canonicalize(&input.boot_program).map_err(|_| fail("missing-source", "boot"))?;
    if !program.is_file() {
        return Err(fail("bad-record", "boot"));
    }
    let program = program
        .to_str()
        .ok_or_else(|| fail("bad-record", "boot"))?
        .to_owned();
    token(&program, "boot")?;
    let args = input
        .boot_args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let ran = execute_process(&program, &args)?;
    Ok(hex(&Sha256::digest(ran.stdout)))
}

fn encode(subject: &str, input: &ReleaseObligations) -> Result<String, PackFailure> {
    token(subject, "subject")?;
    if input.published.is_empty() || input.unpublished.len() + input.published.len() > MAX_PACKAGES
    {
        return Err(fail("missing-source", FACT));
    }
    let mut out = String::new();
    receipt::push_field(&mut out, subject);
    receipt::push_field(&mut out, &input.unpublished.len().to_string());
    for package in &input.unpublished {
        receipt::push_field(&mut out, &path_token(&package.repository, "repository")?);
        token(&package.commit, "commit")?;
        token(&package.manifest, "manifest")?;
        receipt::push_field(&mut out, &package.commit);
        receipt::push_field(&mut out, &package.manifest);
    }
    receipt::push_field(&mut out, &input.published.len().to_string());
    for package in &input.published {
        for path in [
            &package.index,
            &package.krate,
            &package.repository,
            &package.pin_manifest,
            &package.checkout,
        ] {
            receipt::push_field(&mut out, &path_token(path, "path")?);
        }
        token(&package.pin_name, "pin")?;
        token(&package.mirror_left, "mirror")?;
        token(&package.mirror_right, "mirror")?;
        receipt::push_field(&mut out, &package.pin_name);
        receipt::push_field(&mut out, &package.mirror_left);
        receipt::push_field(&mut out, &package.mirror_right);
    }
    receipt::push_field(&mut out, &path_token(&input.boot_program, "boot")?);
    receipt::push_field(&mut out, &input.boot_args.len().to_string());
    for arg in &input.boot_args {
        token(arg, "boot")?;
        receipt::push_field(&mut out, arg);
    }
    Ok(out)
}

fn decode(body: &str) -> Result<(String, ReleaseObligations), PackFailure> {
    let mut fields = Fields { rest: body };
    let subject = fields.next()?.to_owned();
    let unpublished_len = count(fields.next()?)?;
    let mut unpublished = Vec::with_capacity(unpublished_len);
    for _ in 0..unpublished_len {
        unpublished.push(GitPackage {
            repository: PathBuf::from(fields.next()?),
            commit: fields.next()?.to_owned(),
            manifest: fields.next()?.to_owned(),
        });
    }
    let published_len = count(fields.next()?)?;
    if published_len == 0 || unpublished_len + published_len > MAX_PACKAGES {
        return Err(fail("missing-source", FACT));
    }
    let mut published = Vec::with_capacity(published_len);
    for _ in 0..published_len {
        published.push(PublishedObligation {
            index: PathBuf::from(fields.next()?),
            krate: PathBuf::from(fields.next()?),
            repository: PathBuf::from(fields.next()?),
            pin_manifest: PathBuf::from(fields.next()?),
            checkout: PathBuf::from(fields.next()?),
            pin_name: fields.next()?.to_owned(),
            mirror_left: fields.next()?.to_owned(),
            mirror_right: fields.next()?.to_owned(),
        });
    }
    let boot_program = PathBuf::from(fields.next()?);
    let arg_len = count(fields.next()?)?;
    if arg_len > 16 {
        return Err(fail("bound", "boot"));
    }
    let mut boot_args = Vec::with_capacity(arg_len);
    for _ in 0..arg_len {
        boot_args.push(fields.next()?.to_owned());
    }
    fields.finish()?;
    Ok((
        subject,
        ReleaseObligations {
            unpublished,
            published,
            boot_program,
            boot_args,
        },
    ))
}

fn encode_execution(
    identities: &[String],
    order: &[String],
    published: &[ObservedPublished],
    boot_sha: &str,
) -> String {
    let mut out = String::new();
    push_list(&mut out, identities);
    push_list(&mut out, order);
    receipt::push_field(&mut out, &published.len().to_string());
    for crate_row in published {
        receipt::push_field(&mut out, &crate_row.name);
        receipt::push_field(&mut out, &crate_row.version);
        receipt::push_field(&mut out, &crate_row.checksum);
        receipt::push_field(&mut out, &crate_row.commit);
        receipt::push_field(&mut out, &crate_row.selected);
    }
    receipt::push_field(&mut out, boot_sha);
    out
}

fn push_list(out: &mut String, values: &[String]) {
    receipt::push_field(out, &values.len().to_string());
    for value in values {
        receipt::push_field(out, value);
    }
}

struct Fields<'a> {
    rest: &'a str,
}

impl<'a> Fields<'a> {
    fn next(&mut self) -> Result<&'a str, PackFailure> {
        let (field, rest) = receipt::take_field(self.rest)?;
        self.rest = rest;
        Ok(field)
    }

    fn finish(self) -> Result<(), PackFailure> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(fail("bad-record", FACT))
        }
    }
}

fn count(text: &str) -> Result<usize, PackFailure> {
    let value = text
        .parse::<usize>()
        .map_err(|_| fail("bad-record", FACT))?;
    if value > MAX_PACKAGES {
        return Err(fail("bound", FACT));
    }
    Ok(value)
}

fn parse_identity(identity: &str) -> Result<PackageRow, PackFailure> {
    let Some(rest) = identity.strip_prefix("git+") else {
        return Err(fail("evidence-mismatch", "identity"));
    };
    let Some((rev, spec)) = rest.split_once(':') else {
        return Err(fail("evidence-mismatch", "identity"));
    };
    let Some((name, version)) = spec.split_once('@') else {
        return Err(fail("evidence-mismatch", "identity"));
    };
    if !is_rev(rev) || !is_name(name) || !is_version(version) {
        return Err(fail("evidence-mismatch", "identity"));
    }
    Ok(PackageRow {
        name: name.to_owned(),
        version: version.to_owned(),
        url: "git://worktree".to_owned(),
        rev: rev.to_owned(),
    })
}

fn claim_result(identities: &[String]) -> String {
    let mut result = String::from("completion/claim-21");
    for identity in identities {
        result.push(' ');
        result.push_str(identity);
    }
    result
}

fn path_token(path: &Path, code: &str) -> Result<String, PackFailure> {
    let text = path.to_str().ok_or_else(|| fail("bad-record", code))?;
    token(text, code)?;
    Ok(text.to_owned())
}

fn token(value: &str, code: &str) -> Result<(), PackFailure> {
    if value.is_empty()
        || value.contains('\0')
        || value.contains('\n')
        || value.contains("completion/claim-")
        || value.contains("git+")
    {
        return Err(fail("bad-record", code));
    }
    Ok(())
}

fn execution(observations: &[CheckObservation]) -> Result<&str, PackFailure> {
    observations
        .iter()
        .find(|item| item.key == "release.execution")
        .map(|item| item.value.as_str())
        .ok_or_else(|| fail("missing-source", "release.execution"))
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

fn scope_id() -> Result<CheckScopeId, PackFailure> {
    CheckScopeId::from_text(SCOPE).map_err(|error| fail("receipt-refused", &error.to_string()))
}

fn is_rev(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        && !matches!(value, "true" | "published" | "shade-1" | "PortableClosed")
}

fn is_version(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value.bytes().any(|byte| byte.is_ascii_digit())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
}

fn hex(bytes: &[u8]) -> String {
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

#[cfg(test)]
#[path = "release_observed_tests.rs"]
mod tests;
