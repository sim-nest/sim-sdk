use std::collections::BTreeSet;

use sim_artifact_facet::{
    ArtifactFacet, BaseImage, FacetError, IntendedImage, MergeOutcome, ObservedImage, merge_facet,
};
use sim_conformance_core::{
    CheckScopeId, DependencyUseSet, IdKind, InputPort, OwnerBinding, SemanticId,
};
use sim_conformance_packs::{
    MemorySubject, PackRequest, PackVerdict, all_packs, find_pack, packs,
    packs::{facet::FacetLaw, work::PacketLaw},
};
use sim_work_core::{
    ImplementationPacket, PacketDraft, PacketInputSpec, SurfaceEvidence, WorkError,
};

// conformance: foreign public-trait implementation and scope-exact pack registry.

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn request<'a>(checker: &'a str, scope: &'a str, evidence: &'a MemorySubject) -> PackRequest<'a> {
    let binding = find_pack(checker).unwrap().checker_binding().unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    PackRequest {
        checker,
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
fn catalog_has_all_bindings_and_future_scopes_fail_closed() {
    assert_eq!(all_packs().len(), 21);
    let evidence = MemorySubject::default();
    assert!(matches!(
        packs::journal::check(&request(
            "checker/c-journal",
            "journal/composed",
            &evidence,
        )),
        PackVerdict::UnimplementedPack {
            checker: "checker/c-journal",
            ref funded_phase,
            ..
        } if funded_phase == "NV12.26"
    ));
    assert!(matches!(
        packs::facet::check(&request("checker/c-facet", "facet/landing", &evidence)),
        PackVerdict::UnimplementedPack { .. }
    ));
}

#[test]
fn wrong_scope_and_skipped_release_gate_are_named_refusals() {
    let evidence = MemorySubject::default();
    assert!(matches!(
        packs::identity::check(&request("checker/c-id", "identity/not-bound", &evidence)),
        PackVerdict::Refused(ref failure) if failure.code == "wrong-scope"
    ));
    assert!(matches!(
        packs::release::check(&request(
            "checker/c-release",
            "release/nv12-01",
            &evidence,
        )),
        PackVerdict::Refused(ref failure) if failure.code == "missing-evidence"
    ));

    let binding =
        "core/sha256-datum-v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    assert!(matches!(
        packs::identity::check(&PackRequest {
            checker: "checker/c-id",
            binding,
            subject: SUBJECT,
            scope: "identity/vectors",
            evidence: &evidence,
        }),
        PackVerdict::Refused(ref failure) if failure.code == "wrong-binding"
    ));
}

#[test]
fn journal_and_normalized_identity_scopes_are_exact_and_fail_closed() {
    let native_keys = [
        "journal.old-only-replay",
        "journal.new-only-replay",
        "journal.mixed-replay",
        "journal.v1-prefix-byte-identical",
        "journal.corrupt-prefix-refused",
        "journal.corrupt-payload-refused",
        "journal.truncated-transition-refused",
        "journal.head-mismatch-refused",
        "journal.stale-fence-refused",
        "journal.all-upgrade-crashes-recover",
        "journal.all-append-crashes-recover",
        "journal.paused-old-writer-cas-loses",
        "journal.old-residue-inert",
        "journal.old-winner-included",
        "journal.competing-upgrades-one-winner",
        "journal.conflicting-new-writers-one-head",
        "journal.progress-after-every-recovery",
        "journal.correspondence-index-rebuilds",
        "journal.object-index-rebuilds",
        "journal.no-v3-schema-reader",
    ];
    let native = native_keys
        .into_iter()
        .fold(MemorySubject::default(), |subject, key| {
            subject.with(key, "true")
        });
    assert!(matches!(
        packs::journal::check(&request(
            "checker/c-journal",
            "journal/native-compatibility",
            &native,
        )),
        PackVerdict::Pass { .. }
    ));
    let missing = MemorySubject::default();
    assert!(matches!(
        packs::journal::check(&request(
            "checker/c-journal",
            "journal/native-compatibility",
            &missing,
        )),
        PackVerdict::Refused(ref failure) if failure.code == "missing-evidence"
    ));

    let normalized = [
        "identity.journal-entry-is-datum",
        "identity.journal-payload-is-datum",
        "identity.journal-head-is-semantic",
        "identity.storage-id-is-separated",
        "identity.v1-ids-bounded-to-reader",
        "identity.consumers-see-canonical-only",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    })
    .with("identity.journal-algorithm", "core/sha256-datum-v1");
    assert!(matches!(
        packs::identity::check(&request(
            "checker/c-id",
            "identity/journal-normalized",
            &normalized,
        )),
        PackVerdict::Pass { .. }
    ));
}

#[test]
fn journal_performance_and_causal_scopes_bind_measured_facts() {
    let performance = MemorySubject::default()
        .with("journal.performance.records", "100000")
        .with("journal.performance.samples", "5")
        .with("journal.performance.maximum-ns", "1")
        .with(
            "journal.performance.ceiling-ns",
            sim_conformance_packs::packs::journal::COLD_REPLAY_100K_CEILING_NS.to_string(),
        )
        .with("journal.performance.isolated-processes", "true")
        .with("journal.performance.active-reference", "true");
    assert!(matches!(
        packs::journal::check(&request(
            "checker/c-journal",
            "journal/performance",
            &performance,
        )),
        PackVerdict::Pass { .. }
    ));

    let causal = [
        "journal.semantic-deltas-cause-only",
        "journal.evidence-set-roots-persistent",
        "journal.snapshots-verified",
        "journal.snapshot-damage-refused",
        "journal.retention-roots-explicit",
        "journal.telemetry-separate",
        "journal.unchanged-10000-no-records",
        "journal.unchanged-10000-no-linear-evidence-copy",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    });
    assert!(matches!(
        packs::journal::check(&request("checker/c-journal", "journal/causal", &causal)),
        PackVerdict::Pass { .. }
    ));
}

struct ForeignFacet;

impl FacetLaw for ForeignFacet {
    fn merge(
        &self,
        spec: &ArtifactFacet,
        base: &BaseImage,
        observed: &ObservedImage,
        intended: &IntendedImage,
    ) -> Result<MergeOutcome, FacetError> {
        merge_facet(spec, base, observed, intended)
    }
}

struct DisjointLoss;

impl FacetLaw for DisjointLoss {
    fn merge(
        &self,
        spec: &ArtifactFacet,
        base: &BaseImage,
        observed: &ObservedImage,
        intended: &IntendedImage,
    ) -> Result<MergeOutcome, FacetError> {
        let result = merge_facet(spec, base, observed, intended)?;
        if matches!(result, MergeOutcome::Merged { .. }) {
            Ok(MergeOutcome::AlreadyTrue)
        } else {
            Ok(result)
        }
    }
}

#[test]
fn foreign_facet_is_judged_through_public_traits_and_mutant_is_named() {
    assert!(packs::facet::check_implementation(&ForeignFacet).is_ok());
    let failure = packs::facet::check_implementation(&DisjointLoss).unwrap_err();
    assert_eq!(failure.code, "disjoint-edit-loss");
}

struct ForeignPacket;

impl PacketLaw for ForeignPacket {
    fn build(
        &self,
        draft: PacketDraft,
        binding: &OwnerBinding,
        uses: &DependencyUseSet,
        evidence: &[SurfaceEvidence],
        inputs: Vec<PacketInputSpec>,
        port: &mut dyn InputPort,
    ) -> Result<ImplementationPacket, WorkError> {
        sim_work_core::PacketBuilder::build(draft, binding, uses, evidence, inputs, port)
    }
}

#[test]
fn foreign_packet_builder_is_judged_through_public_traits() {
    assert!(packs::work::check_implementation(&ForeignPacket).is_ok());
}

fn sid<K: IdKind>(value: &str) -> SemanticId<K> {
    SemanticId::from_text(value).unwrap()
}

#[test]
fn one_static_binding_yields_four_exact_invocation_ids() {
    let spec = find_pack("checker/c-id").unwrap();
    let binding = spec.checker_binding().unwrap();
    let scopes: BTreeSet<CheckScopeId> = [sid("identity/register"), sid("identity/vectors")]
        .into_iter()
        .collect();
    let original = binding.id().clone();
    assert_ne!(render(binding.id().content_id()), spec.activation_binding);
    let mut invocations = BTreeSet::new();
    for subject in [sid("subject/a"), sid("subject/b")] {
        for scope in scopes.clone() {
            let invocation = binding
                .instantiate(
                    sid("sim-conformance-packs@0.2.0"),
                    spec.pack_id().unwrap(),
                    subject.clone(),
                    scope,
                    sid("closure/bootstrap"),
                )
                .unwrap();
            invocations.insert(invocation.id().clone());
        }
    }
    assert_eq!(invocations.len(), 4);
    assert_eq!(binding.id(), &original);
}
