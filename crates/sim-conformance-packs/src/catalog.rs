//! Static checker, phase, and scope catalog.

/// One statically registered checker pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackSpec {
    /// Stable checker id.
    pub checker: &'static str,
    /// Exact activated static checker-binding id.
    pub binding: &'static str,
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
    ($checker:literal, $binding:literal, $pack:literal, $phase:literal, [$($scope:literal),* $(,)?], [$($implemented:literal),* $(,)?]) => {
        PackSpec {
            checker: $checker,
            binding: $binding,
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
        "core/sha256-datum-v1:683334ada045fdb5fdfd3709100a703557df210835040bfe623d7b94ec00b01f",
        "pack/retirement",
        "NV12.01",
        ["retirement/bootstrap", "retirement/final"],
        ["retirement/bootstrap"]
    ),
    spec!(
        "checker/c-id",
        "core/sha256-datum-v1:9c5981c7fc65df78db1079ee58429821ea93bb36dd18c241a03ed63e24d2b75b",
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
        "core/sha256-datum-v1:f794a7d0c15f8b9fe55e9a2de0f46b547491005d41909d4ede2968ff3632a95f",
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
        "core/sha256-datum-v1:506025bc7d5e0542d24e89073bb9852f6c906e777370b49a29b51450df88cc55",
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
        "core/sha256-datum-v1:c8d5369ab87dc0af89e94e963858d534fcd83421c688524ff4acb86524ebbfb3",
        "pack/source",
        "NV12.10",
        ["source/admission", "source/performance", "source/composed"],
        []
    ),
    spec!(
        "checker/c-evidence",
        "core/sha256-datum-v1:dfe5edd6c822eb896aa73f30cfbbde2a53a9fe9f5dd941f3e158d211631447ca",
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
        "core/sha256-datum-v1:5c6a05e3ac7358f772fe94b15e8448015b1c1192bace741d9deecb7469e8d374",
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
        "core/sha256-datum-v1:ff8b794fce7a36c83cc6ff60e7d235cbe82acdb2a3ee0f1afec90959f22c7ecf",
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
        "core/sha256-datum-v1:532cc7928742e608999e3142c9bf1f5e0061a7fe988c8352c3edc9240be8d2cf",
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
        "core/sha256-datum-v1:d08c6f27822723694facd7d21add64565178eeba7e497c68c6eed279bd7589a8",
        "pack/control",
        "NV12.13",
        ["control/liveness", "control/composed"],
        []
    ),
    spec!(
        "checker/c-work",
        "core/sha256-datum-v1:87c5ff26728297fe5e1ffcdae23065bb9c849f608f1009ee116d7742582fef5d",
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
        "core/sha256-datum-v1:92ba62611e349ca12f9fc6b7bc96ff73366f4d19a923f73dfd170ecb509455fe",
        "pack/drive",
        "NV12.25",
        ["drive/native", "drive/hot-no-op", "drive/composed"],
        []
    ),
    spec!(
        "checker/c-converge",
        "core/sha256-datum-v1:21be62ab7560dbd44b14ede674480b230581192e2fc3679a674255b05d338fba",
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
        "core/sha256-datum-v1:cabd7057fee809692ed3e31a2b2aaec6c9d420e02dd6d56e64ff89819a491f75",
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
        "core/sha256-datum-v1:d3b3b658780dc2d1a33aa2d3dc6c422b17e39d200be42bde8df40a47cec589c2",
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
        "core/sha256-datum-v1:bc08074815f8d4fb19a1e7a41f1dfaa788da3bd9a7b1d3f0f619e62a53519ccf",
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
        "core/sha256-datum-v1:2694e192692ce1baded061bc4882733d55f18e49d99909b0c0c2264bd665fec6",
        "pack/authoring",
        "NV12.24",
        ["authoring/family", "authoring/composed"],
        []
    ),
    spec!(
        "checker/c-port",
        "core/sha256-datum-v1:95ae5cbe403fc8e57cfd0f1273b0443b0e96d522b8ea92ad5e3001d27f687c37",
        "pack/portability",
        "NV12.11",
        ["portability/foreign-project", "portability/consumer-seams"],
        []
    ),
    spec!(
        "checker/c-product",
        "core/sha256-datum-v1:b9baa94a3ae79f1b3fdd30f73c10684c7f7ebe1802b19dfea8ee8c91b1d3f3a5",
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
        "core/sha256-datum-v1:f12e44d0825b1b54bec8ddfab96007eccc97b9dbcf99ac8ddfe6cb34fc720cf3",
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
        "core/sha256-datum-v1:f14267fae4d45eb57c175546ebd809831a0cfa9510bd109f04ec9a280269feab",
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
