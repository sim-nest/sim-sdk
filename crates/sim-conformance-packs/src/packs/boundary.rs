//! Architecture boundary inventory checks.

use crate::{CheckObservation, PackRequest, PackVerdict, harness};

/// Checks the exact requested boundary scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-boundary", request, |request| {
        match request.scope {
            "boundary/inventory" => inventory(request),
            "boundary/identity-closure" => identity_closure(request),
            "boundary/local-adapter" => local_adapter(request),
            "boundary/projection-admission" => projection_admission(request),
            _ => unreachable!("availability was checked before dispatch"),
        }
    })
}

fn projection_admission(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    [
        "boundary.kernel-unchanged",
        "boundary.projection-registry-open",
        "boundary.config-shape-checked",
        "boundary.inputs-immutable-and-selected",
        "boundary.mediated-reads-exact",
        "boundary.semantic-envelope-separated",
        "boundary.qualification-separate-from-confinement",
        "boundary.native-source-and-dependencies-reviewed",
        "boundary.native-loaded-code-identity-exact",
        "boundary.untrusted-native-refused",
        "boundary.native-proc-read-refused",
        "boundary.unprojected-mount-read-refused",
        "boundary.wasm-import-manifest-complete",
        "boundary.wasm-imports-exact",
        "boundary.wasm-clock-random-wasi-refused",
        "boundary.wasm-runtime-semantics-qualified",
        "boundary.wasm-instance-fresh",
        "boundary.wasm-budgets-bound",
        "boundary.bwrap-confinement-not-purity",
        "boundary.absent-membrane-typed-unavailable",
        "boundary.baseline-provider-kinds-complete",
        "boundary.federated-owner-closure-sealed",
    ]
    .into_iter()
    .map(|key| harness::expect_true(request, key))
    .collect()
}

fn local_adapter(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    [
        "boundary.kernel-unchanged",
        "boundary.behavior-remains-loaded",
        "boundary.local-port-owned-by-runtime",
        "boundary.local-adapter-owned-by-ubuntu-capsule",
        "boundary.packet-tooling-has-no-native-process",
        "boundary.command-spec-has-no-native-path",
        "boundary.native-paths-boot-resolved",
        "boundary.operation-gate-composed-on-journal",
        "boundary.process-port-reused",
        "boundary.sandbox-launcher-reused",
        "boundary.no-ambient-process",
        "boundary.no-ambient-network",
        "boundary.no-general-checkout-transaction",
        "boundary.operator-retains-integration-and-release",
        "boundary.local-route-and-specimen-indexed",
    ]
    .into_iter()
    .map(|key| harness::expect_true(request, key))
    .collect()
}

fn inventory(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    [
        "boundary.kernel-unchanged",
        "boundary.behavior-remains-loaded",
        "boundary.singular-graphs",
        "boundary.singular-stores",
        "boundary.fake-effects-only",
        "boundary.ambient-observation-refused",
        "boundary.inventory-complete",
    ]
    .into_iter()
    .map(|key| harness::expect_true(request, key))
    .collect()
}

fn identity_closure(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, crate::PackFailure> {
    [
        "boundary.kernel-unchanged",
        "boundary.kernel-policy-absent",
        "boundary.behavior-remains-loaded",
        "boundary.semantic-byte-types-distinct",
        "boundary.loader-membrane-retained",
        "boundary.tooling-delegates-codec",
        "boundary.compatibility-reader-read-only",
        "boundary.ambient-identity-authority-refused",
        "boundary.cache-reuse-requires-exact-semantics",
        "boundary.identity-closure-complete",
    ]
    .into_iter()
    .map(|key| harness::expect_true(request, key))
    .collect()
}
