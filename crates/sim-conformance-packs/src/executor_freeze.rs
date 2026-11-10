// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Compiled digest of the executor closure this package calls.
//!
//! The bytes are the source the pack is compiled against. A runtime walk of
//! the same paths is the outside freeze check. This module does not read the
//! filesystem.

use sha2::{Digest, Sha256};

/// Why a closure listing cannot be hashed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosureFault {
    /// The listing had no files.
    Empty,
    /// A path is empty, absolute, or escapes its root.
    BadPath,
    /// The same path appeared twice.
    Duplicate,
}

macro_rules! closure_files {
    ($($path:literal),+ $(,)?) => {
        /// Repository-relative files whose bytes are the frozen executor.
        pub const EXECUTOR_CLOSURE_PATHS: &[&str] = &[$($path),+];

        const COMPILED: &[(&str, &[u8])] = &[
            $(($path, include_bytes!(concat!("../../../../", $path)))),+
        ];
    };
}

closure_files!(
    "sim-agent-net/Cargo.lock",
    "sim-agent-net/crates/sim-lib-roadmap-runner/Cargo.toml",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/accept.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/author.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/author_check.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/author_compile.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/author_ops.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/author_parse.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/binding.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/budget.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/causal.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/codec.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/compatibility.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/conduct.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/converge.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/converge_close.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/converge_propose.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/discharge.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/drive.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/drive_commit.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/drive_step.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/fixed.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/fixed_check.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/fixed_identity.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/implementer.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/journal.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/journal_delta.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/journal_replay.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/journal_share.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/journal_share_tree.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/journal_util.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/land.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/lib.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/local_command.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/model.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/mutation.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/mutation/fs_workspace.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/packet_work.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/packet_work_build.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/packet_work_check.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/packet_work_decode.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/proof.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/proof/error.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/proof/evidence.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/proof/evidence_bind.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/proof/evidence_discover.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/proof/normalization.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/proof/receipt.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/reconcile.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/record_law.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/recovery.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/refiner.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/schedule.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/schedule_detail.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/schedule_ops.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/schedule_session.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/selected.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/selected_bind.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/selected_effect.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/selected_mem.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/service.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/source.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/status.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/transact.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/transact_check.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/transact_preview.rs",
    "sim-agent-net/crates/sim-lib-roadmap-runner/src/transact_store.rs",
    "sim-agent-net/crates/sim-roadmap-core/Cargo.toml",
    "sim-agent-net/crates/sim-roadmap-core/src/admit.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/completion.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/datum.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/datum_guide.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/id.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/inheritance.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/lib.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/path.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/tree.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/types.rs",
    "sim-agent-net/crates/sim-roadmap-core/src/validation.rs",
    "sim-agent-net/crates/sim-roadmap-exec-core/Cargo.toml",
    "sim-agent-net/crates/sim-roadmap-exec-core/src/card.rs",
    "sim-agent-net/crates/sim-roadmap-exec-core/src/failure.rs",
    "sim-agent-net/crates/sim-roadmap-exec-core/src/lib.rs",
    "sim-agent-net/crates/sim-roadmap-exec-core/src/model.rs",
    "sim-agent-net/crates/sim-roadmap-exec-core/src/recovery.rs",
    "sim-agent-net/crates/sim-roadmap-exec-core/src/reduce.rs",
    "sim-agent-net/crates/sim-source-deck/Cargo.toml",
    "sim-agent-net/crates/sim-source-deck/src/build.rs",
    "sim-agent-net/crates/sim-source-deck/src/canonical.rs",
    "sim-agent-net/crates/sim-source-deck/src/lib.rs",
    "sim-agent-net/crates/sim-source-deck/src/types.rs",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/Cargo.toml",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/src/journal.rs",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/src/lib.rs",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/src/lifecycle.rs",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/src/lifecycle_tests.rs",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/src/records.rs",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/src/sha256.rs",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/src/symbols.rs",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/src/tests.rs",
    "sim-agent-net/crates/sim-lib-agent-conduct-core/src/usage.rs",
    "sim-agent-net/crates/sim-roadmap-plan/Cargo.toml",
    "sim-agent-net/crates/sim-roadmap-plan/src/compile.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/completion.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/explain.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/ground.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/invalidation.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/key.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/lib.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/projection.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/promise.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/query.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/readiness.rs",
    "sim-agent-net/crates/sim-roadmap-plan/src/tests.rs",
    "sim-agent-net/crates/sim-roadmap-refine/Cargo.toml",
    "sim-agent-net/crates/sim-roadmap-refine/src/apply.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/atomicity.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/certificate.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/diff.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/lib.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/policy.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/profile.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/proposal.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/rank.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/refusal.rs",
    "sim-agent-net/crates/sim-roadmap-refine/src/tests.rs",
);

/// Stable label of the workspace resolver lock inside the executor identity.
pub const WORKSPACE_RESOLVER_LOCK: &str = "resolver/workspace-cargo.lock";

const WORKSPACE_RESOLVER_BYTES: &[u8] = include_bytes!("../../../Cargo.lock");

/// Hex digest of the compiled executor, including the workspace resolver lock.
pub fn compiled_executor_closure() -> String {
    let mut entries = COMPILED.to_vec();
    entries.push((WORKSPACE_RESOLVER_LOCK, WORKSPACE_RESOLVER_BYTES));
    hash_closure(&entries).expect("compiled executor closure is well formed")
}

/// Hashes sorted closure entries. Paths stay repository-relative.
pub fn hash_closure(entries: &[(&str, &[u8])]) -> Result<String, ClosureFault> {
    if entries.is_empty() {
        return Err(ClosureFault::Empty);
    }
    let mut rows = entries.to_vec();
    rows.sort_by(|left, right| left.0.cmp(right.0));
    let mut hasher = Sha256::new();
    let mut previous = "";
    for (path, bytes) in rows {
        if path.is_empty()
            || path.starts_with('/')
            || path.contains('\0')
            || path.split('/').any(|part| part == "..")
        {
            return Err(ClosureFault::BadPath);
        }
        if path == previous {
            return Err(ClosureFault::Duplicate);
        }
        previous = path;
        hasher.update((path.len() as u64).to_be_bytes());
        hasher.update(path.as_bytes());
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
