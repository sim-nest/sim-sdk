// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Static checker, phase, and scope catalog.

use std::collections::BTreeSet;

use sim_conformance_core::{
    CheckArgument, CheckScopeId, CheckTemplate, CheckerBinding, CommandId, ConformanceError,
    ConformancePackId, EnvironmentPolicyId, OutputShapeId, OwnerBindingId, ProofCodeId,
    WorkingDirectoryPolicyId,
};

const ACTIVATION_OWNER_BINDING: &str =
    "core/sha256-datum-v1:6fc0e9594fe3a592c1877d9034cc7a6b5f9aa9ce50e3251a705955b348ccf827";

/// One statically registered checker pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackSpec {
    /// Stable checker id.
    pub checker: &'static str,
    /// Exact immutable NV12.00 activation-record id from which the runtime binding is derived.
    pub activation_binding: &'static str,
    /// Stable pack id.
    pub pack: &'static str,
    /// Phase that first funds the entrypoint.
    pub producing_phase: &'static str,
    /// Every scope the static checker binding permits.
    pub allowed_scopes: &'static [&'static str],
    /// Scopes whose scenarios are implemented in this crate version.
    pub implemented_scopes: &'static [&'static str],
}

impl PackSpec {
    /// Returns the exact immutable published checker-code identity.
    pub fn checker_code_id(&self) -> Result<ProofCodeId, ConformanceError> {
        self.checker_code_id_for_build_inputs(env!("SIM_CONFORMANCE_PACKS_BUILD_INPUTS_SHA256"))
    }

    fn checker_code_id_for_build_inputs(
        &self,
        build_inputs_sha256: &str,
    ) -> Result<ProofCodeId, ConformanceError> {
        if build_inputs_sha256.len() != 64
            || !build_inputs_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(ConformanceError::UnresolvedBinding);
        }
        ProofCodeId::from_text(&format!(
            "sim-conformance-packs@{};build-inputs-sha256={build_inputs_sha256}",
            env!("CARGO_PKG_VERSION")
        ))
    }

    /// Returns the SHA-256 identity of the exact source, dependency lock,
    /// toolchain, target, feature, and compiler configuration inputs used to
    /// build this installed checker.
    pub const fn build_inputs_sha256(&self) -> &'static str {
        env!("SIM_CONFORMANCE_PACKS_BUILD_INPUTS_SHA256")
    }

    /// Returns the exact dependency-lock digest selected for this build.
    pub const fn dependency_lock_sha256(&self) -> &'static str {
        env!("SIM_CONFORMANCE_PACKS_DEPENDENCY_LOCK_SHA256")
    }

    /// Returns the typed pack id bound to this exact activation record.
    pub fn pack_id(&self) -> Result<ConformancePackId, ConformanceError> {
        ConformancePackId::from_text(&format!("{}@{}", self.pack, self.activation_binding))
    }

    /// Constructs the typed runtime binding derived from this frozen activation record.
    pub fn checker_binding(&self) -> Result<CheckerBinding, ConformanceError> {
        let allowed_scopes = self
            .allowed_scopes
            .iter()
            .map(|scope| CheckScopeId::from_text(scope))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let template = CheckTemplate::new(
            "cargo".into(),
            vec![
                CheckArgument::Literal("run".into()),
                CheckArgument::Literal("--manifest-path".into()),
                CheckArgument::Literal("../sim-tooling/Cargo.toml".into()),
                CheckArgument::Literal("-p".into()),
                CheckArgument::Literal("xtask".into()),
                CheckArgument::Literal("--".into()),
                CheckArgument::Literal("check-pack".into()),
                CheckArgument::Literal("--checker".into()),
                CheckArgument::Literal(self.checker.into()),
                CheckArgument::Literal("--binding".into()),
                CheckArgument::BindingSlot,
                CheckArgument::Literal("--subject".into()),
                CheckArgument::SubjectSlot,
                CheckArgument::Literal("--scope".into()),
                CheckArgument::ScopeSlot,
            ],
            WorkingDirectoryPolicyId::from_text("command/sim-private-root")?,
            EnvironmentPolicyId::from_text("command/sealed-rust-toolchain")?,
            OutputShapeId::from_text("check/result-v1")?,
        )?;
        CheckerBinding::new(
            self.checker.into(),
            OwnerBindingId::from_text(ACTIVATION_OWNER_BINDING)?,
            self.public_symbol()?.into(),
            vec![self.pack_id()?],
            OutputShapeId::from_text("check/receipt-v1")?,
            crate::owner_revocation_source()?,
            CommandId::from_text(
                "cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo doc --workspace --no-deps && cargo clippy --workspace --all-features --all-targets -- -D warnings && cargo test --workspace --all-features",
            )?,
            CommandId::from_text("cargo run -p xtask -- simdoc --check")?,
            allowed_scopes,
            template,
        )
    }

    /// Constructs the distinct binding for an in-process native Rust call.
    ///
    /// The native binding is currently funded only for C-OP's
    /// `operation/local` bootstrap join. It must not be substituted for the
    /// maintenance command binding: each binding records a different call that
    /// actually ran and therefore has a different identity.
    pub fn native_operation_local_binding(&self) -> Result<CheckerBinding, ConformanceError> {
        if self.checker != "checker/c-op"
            || !self.allowed_scopes.contains(&"operation/local")
            || !self.implemented_scopes.contains(&"operation/local")
        {
            return Err(ConformanceError::UnauthorizedScope);
        }
        let scope = CheckScopeId::from_text("operation/local")?;
        let template = CheckTemplate::new(
            "sim_check_pack::invoke_operation_local".into(),
            vec![
                CheckArgument::Literal(self.checker.into()),
                CheckArgument::BindingSlot,
                CheckArgument::SubjectSlot,
                CheckArgument::ScopeSlot,
            ],
            WorkingDirectoryPolicyId::from_text("call/no-working-directory")?,
            EnvironmentPolicyId::from_text("call/native-rust-v1")?,
            OutputShapeId::from_text("check/result-v1")?,
        )?;
        CheckerBinding::new(
            self.checker.into(),
            OwnerBindingId::from_text(ACTIVATION_OWNER_BINDING)?,
            self.public_symbol()?.into(),
            vec![self.pack_id()?],
            OutputShapeId::from_text("check/receipt-v1")?,
            crate::owner_revocation_source()?,
            CommandId::from_text(
                "cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo doc --workspace --no-deps && cargo clippy --workspace --all-features --all-targets -- -D warnings && cargo test --workspace --all-features",
            )?,
            CommandId::from_text("cargo run -p xtask -- simdoc --check")?,
            BTreeSet::from([scope]),
            template,
        )
    }

    /// Constructs the distinct native binding for projection admission.
    ///
    /// This is a C-BOUNDARY call and deliberately does not widen C-OP's sole
    /// `operation/local` native authority.
    pub fn native_projection_binding(&self) -> Result<CheckerBinding, ConformanceError> {
        if self.checker != "checker/c-boundary"
            || !self
                .allowed_scopes
                .contains(&"boundary/projection-admission")
            || !self
                .implemented_scopes
                .contains(&"boundary/projection-admission")
        {
            return Err(ConformanceError::UnauthorizedScope);
        }
        let scope = CheckScopeId::from_text("boundary/projection-admission")?;
        let template = CheckTemplate::new(
            "sim_check_pack::invoke_projection_admission".into(),
            vec![
                CheckArgument::Literal(self.checker.into()),
                CheckArgument::BindingSlot,
                CheckArgument::SubjectSlot,
                CheckArgument::ScopeSlot,
            ],
            WorkingDirectoryPolicyId::from_text("call/no-working-directory")?,
            EnvironmentPolicyId::from_text("call/native-rust-v1")?,
            OutputShapeId::from_text("check/result-v1")?,
        )?;
        CheckerBinding::new(
            self.checker.into(),
            OwnerBindingId::from_text(ACTIVATION_OWNER_BINDING)?,
            self.public_symbol()?.into(),
            vec![self.pack_id()?],
            OutputShapeId::from_text("check/receipt-v1")?,
            crate::owner_revocation_source()?,
            CommandId::from_text(
                "cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo doc --workspace --no-deps && cargo clippy --workspace --all-features --all-targets -- -D warnings && cargo test --workspace --all-features",
            )?,
            CommandId::from_text("cargo run -p xtask -- simdoc --check")?,
            BTreeSet::from([scope]),
            template,
        )
    }

    fn public_symbol(&self) -> Result<&'static str, ConformanceError> {
        Ok(match self.checker {
            "checker/c-v3" => "sim_conformance_packs::packs::retirement::check",
            "checker/c-id" => "sim_conformance_packs::packs::identity::check",
            "checker/c-own" => "sim_conformance_packs::packs::ownership::check",
            "checker/c-boundary" => "sim_conformance_packs::packs::boundary::check",
            "checker/c-source" => "sim_conformance_packs::packs::source::check",
            "checker/c-evidence" => "sim_conformance_packs::packs::evidence::check",
            "checker/c-op" => "sim_conformance_packs::packs::operation::check",
            "checker/c-journal" => "sim_conformance_packs::packs::journal::check",
            "checker/c-closure" => "sim_conformance_packs::packs::closure::check",
            "checker/c-control" => "sim_conformance_packs::packs::control::check",
            "checker/c-work" => "sim_conformance_packs::packs::work::check",
            "checker/c-drive" => "sim_conformance_packs::packs::drive::check",
            "checker/c-converge" => "sim_conformance_packs::packs::convergence::check",
            "checker/c-facet" => "sim_conformance_packs::packs::facet::check",
            "checker/c-disclose" => "sim_conformance_packs::packs::disclosure::check",
            "checker/c-deliver" => "sim_conformance_packs::packs::delivery::check",
            "checker/c-author" => "sim_conformance_packs::packs::authoring::check",
            "checker/c-port" => "sim_conformance_packs::packs::portability::check",
            "checker/c-product" => "sim_conformance_packs::packs::product::check",
            "checker/c-release" => "sim_conformance_packs::packs::release::check",
            "checker/c-succeed" => "sim_conformance_packs::packs::succession::check",
            _ => return Err(ConformanceError::UnresolvedBinding),
        })
    }

    /// Returns the first phase that funds scenarios for one declared scope.
    pub fn funded_phase(&self, scope: &str) -> String {
        if let Some(suffix) = scope.strip_prefix("release/nv12-") {
            return format!("NV12.{suffix}");
        }
        match scope {
            "retirement/final" => "NV12.27",
            "identity/journal-normalized" => "NV12.02",
            "identity/closure-final" | "identity/store-roundtrip" => "NV12.03",
            "identity/world-seal" => "NV12.10",
            "identity/evidence" => "NV12.12",
            "identity/fixed-point" => "NV12.16",
            "identity/final-seal" => "NV12.26",
            "ownership/dependencies" => "NV12.03",
            "ownership/bootstrap" => "NV12.05",
            "ownership/produced" => "NV12.06",
            "ownership/roadmap-final" => "NV12.28",
            "boundary/identity-closure" => "NV12.03",
            "boundary/local-adapter" => "NV12.05",
            "boundary/projection-admission" => "NV12.06",
            "boundary/observation" => "NV12.09",
            "boundary/source-closure" => "NV12.10",
            "boundary/composed" => "NV12.26",
            "source/composed" => "NV12.26",
            "evidence/predicted-closure" => "NV12.07",
            "evidence/identity-grade" => "NV12.12",
            "evidence/cold-oracle" => "NV12.14",
            "evidence/convergence" => "NV12.15",
            "evidence/composed" => "NV12.26",
            "operation/log" => "NV12.04",
            "operation/reconcile" | "operation/local" => "NV12.05",
            "operation/delivery-adapters" => "NV12.21",
            "operation/composed" => "NV12.26",
            "journal/composed" => "NV12.26",
            "closure/plan" => "NV12.08",
            "closure/frontier" | "closure/resources" => "NV12.13",
            "closure/composed" => "NV12.26",
            "control/composed" => "NV12.26",
            "work/packet-effects" => "NV12.05",
            "work/convergence" => "NV12.15",
            "work/native-dispatch" => "NV12.23",
            "work/composed" => "NV12.26",
            "drive/composed" => "NV12.26",
            "convergence/fixed-point" => "NV12.16",
            "convergence/composed" => "NV12.26",
            "facet/generated" => "NV12.16",
            "facet/transaction" => "NV12.17",
            "facet/landing" => "NV12.18",
            "facet/composed" => "NV12.26",
            "disclosure/transaction" => "NV12.17",
            "disclosure/landing" => "NV12.18",
            "disclosure/composed" => "NV12.26",
            "delivery/observation" => "NV12.19",
            "delivery/plan" => "NV12.20",
            "delivery/adapters" => "NV12.21",
            "delivery/real-canary" => "NV12.22",
            "delivery/composed" => "NV12.26",
            "authoring/family" => "NV12.24",
            "authoring/composed" => "NV12.26",
            "portability/consumer-seams" => "NV12.26",
            "product/prove-plan" => "NV12.08",
            "product/prove" => "NV12.14",
            "product/convergence-queries" => "NV12.15",
            "product/landing-queries" => "NV12.18",
            "product/deliver" => "NV12.20",
            "product/roadmap" => "NV12.25",
            "product/composed" => "NV12.26",
            "product/installed" => "NV12.27",
            "succession/composed-precheck" => "NV12.27",
            "succession/cold-change" => "NV12.28",
            _ => self.producing_phase,
        }
        .into()
    }
}

#[path = "catalog_table.rs"]
mod catalog_table;

use catalog_table::PACKS;

/// Returns every registered checker in stable catalog order.
pub const fn all_packs() -> &'static [PackSpec] {
    PACKS
}

/// Looks up one exact checker id.
pub fn find_pack(checker: &str) -> Option<&'static PackSpec> {
    PACKS.iter().find(|spec| spec.checker == checker)
}

#[cfg(test)]
mod tests {
    use sim_conformance_core::{
        CheckInputClosureId, CheckScopeId, CheckedSubjectId, CheckerReceipt,
        CheckerRevocationDecision, CheckerRevocationHeadId, CheckerRevocationKey,
        CheckerRevocationSet, EvidenceGrade, EvidenceProvenanceId, EvidenceSetId, IdKind, PolicyId,
        RevocationStatus, SemanticId,
    };

    use super::*;

    fn sid<K: IdKind>(value: &str) -> SemanticId<K> {
        SemanticId::from_text(value).unwrap()
    }

    #[test]
    fn same_version_source_substitution_changes_code_and_invocation_identity() {
        let spec = find_pack("checker/c-op").unwrap();
        let source_a = spec
            .checker_code_id_for_build_inputs(&"a".repeat(64))
            .unwrap();
        let source_b = spec
            .checker_code_id_for_build_inputs(&"b".repeat(64))
            .unwrap();
        assert_ne!(source_a, source_b);
        let binding = spec.checker_binding().unwrap();
        let subject: CheckedSubjectId = sid("subject/same");
        let scope: CheckScopeId = sid("operation/local");
        let closure: CheckInputClosureId = sid("closure/same");
        let instantiate = |code| {
            binding
                .instantiate(
                    code,
                    spec.pack_id().unwrap(),
                    subject.clone(),
                    scope.clone(),
                    closure.clone(),
                )
                .unwrap()
        };
        let invocation_a = instantiate(source_a);
        let invocation_b = instantiate(source_b);
        assert_ne!(invocation_a.id(), invocation_b.id());

        let issue = |invocation: &sim_conformance_core::CheckInvocation| {
            let key = CheckerRevocationKey::for_invocation(
                binding.revocation_source().clone(),
                invocation,
            )
            .unwrap();
            let policy: PolicyId = sid("policy/same-version-test");
            let head: CheckerRevocationHeadId = sid("head/same");
            let provenance: EvidenceProvenanceId = sid("provenance/exact");
            let support: EvidenceSetId = sid("support/exact");
            let selected = CheckerRevocationSet::from_owner_snapshot(
                binding.owner().clone(),
                binding.revocation_source().clone(),
                policy.clone(),
                head,
                vec![
                    CheckerRevocationDecision::new(key.clone(), RevocationStatus::Current).unwrap(),
                ],
            )
            .unwrap()
            .lookup(&key);
            CheckerReceipt::passing(
                &binding,
                invocation,
                sid("result/pass"),
                EvidenceGrade::Reproducible,
                provenance,
                policy,
                support,
                &selected,
            )
            .unwrap()
        };
        assert_ne!(issue(&invocation_a).id(), issue(&invocation_b).id());
    }

    #[test]
    fn installed_build_identity_is_a_lowercase_sha256() {
        let identity = find_pack("checker/c-op").unwrap().build_inputs_sha256();
        assert_eq!(identity.len(), 64);
        assert!(
            identity
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        );
    }

    #[test]
    fn native_operation_binding_is_distinct_and_names_the_call_that_runs() {
        let spec = find_pack("checker/c-op").unwrap();
        let command = spec.checker_binding().unwrap();
        let native = spec.native_operation_local_binding().unwrap();
        assert_ne!(command.id(), native.id());
        assert_eq!(
            native.template().entrypoint(),
            "sim_check_pack::invoke_operation_local"
        );
        assert_eq!(
            native.template().cwd(),
            &WorkingDirectoryPolicyId::from_text("call/no-working-directory").unwrap()
        );
        assert_eq!(
            native.template().environment(),
            &EnvironmentPolicyId::from_text("call/native-rust-v1").unwrap()
        );
        assert_eq!(
            native.allowed_scopes(),
            &BTreeSet::from([CheckScopeId::from_text("operation/local").unwrap()])
        );
        assert!(
            find_pack("checker/c-work")
                .unwrap()
                .native_operation_local_binding()
                .is_err()
        );
    }
}
