//! Pure artifact-facet conformance scenarios.

use sim_artifact_facet::{
    ArtifactFacet, ArtifactId, BaseImage, DisclosureDecisionId, FacetError, FacetIdKind,
    FacetSemanticId, IntendedImage, MergeOutcome, MergePolicy, ObservedImage, OwnerId,
    PortableImage, ProjectionId, RegionOwnership, RegionSelector, merge_facet,
};

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Public seam used to judge the canonical law and foreign implementations.
pub trait FacetLaw {
    /// Applies one already role-checked three-way facet comparison.
    fn merge(
        &self,
        spec: &ArtifactFacet,
        base: &BaseImage,
        observed: &ObservedImage,
        intended: &IntendedImage,
    ) -> Result<MergeOutcome, FacetError>;
}

/// The released `sim-artifact-facet` implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct CanonicalFacetLaw;

impl FacetLaw for CanonicalFacetLaw {
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

/// Checks the exact requested facet scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-facet", request, |_| {
        check_implementation(&CanonicalFacetLaw)
    })
}

/// Runs the complete bootstrap pure-facet scenarios through a public trait.
pub fn check_implementation(law: &dyn FacetLaw) -> Result<Vec<CheckObservation>, PackFailure> {
    let exact = spec(
        MergePolicy::Exact,
        RegionSelector::ExactPath("src/lib.rs".into()),
    )?;
    let base_file = file("base\n")?;
    let base = BaseImage::check(&exact, base_file.clone()).map_err(facet_failure)?;
    let observed = ObservedImage::check(&exact, file("post\n")?).map_err(facet_failure)?;
    let intended = IntendedImage::check(&exact, file("post\n")?).map_err(facet_failure)?;
    if law
        .merge(&exact, &base, &observed, &intended)
        .map_err(facet_failure)?
        != MergeOutcome::AlreadyTrue
    {
        return Err(failure("already-true"));
    }

    let observed = ObservedImage::check(&exact, file("foreign\n")?).map_err(facet_failure)?;
    let retained = observed.id().clone();
    let intended = IntendedImage::check(&exact, base_file.clone()).map_err(facet_failure)?;
    if law
        .merge(&exact, &base, &observed, &intended)
        .map_err(facet_failure)?
        != (MergeOutcome::Unchanged { retained })
    {
        return Err(failure("unchanged-retention"));
    }

    let observed = ObservedImage::check(&exact, base_file).map_err(facet_failure)?;
    let intended = IntendedImage::check(&exact, file("post\n")?).map_err(facet_failure)?;
    if !matches!(
        law.merge(&exact, &base, &observed, &intended).map_err(facet_failure)?,
        MergeOutcome::Apply { postimage } if postimage.image() == &file("post\n")?
    ) {
        return Err(failure("direct-apply"));
    }

    let linewise = spec(
        MergePolicy::Linewise {
            max_lines: 16,
            max_cells: 1_024,
        },
        RegionSelector::ExactPath("src/lib.rs".into()),
    )?;
    let base = BaseImage::check(&linewise, file("one\ntwo\nthree\n")?).map_err(facet_failure)?;
    let observed =
        ObservedImage::check(&linewise, file("ONE\ntwo\nthree\n")?).map_err(facet_failure)?;
    let intended =
        IntendedImage::check(&linewise, file("one\ntwo\nTHREE\n")?).map_err(facet_failure)?;
    if !matches!(
        law.merge(&linewise, &base, &observed, &intended).map_err(facet_failure)?,
        MergeOutcome::Merged { postimage }
            if postimage.image() == &file("ONE\ntwo\nTHREE\n")?
    ) {
        return Err(PackFailure {
            code: "disjoint-edit-loss",
            detail: "foreign and intended disjoint edits were not both retained".into(),
        });
    }

    let observed =
        ObservedImage::check(&linewise, file("one\nLEFT\nthree\n")?).map_err(facet_failure)?;
    let intended =
        IntendedImage::check(&linewise, file("one\nRIGHT\nthree\n")?).map_err(facet_failure)?;
    if !matches!(
        law.merge(&linewise, &base, &observed, &intended)
            .map_err(facet_failure)?,
        MergeOutcome::Conflict { .. }
    ) {
        return Err(PackFailure {
            code: "stale-base-not-refused",
            detail: "overlapping foreign and intended edits did not conflict".into(),
        });
    }

    generated_owner_refusal()?;
    line_region_refusal(law)?;
    Ok(vec![CheckObservation {
        key: "facet.pure-scenarios".into(),
        value: "already-true,unchanged,apply,disjoint,overlap,owner,region".into(),
    }])
}

fn generated_owner_refusal() -> Result<(), PackFailure> {
    let result = ArtifactFacet::new(
        id::<sim_artifact_facet::ArtifactKind>("artifact/demo")?,
        id::<sim_artifact_facet::OwnerKind>("owner/authored")?,
        RegionSelector::ExactPath("src/lib.rs".into()),
        id::<sim_artifact_facet::ProjectionKind>("projection/file")?,
        MergePolicy::Exact,
        id::<sim_artifact_facet::DisclosureKind>("disclosure/public")?,
        RegionOwnership::Generated {
            generator: id::<sim_artifact_facet::OwnerKind>("owner/generator")?,
        },
    );
    if result != Err(FacetError::WrongOwner) {
        return Err(PackFailure {
            code: "wrong-owner-accepted",
            detail: "generated facet accepted a different generator owner".into(),
        });
    }
    Ok(())
}

fn line_region_refusal(law: &dyn FacetLaw) -> Result<(), PackFailure> {
    let facet = spec(
        MergePolicy::Linewise {
            max_lines: 16,
            max_cells: 1_024,
        },
        RegionSelector::LineRange {
            path: "src/lib.rs".into(),
            start: 1,
            end: 2,
        },
    )?;
    let base_image = file("one\ntwo\nthree\n")?;
    let base = BaseImage::check(&facet, base_image.clone()).map_err(facet_failure)?;
    let observed = ObservedImage::check(&facet, base_image).map_err(facet_failure)?;
    let intended =
        IntendedImage::check(&facet, file("ONE\ntwo\nthree\n")?).map_err(facet_failure)?;
    if law.merge(&facet, &base, &observed, &intended) != Err(FacetError::OutOfRegion) {
        return Err(PackFailure {
            code: "out-of-region-accepted",
            detail: "line facet accepted an edit before its region".into(),
        });
    }
    Ok(())
}

fn spec(policy: MergePolicy, region: RegionSelector) -> Result<ArtifactFacet, PackFailure> {
    ArtifactFacet::new(
        artifact_id("artifact/demo")?,
        owner_id("owner/crate")?,
        region,
        projection_id("projection/file")?,
        policy,
        disclosure_id("disclosure/public")?,
        RegionOwnership::Authored,
    )
    .map_err(facet_failure)
}

fn artifact_id(value: &str) -> Result<ArtifactId, PackFailure> {
    id(value)
}

fn owner_id(value: &str) -> Result<OwnerId, PackFailure> {
    id(value)
}

fn projection_id(value: &str) -> Result<ProjectionId, PackFailure> {
    id(value)
}

fn disclosure_id(value: &str) -> Result<DisclosureDecisionId, PackFailure> {
    id(value)
}

fn id<K: FacetIdKind>(value: &str) -> Result<FacetSemanticId<K>, PackFailure> {
    FacetSemanticId::from_text(value).map_err(facet_failure)
}

fn file(value: &str) -> Result<PortableImage, PackFailure> {
    PortableImage::file(value.as_bytes(), 0o644).map_err(facet_failure)
}

fn facet_failure(error: FacetError) -> PackFailure {
    PackFailure {
        code: "facet-law-refusal",
        detail: error.to_string(),
    }
}

fn failure(detail: &str) -> PackFailure {
    PackFailure {
        code: "facet-law-mismatch",
        detail: detail.into(),
    }
}
