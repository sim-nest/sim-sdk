// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Portability conformance pack.
//!
//! The foreign-project scope calls the frozen executor and reads the owner
//! back from the source provider. The consumer-seams scope runs the outside
//! composition specimen. The law trait remains available to substituted laws.

#[path = "consumer_seams.rs"]
pub mod consumer_seams;

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Public seam used to judge the canonical law and a substituted law.
pub trait PortabilityLaw {
    /// Reads one selector from a caller-named batch.
    fn snapshot(&self, batch: &str, selector: &str) -> Snapshot;
    /// Compares one selector across two batches.
    fn snapshot_diff(&self, before: &str, after: &str, selector: &str) -> SnapshotDiff;
    /// Projects one declared kind and fact.
    fn project(&self, kind: &str, fact: &str) -> Project;
    /// Explains one declared conclusion and fact.
    fn explain(&self, conclusion: &str, fact: &str) -> Explain;
    /// Seals one caller-named resolver document.
    fn seal(&self, batch: &str) -> Seal;
    /// Names resolver fields that differ between two documents.
    fn seal_diff(&self, before: &str, after: &str) -> SealDiff;
}

/// One snapshot or a gap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// Gap reason when the batch or provider is missing.
    pub gap: Option<&'static str>,
    /// Observation status when `gap` is absent.
    pub status: String,
    /// Semantic value bound only to an observed row.
    pub value: Option<String>,
    /// Provider port that was read, empty for a gap.
    pub port: String,
    /// Selectors actually read.
    pub reads: Vec<String>,
    /// Effect calls performed by the read.
    pub effects: u64,
    /// Processes started by the read.
    pub subprocesses: u64,
    /// Whether the law contacted a network.
    pub network: bool,
}

/// Comparison of two snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotDiff {
    /// Gap reason when either batch or provider is missing.
    pub gap: Option<&'static str>,
    /// Whether the semantic rows differ.
    pub changed: bool,
    /// Selectors read while the rows were compared.
    pub reads: Vec<String>,
    /// Effect calls performed by the comparison.
    pub effects: u64,
}

/// Admission of one projection pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    /// Whether the kind and fact are declared.
    pub admitted: bool,
    /// Conclusion affected when the pair is declared.
    pub affected: Option<&'static str>,
}

/// Explanation of one declared dependency, or an unknown pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explain {
    /// Owner that declares the dependency.
    pub owner: Option<&'static str>,
    /// Declared conclusion.
    pub conclusion: Option<&'static str>,
    /// Declared fact.
    pub fact: Option<&'static str>,
}

/// Source admission for one document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seal {
    /// Gap reason when the document is not published.
    pub gap: Option<&'static str>,
    /// `admit` or `refuse` when the document exists.
    pub admission: &'static str,
    /// Refusal reason when admission is `refuse`.
    pub reason: Option<&'static str>,
    /// Whether a compiler was opened. The law keeps this false.
    pub compiler_opened: bool,
    /// Resolver domain ids.
    pub domains: Vec<String>,
    /// Member sources.
    pub sources: Vec<String>,
    /// Generated checksum, when the document has one.
    pub generated_checksum: Option<String>,
}

/// Field-level source diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealDiff {
    /// Gap reason when either document is missing.
    pub gap: Option<&'static str>,
    /// Whether any sealed field differs.
    pub changed: bool,
    /// Differing fields.
    pub inputs: Vec<SealChange>,
    /// Whether a compiler was opened.
    pub compiler_opened: bool,
}

/// One changed resolver field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealChange {
    /// Domain that owns the field.
    pub domain: String,
    /// Field family.
    pub origin: String,
    /// Field subject.
    pub name: String,
    /// Changed field.
    pub field: String,
}

/// The released foreign-project law.
#[derive(Clone, Copy, Debug, Default)]
pub struct CanonicalLaw;

impl PortabilityLaw for CanonicalLaw {
    fn snapshot(&self, batch: &str, selector: &str) -> Snapshot {
        let Some(rows) = rows_of(batch) else {
            return gap_snapshot("unknown-batch");
        };
        let Some((prefix, _)) = selector.split_once('/') else {
            return gap_snapshot("missing-capability");
        };
        if !rows.iter().any(|row| row.port == prefix) {
            return gap_snapshot("missing-capability");
        }
        let Some(row) = rows.iter().find(|row| row.selector == selector) else {
            return Snapshot {
                gap: None,
                status: "absent".to_owned(),
                value: None,
                port: prefix.to_owned(),
                reads: vec![selector.to_owned()],
                effects: 0,
                subprocesses: 0,
                network: false,
            };
        };
        Snapshot {
            gap: None,
            status: row.status.to_owned(),
            value: row.value.map(str::to_owned),
            port: row.port.to_owned(),
            reads: vec![selector.to_owned()],
            effects: 0,
            subprocesses: 0,
            network: false,
        }
    }

    fn snapshot_diff(&self, before: &str, after: &str, selector: &str) -> SnapshotDiff {
        let left = self.snapshot(before, selector);
        let right = self.snapshot(after, selector);
        if let Some(reason) = left.gap.or(right.gap) {
            return SnapshotDiff {
                gap: Some(reason),
                changed: false,
                reads: Vec::new(),
                effects: 0,
            };
        }
        let changed = left.status != right.status || left.value != right.value;
        SnapshotDiff {
            gap: None,
            changed,
            reads: if changed {
                [left.reads, right.reads].concat()
            } else {
                Vec::new()
            },
            effects: 0,
        }
    }

    fn project(&self, kind: &str, fact: &str) -> Project {
        match proof(kind, fact) {
            Some(row) => Project {
                admitted: true,
                affected: Some(row.conclusion),
            },
            None => Project {
                admitted: false,
                affected: None,
            },
        }
    }

    fn explain(&self, conclusion: &str, fact: &str) -> Explain {
        let Some(row) = PROOFS
            .iter()
            .find(|row| row.conclusion == conclusion && row.fact == fact)
        else {
            return Explain {
                owner: None,
                conclusion: None,
                fact: None,
            };
        };
        Explain {
            owner: Some(row.owner),
            conclusion: Some(row.conclusion),
            fact: Some(row.fact),
        }
    }

    fn seal(&self, batch: &str) -> Seal {
        let Some(document) = document(batch) else {
            return Seal {
                gap: Some("unknown-batch"),
                admission: "refuse",
                reason: Some("unknown-batch"),
                compiler_opened: false,
                domains: Vec::new(),
                sources: Vec::new(),
                generated_checksum: None,
            };
        };
        let (admission, reason) = admit(document.members);
        Seal {
            gap: None,
            admission,
            reason,
            compiler_opened: false,
            domains: document
                .domains
                .iter()
                .map(|domain| (*domain).to_owned())
                .collect(),
            sources: document
                .members
                .iter()
                .map(|member| member.source.to_owned())
                .collect(),
            generated_checksum: Some(document.generated_checksum.to_owned()),
        }
    }

    fn seal_diff(&self, before: &str, after: &str) -> SealDiff {
        let (Some(left), Some(right)) = (document(before), document(after)) else {
            return SealDiff {
                gap: Some("unknown-batch"),
                changed: false,
                inputs: Vec::new(),
                compiler_opened: false,
            };
        };
        let mut inputs = Vec::new();
        if left.generated_checksum != right.generated_checksum {
            inputs.push(SealChange {
                domain: left.generated_domain.to_owned(),
                origin: "generated".to_owned(),
                name: left.generated_name.to_owned(),
                field: "checksum".to_owned(),
            });
        }
        SealDiff {
            gap: None,
            changed: !inputs.is_empty(),
            inputs,
            compiler_opened: false,
        }
    }
}

/// Checks the exact requested portability scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-port", request, |request| match request.scope {
        "portability/foreign-project" => {
            crate::executor::require_subject(request.subject)?;
            crate::executor::require_support(request)?;
            crate::executor::foreign_project(request.subject)
        }
        "portability/consumer-seams" => consumer_seams::check_subject(request),
        _ => Err(failure("wrong-scope")),
    })
}

/// Runs the foreign-project scenarios through a public trait.
pub fn check_implementation(
    law: &dyn PortabilityLaw,
) -> Result<Vec<CheckObservation>, PackFailure> {
    custom_provider(law)?;
    shared_names(law)?;
    hostile_remote(law)?;
    nested_and_generated(law)?;
    proofs(law)?;
    admission(law)?;
    Ok(vec![CheckObservation {
        key: "portability.foreign-project".to_owned(),
        value: "provider,nested,generated,proof,remote,membrane,admission".to_owned(),
    }])
}

fn custom_provider(law: &dyn PortabilityLaw) -> Result<(), PackFailure> {
    let row = law.snapshot("lantern", "provider/widget");
    if row.gap.is_some()
        || row.status != "observed"
        || row.value.as_deref() != Some("widget-body")
        || row.port != "provider"
        || row.reads != ["provider/widget"]
        || row.effects != 0
        || row.subprocesses != 0
        || row.network
    {
        return Err(failure("custom-provider"));
    }
    let generated = law.snapshot("lantern", "provider/generated");
    if generated.value.as_deref() != Some("generated-widget") || generated.port != "provider" {
        return Err(failure("generated-output"));
    }
    let missing = law.snapshot("lantern", "membrane/linux");
    if missing.gap != Some("missing-capability") || !missing.reads.is_empty() || missing.network {
        return Err(failure("missing-capability"));
    }
    Ok(())
}

fn shared_names(law: &dyn PortabilityLaw) -> Result<(), PackFailure> {
    let left = law.snapshot("lantern", "provider/widget");
    let right = law.snapshot("lantern-alias", "provider/widget");
    if left.value != right.value || left.status != right.status {
        return Err(failure("project-name-branch"));
    }
    let same = law.snapshot_diff("lantern", "lantern-alias", "provider/widget");
    if same.gap.is_some() || same.changed || !same.reads.is_empty() || same.effects != 0 {
        return Err(failure("project-name-branch"));
    }
    let changed = law.snapshot_diff("lantern", "lantern-revised", "provider/widget");
    if !changed.changed || changed.reads != ["provider/widget", "provider/widget"] {
        return Err(failure("lifecycle"));
    }
    Ok(())
}

fn hostile_remote(law: &dyn PortabilityLaw) -> Result<(), PackFailure> {
    let row = law.snapshot("lantern", "remote/hostile");
    if row.gap.is_some()
        || row.status != "unavailable"
        || row.value.is_some()
        || row.port != "remote"
        || row.network
        || row.effects != 0
        || row.subprocesses != 0
    {
        return Err(failure("hostile-remote"));
    }
    Ok(())
}

fn nested_and_generated(law: &dyn PortabilityLaw) -> Result<(), PackFailure> {
    let sealed = law.seal("lantern");
    if sealed.gap.is_some()
        || !sealed
            .domains
            .iter()
            .any(|domain| domain == "resolver/lantern")
        || !sealed
            .domains
            .iter()
            .any(|domain| domain == "resolver/lantern/vendor/widgets")
        || !sealed
            .sources
            .iter()
            .any(|source| source == "crates/lantern")
        || !sealed
            .sources
            .iter()
            .any(|source| source == "vendor/widgets/widget")
        || sealed
            .sources
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 2
    {
        return Err(failure("nested-root"));
    }
    if sealed.generated_checksum.as_deref() != Some("aaaa") {
        return Err(failure("generated-output"));
    }
    Ok(())
}

fn proofs(law: &dyn PortabilityLaw) -> Result<(), PackFailure> {
    let projected = law.project("world/generated-ownership-v1", "fact/lantern-generated");
    if !projected.admitted || projected.affected != Some("conclusion/lantern-proof") {
        return Err(failure("proof-declaration"));
    }
    let explained = law.explain("conclusion/lantern-proof", "fact/lantern-source");
    if explained.owner != Some("lantern/widget")
        || explained.conclusion != Some("conclusion/lantern-proof")
        || explained.fact != Some("fact/lantern-source")
    {
        return Err(failure("proof-declaration"));
    }
    let unknown = law.explain("conclusion/stranger", "fact/lantern-source");
    if unknown.owner.is_some() {
        return Err(failure("unknown-owner"));
    }
    let unsupported = law.project("no-such/kind", "no-such/fact");
    if unsupported.admitted {
        return Err(failure("unsupported-pair"));
    }
    Ok(())
}

fn admission(law: &dyn PortabilityLaw) -> Result<(), PackFailure> {
    let admitted = law.seal("lantern");
    if admitted.admission != "admit" || admitted.reason.is_some() || admitted.compiler_opened {
        return Err(failure("source-admission"));
    }
    let stub = law.seal("lantern-stub");
    if stub.admission != "refuse" || stub.reason != Some("stub-only") || stub.compiler_opened {
        return Err(failure("stub-refusal"));
    }
    let same = law.seal_diff("lantern", "lantern-alias");
    if same.gap.is_some() || same.changed || !same.inputs.is_empty() || same.compiler_opened {
        return Err(failure("unchanged-document"));
    }
    let changed = law.seal_diff("lantern", "lantern-revised");
    if changed.compiler_opened || changed.inputs.len() != 1 {
        return Err(failure("generated-invalidation"));
    }
    let input = &changed.inputs[0];
    if input.field != "checksum"
        || input.origin != "generated"
        || input.name != "src/generated.rs"
        || input.domain != "resolver/lantern"
    {
        return Err(failure("generated-invalidation"));
    }
    let unknown = law.seal("no-such");
    if unknown.gap != Some("unknown-batch") || unknown.compiler_opened {
        return Err(failure("unknown-batch"));
    }
    Ok(())
}

fn failure(code: &'static str) -> PackFailure {
    PackFailure {
        code,
        detail: code.to_owned(),
    }
}

fn gap_snapshot(reason: &'static str) -> Snapshot {
    Snapshot {
        gap: Some(reason),
        status: String::new(),
        value: None,
        port: String::new(),
        reads: Vec::new(),
        effects: 0,
        subprocesses: 0,
        network: false,
    }
}

struct ObservationRow {
    port: &'static str,
    selector: &'static str,
    status: &'static str,
    value: Option<&'static str>,
}

struct ProofRow {
    kind: &'static str,
    fact: &'static str,
    owner: &'static str,
    conclusion: &'static str,
}

struct MemberRow {
    source: &'static str,
    origin: &'static str,
    checksum: &'static str,
    kind: &'static str,
    compiling: bool,
}

struct Document {
    members: &'static [MemberRow],
    domains: &'static [&'static str],
    generated_domain: &'static str,
    generated_name: &'static str,
    generated_checksum: &'static str,
}

const WIDGET: &[ObservationRow] = &[
    ObservationRow {
        port: "provider",
        selector: "provider/widget",
        status: "observed",
        value: Some("widget-body"),
    },
    ObservationRow {
        port: "provider",
        selector: "provider/generated",
        status: "observed",
        value: Some("generated-widget"),
    },
    ObservationRow {
        port: "remote",
        selector: "remote/hostile",
        status: "unavailable",
        value: None,
    },
];

const WIDGET_REVISED: &[ObservationRow] = &[
    ObservationRow {
        port: "provider",
        selector: "provider/widget",
        status: "observed",
        value: Some("widget-next"),
    },
    ObservationRow {
        port: "provider",
        selector: "provider/generated",
        status: "observed",
        value: Some("generated-widget"),
    },
    ObservationRow {
        port: "remote",
        selector: "remote/hostile",
        status: "unavailable",
        value: None,
    },
];

const PROOFS: &[ProofRow] = &[
    ProofRow {
        kind: "world/package-assembly-v1",
        fact: "fact/lantern-source",
        owner: "lantern/widget",
        conclusion: "conclusion/lantern-proof",
    },
    ProofRow {
        kind: "world/generated-ownership-v1",
        fact: "fact/lantern-generated",
        owner: "lantern/widget",
        conclusion: "conclusion/lantern-proof",
    },
];

const COMPLETE: &[MemberRow] = &[
    MemberRow {
        source: "crates/lantern",
        origin: "path",
        checksum: "abcdef",
        kind: "path-content",
        compiling: true,
    },
    MemberRow {
        source: "vendor/widgets/widget",
        origin: "path",
        checksum: "123456",
        kind: "path-content",
        compiling: false,
    },
];

const STUB: &[MemberRow] = &[
    MemberRow {
        source: "crates/lantern",
        origin: "stub",
        checksum: "-",
        kind: "metadata",
        compiling: true,
    },
    MemberRow {
        source: "vendor/widgets/widget",
        origin: "path",
        checksum: "123456",
        kind: "path-content",
        compiling: false,
    },
];

const DOMAINS: &[&str] = &["resolver/lantern", "resolver/lantern/vendor/widgets"];

fn rows_of(batch: &str) -> Option<&'static [ObservationRow]> {
    match batch {
        "lantern" | "lantern-alias" => Some(WIDGET),
        "lantern-revised" => Some(WIDGET_REVISED),
        _ => None,
    }
}

fn document(batch: &str) -> Option<Document> {
    let (members, generated_checksum) = match batch {
        "lantern" | "lantern-alias" => (COMPLETE, "aaaa"),
        "lantern-revised" => (COMPLETE, "bbbb"),
        "lantern-stub" => (STUB, "aaaa"),
        _ => return None,
    };
    Some(Document {
        members,
        domains: DOMAINS,
        generated_domain: "resolver/lantern",
        generated_name: "src/generated.rs",
        generated_checksum,
    })
}

fn proof(kind: &str, fact: &str) -> Option<&'static ProofRow> {
    PROOFS
        .iter()
        .find(|row| row.kind == kind && row.fact == fact)
}

fn admit(members: &[MemberRow]) -> (&'static str, Option<&'static str>) {
    for member in members {
        if member.origin == "stub" {
            return ("refuse", Some("stub-only"));
        }
        if member.checksum == "-" || member.kind == "none" {
            return ("refuse", Some("missing-input"));
        }
    }
    if !members.iter().any(|member| member.compiling) {
        return ("refuse", Some("absent-input"));
    }
    ("admit", None)
}
