// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_packs::{
    MemorySubject, PackRequest, PackVerdict, find_pack, packs,
    packs::portability::{
        CanonicalLaw, Explain, PortabilityLaw, Project, Seal, SealDiff, Snapshot, SnapshotDiff,
    },
};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn request<'a>(scope: &'a str, evidence: &'a MemorySubject) -> PackRequest<'a> {
    let binding = find_pack("checker/c-port")
        .unwrap()
        .checker_binding()
        .unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    PackRequest {
        checker: "checker/c-port",
        binding,
        subject: SUBJECT,
        scope,
        evidence,
    }
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

#[test]
fn foreign_project_scope_passes_and_consumer_seams_need_a_freeze() {
    let evidence = MemorySubject::default();
    assert!(matches!(
        packs::portability::check(&request("portability/foreign-project", &evidence)),
        PackVerdict::Pass { .. }
    ));
    assert!(matches!(
        packs::portability::check(&request("portability/consumer-seams", &evidence)),
        PackVerdict::Refused(ref failure) if failure.code == "missing-freeze"
    ));
    assert!(matches!(
        packs::portability::check(&request("portability/not-a-scope", &evidence)),
        PackVerdict::Refused(ref failure) if failure.code == "wrong-scope"
    ));
}

#[test]
fn a_project_name_branch_and_a_compiler_are_refused() {
    assert_eq!(
        packs::portability::check_implementation(&NameBranch)
            .unwrap_err()
            .code,
        "project-name-branch"
    );
    assert_eq!(
        packs::portability::check_implementation(&CompilerLaw)
            .unwrap_err()
            .code,
        "source-admission"
    );
}

struct NameBranch;

impl PortabilityLaw for NameBranch {
    fn snapshot(&self, batch: &str, selector: &str) -> Snapshot {
        let mut row = CanonicalLaw.snapshot(batch, selector);
        if batch == "lantern-alias" {
            row.value = Some("branch-value".to_owned());
        }
        row
    }

    fn snapshot_diff(&self, before: &str, after: &str, selector: &str) -> SnapshotDiff {
        CanonicalLaw.snapshot_diff(before, after, selector)
    }

    fn project(&self, kind: &str, fact: &str) -> Project {
        CanonicalLaw.project(kind, fact)
    }

    fn explain(&self, conclusion: &str, fact: &str) -> Explain {
        CanonicalLaw.explain(conclusion, fact)
    }

    fn seal(&self, batch: &str) -> Seal {
        CanonicalLaw.seal(batch)
    }

    fn seal_diff(&self, before: &str, after: &str) -> SealDiff {
        CanonicalLaw.seal_diff(before, after)
    }
}

struct CompilerLaw;

impl PortabilityLaw for CompilerLaw {
    fn snapshot(&self, batch: &str, selector: &str) -> Snapshot {
        CanonicalLaw.snapshot(batch, selector)
    }

    fn snapshot_diff(&self, before: &str, after: &str, selector: &str) -> SnapshotDiff {
        CanonicalLaw.snapshot_diff(before, after, selector)
    }

    fn project(&self, kind: &str, fact: &str) -> Project {
        CanonicalLaw.project(kind, fact)
    }

    fn explain(&self, conclusion: &str, fact: &str) -> Explain {
        CanonicalLaw.explain(conclusion, fact)
    }

    fn seal(&self, batch: &str) -> Seal {
        let mut sealed = CanonicalLaw.seal(batch);
        sealed.compiler_opened = true;
        sealed
    }

    fn seal_diff(&self, before: &str, after: &str) -> SealDiff {
        CanonicalLaw.seal_diff(before, after)
    }
}
