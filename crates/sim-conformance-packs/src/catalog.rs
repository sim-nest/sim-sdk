//! Static checker, phase, and scope catalog.

/// One statically registered checker pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackSpec {
    /// Stable checker id.
    pub checker: &'static str,
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

macro_rules! spec {
    ($checker:literal, $pack:literal, $phase:literal, [$($scope:literal),* $(,)?], [$($implemented:literal),* $(,)?]) => {
        PackSpec {
            checker: $checker,
            pack: $pack,
            producing_phase: $phase,
            allowed_scopes: &[$($scope),*],
            implemented_scopes: &[$($implemented),*],
        }
    };
}

const PACKS: &[PackSpec] = &[
    spec!(
        "checker/c-v3",
        "pack/retirement",
        "NV12.01",
        ["retirement/bootstrap", "retirement/final"],
        ["retirement/bootstrap"]
    ),
    spec!(
        "checker/c-id",
        "pack/identity",
        "NV12.01",
        [
            "identity/register",
            "identity/vectors",
            "identity/journal-normalized",
            "identity/closure-final",
            "identity/store-roundtrip",
            "identity/fixed-point",
            "identity/world-seal",
            "identity/evidence",
            "identity/final-seal"
        ],
        ["identity/register", "identity/vectors"]
    ),
    spec!(
        "checker/c-own",
        "pack/ownership",
        "NV12.01",
        [
            "ownership/activation",
            "ownership/dependencies",
            "ownership/bootstrap",
            "ownership/produced",
            "ownership/roadmap-final"
        ],
        ["ownership/activation"]
    ),
    spec!(
        "checker/c-boundary",
        "pack/boundary",
        "NV12.01",
        [
            "boundary/inventory",
            "boundary/local-adapter",
            "boundary/projection-admission",
            "boundary/observation",
            "boundary/source-closure",
            "boundary/composed"
        ],
        ["boundary/inventory"]
    ),
    spec!(
        "checker/c-source",
        "pack/source",
        "NV12.10",
        ["source/admission", "source/performance", "source/composed"],
        []
    ),
    spec!(
        "checker/c-evidence",
        "pack/evidence",
        "NV12.06",
        [
            "evidence/projection",
            "evidence/predicted-closure",
            "evidence/identity-grade",
            "evidence/cold-oracle",
            "evidence/convergence",
            "evidence/composed"
        ],
        []
    ),
    spec!(
        "checker/c-op",
        "pack/operation",
        "NV12.04",
        [
            "operation/log",
            "operation/reconcile",
            "operation/local",
            "operation/delivery-adapters",
            "operation/composed"
        ],
        []
    ),
    spec!(
        "checker/c-journal",
        "pack/journal",
        "NV12.02",
        [
            "journal/native-compatibility",
            "journal/performance",
            "journal/causal",
            "journal/composed"
        ],
        []
    ),
    spec!(
        "checker/c-closure",
        "pack/closure",
        "NV12.06",
        [
            "closure/projection",
            "closure/plan",
            "closure/frontier",
            "closure/resources",
            "closure/composed"
        ],
        []
    ),
    spec!(
        "checker/c-control",
        "pack/control",
        "NV12.13",
        ["control/liveness", "control/composed"],
        []
    ),
    spec!(
        "checker/c-work",
        "pack/work",
        "NV12.01",
        [
            "work/packet-pure",
            "work/packet-effects",
            "work/convergence",
            "work/native-dispatch",
            "work/composed"
        ],
        ["work/packet-pure"]
    ),
    spec!(
        "checker/c-drive",
        "pack/drive",
        "NV12.25",
        ["drive/native", "drive/hot-no-op", "drive/composed"],
        []
    ),
    spec!(
        "checker/c-converge",
        "pack/convergence",
        "NV12.15",
        [
            "convergence/candidate",
            "convergence/fixed-point",
            "convergence/composed"
        ],
        []
    ),
    spec!(
        "checker/c-facet",
        "pack/facet",
        "NV12.01",
        [
            "facet/pure",
            "facet/generated",
            "facet/transaction",
            "facet/landing",
            "facet/composed"
        ],
        ["facet/pure"]
    ),
    spec!(
        "checker/c-disclose",
        "pack/disclosure",
        "NV12.06",
        [
            "disclosure/projection",
            "disclosure/transaction",
            "disclosure/landing",
            "disclosure/composed"
        ],
        []
    ),
    spec!(
        "checker/c-deliver",
        "pack/delivery",
        "NV12.19",
        [
            "delivery/observation",
            "delivery/plan",
            "delivery/adapters",
            "delivery/real-canary",
            "delivery/composed"
        ],
        []
    ),
    spec!(
        "checker/c-author",
        "pack/authoring",
        "NV12.24",
        ["authoring/family", "authoring/composed"],
        []
    ),
    spec!(
        "checker/c-port",
        "pack/portability",
        "NV12.11",
        ["portability/foreign-project", "portability/consumer-seams"],
        []
    ),
    spec!(
        "checker/c-product",
        "pack/product",
        "NV12.06",
        [
            "product/world",
            "product/prove-plan",
            "product/prove",
            "product/convergence-queries",
            "product/landing-queries",
            "product/deliver",
            "product/roadmap",
            "product/query-latency",
            "product/installed",
            "product/composed"
        ],
        []
    ),
    spec!(
        "checker/c-release",
        "pack/release",
        "NV12.01",
        [
            "release/nv12-01",
            "release/nv12-02",
            "release/nv12-03",
            "release/nv12-04",
            "release/nv12-05",
            "release/nv12-06",
            "release/nv12-07",
            "release/nv12-08",
            "release/nv12-09",
            "release/nv12-10",
            "release/nv12-11",
            "release/nv12-12",
            "release/nv12-13",
            "release/nv12-14",
            "release/nv12-15",
            "release/nv12-16",
            "release/nv12-17",
            "release/nv12-18",
            "release/nv12-19",
            "release/nv12-20",
            "release/nv12-21",
            "release/nv12-22",
            "release/nv12-23",
            "release/nv12-24",
            "release/nv12-25",
            "release/nv12-26",
            "release/nv12-27",
            "release/nv12-28"
        ],
        ["release/nv12-01"]
    ),
    spec!(
        "checker/c-succeed",
        "pack/succession",
        "NV12.28",
        ["succession/composed-precheck", "succession/cold-change"],
        []
    ),
];

/// Returns every registered checker in stable catalog order.
pub const fn all_packs() -> &'static [PackSpec] {
    PACKS
}

/// Looks up one exact checker id.
pub fn find_pack(checker: &str) -> Option<&'static PackSpec> {
    PACKS.iter().find(|spec| spec.checker == checker)
}
