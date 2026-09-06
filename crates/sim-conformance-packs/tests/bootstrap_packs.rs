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
            "journal/native-compatibility",
            &evidence,
        )),
        PackVerdict::UnimplementedPack {
            checker: "checker/c-journal",
            ref funded_phase,
            ..
        } if funded_phase == "NV12.02"
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
