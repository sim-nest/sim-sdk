// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Advice judged by the frozen executor's proof catalog.
//!
//! A shell string never becomes a catalog leaf. Artifact equality is the
//! authoritative verifier and does not launch a process.

use std::collections::{BTreeMap, BTreeSet};

use sim_lib_exec::{
    ProcessCancellation, SandboxAttempt, SandboxLauncher, SandboxLimits, SandboxRequest,
};
use sim_lib_roadmap_runner::{
    CommandProof, ProofCatalog, ProofDisposition, ProofLeaf, ProofOutputId, StructuredExpectation,
    execute_proof_leaf,
};

use super::SeamFault;

/// Refuses a shell command and judges advice bytes through artifact equality.
pub(super) fn catalog_refuses_advice_as_proof() -> Result<(), SeamFault> {
    match ProofCatalog::new([ProofLeaf::command(shell_command())]) {
        Err(error)
            if error
                .to_string()
                .contains("program must be an opaque tool id, not a shell string") => {}
        Err(error) => panic!("shell catalog: {error}"),
        Ok(_) => return Err(SeamFault::AdviceRejected),
    }
    let launcher = RefuseLaunch;
    let cancel = ProcessCancellation::default();
    let passed = execute_proof_leaf(
        &catalog(b"advice", b"advice"),
        "leaf/equal",
        &launcher,
        &cancel,
        "observed",
    )
    .unwrap_or_else(|error| panic!("equal leaf: {error}"));
    let failed = execute_proof_leaf(
        &catalog(b"advice", b"decision"),
        "leaf/equal",
        &launcher,
        &cancel,
        "observed",
    )
    .unwrap_or_else(|error| panic!("unequal leaf: {error}"));
    if passed.disposition == ProofDisposition::Passed
        && failed.disposition == ProofDisposition::Failed
    {
        Ok(())
    } else {
        Err(SeamFault::AdviceRejected)
    }
}

fn catalog(left: &[u8], right: &[u8]) -> ProofCatalog {
    ProofCatalog::new([ProofLeaf::ArtifactEquality {
        name: "leaf/equal".into(),
        left: left.to_vec(),
        right: right.to_vec(),
    }])
    .unwrap_or_else(|error| panic!("equality catalog: {error}"))
}

fn shell_command() -> CommandProof {
    CommandProof {
        name: "leaf/shell".into(),
        effect_id: "effect-shell".into(),
        program: "cargo test".into(),
        argv: vec!["--literal".into()],
        working_directory: "/source".into(),
        environment: BTreeMap::new(),
        allowed_environment_keys: BTreeSet::new(),
        source_mount: "checkout:exact-head".into(),
        scratch_mount: Some("scratch:proof".into()),
        source_read_only: true,
        limits: SandboxLimits {
            cpu_seconds: 1,
            memory_bytes: 32 * 1024 * 1024,
            wall_time_ms: 100,
            process_count: 2,
            file_count: 2,
            file_bytes: 1024,
            output_bytes: 1024,
            stdin_bytes: 1,
        },
        expected: StructuredExpectation {
            stdout: ProofOutputId::sha256(b"advice"),
            exit_code: 0,
        },
    }
}

struct RefuseLaunch;

impl SandboxLauncher for RefuseLaunch {
    fn id(&self) -> &str {
        "refuse"
    }

    fn launch(&self, _: &SandboxRequest, _: &ProcessCancellation) -> SandboxAttempt {
        panic!("artifact equality consulted the launcher")
    }
}
