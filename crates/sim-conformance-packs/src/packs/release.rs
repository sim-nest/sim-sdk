// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Exact release-closure evidence checks.

#[path = "release_binding.rs"]
mod binding;
#[path = "release_boot.rs"]
mod boot;
#[path = "release_brochure.rs"]
mod brochure;
#[path = "release_closure.rs"]
mod closure;
#[path = "release_cold.rs"]
mod cold;
#[path = "release_command.rs"]
mod command;
#[path = "release_consumers.rs"]
mod consumers;
#[path = "release_converge.rs"]
mod converge;
#[path = "release_facet.rs"]
mod facet_tx;
#[path = "release_index.rs"]
mod index_guard;
#[path = "release_negatives.rs"]
mod negatives;
#[path = "release_neutral.rs"]
mod neutral;
#[path = "release_observed.rs"]
mod observed;
#[path = "release_receipt.rs"]
mod receipt;
#[path = "release_recipe.rs"]
mod recipe_guard;
#[path = "release_replay.rs"]
mod replay;
#[path = "release_retirement.rs"]
mod retirement;
#[path = "release_roadmap.rs"]
mod roadmap;
#[path = "release_roundtrip.rs"]
mod roundtrip;
#[path = "release_route.rs"]
mod route_guard;
#[path = "release_seams.rs"]
mod seams;
#[path = "release_selection.rs"]
mod selection;
#[path = "release_source.rs"]
mod source;
#[path = "release_stage.rs"]
mod stage_one;
#[path = "release_transfer.rs"]
mod transfer;
#[path = "release_universe.rs"]
mod universe;
#[path = "release_validation.rs"]
mod validation;
#[path = "release_worktree.rs"]
mod worktree;

pub use binding::{
    ObservedBindingCatalog, ObservedIdentityRoundtrip, bind_binding_catalog,
    bind_identity_roundtrip, observe_binding_catalog, observe_identity_roundtrip,
};
pub use boot::{ObservedBootGuard, bind_boot_guard, observe_boot_guard};
pub use brochure::{ObservedBrochureGuard, bind_brochure_guard, observe_brochure_guard};
pub use cold::{ObservedColdStage, bind_cold_stage, observe_cold_stage};
pub use consumers::{ObservedConsumers, bind_consumers, observe_consumers};
pub use converge::{
    ObservedSealedConvergence, bind_sealed_convergence, observe_sealed_convergence,
};
pub use facet_tx::{ObservedFacetTransaction, bind_facet_transaction, observe_facet_transaction};
pub use index_guard::{ObservedIndexGuard, bind_index_guard, observe_index_guard};
pub use negatives::{ObservedNegatives, bind_negatives, observe_negatives};
pub use neutral::{ObservedNeutralInput, bind_neutral_input, observe_neutral_input};
pub use observed::{
    ObservedPublished, ObservedReleaseObligations, PublishedObligation, ReleaseObligations,
    bind_release_obligations, observe_release_obligations,
};
pub use recipe_guard::{ObservedRecipeGuard, bind_recipe_guard, observe_recipe_guard};
pub use replay::{
    ExecutedEntry, ExecutedJournal, ObservedJournalCompletion, format_executed_journal,
    observe_journal_completion,
};
pub use retirement::{ObservedRetirementGuard, bind_retirement_guard, observe_retirement_guard};
pub use roadmap::{ObservedRoadmapAuthority, bind_roadmap_authority, observe_roadmap_authority};
pub use roundtrip::{
    ObservedSourceRoundtrip, SourceRoundtrip, bind_source_roundtrip, observe_source_roundtrip,
};
pub use route_guard::{ObservedRouteGuard, bind_route_guard, observe_route_guard};
pub use seams::{ObservedPortableSeams, bind_portable_seams, observe_portable_seams};
pub use selection::{
    ObservedSourceSelection, SelectedSource, bind_source_selection, observe_source_selection,
};
pub use source::{ObservedSourceRelease, observe_source_identity};
pub use stage_one::{ObservedStageOne, bind_stage_one, observe_stage_one};
pub use transfer::{
    ObservedAuthorityTransfer, TransferSource, bind_authority_transfer, observe_authority_transfer,
};
pub use universe::{ObservedSourceUniverse, bind_source_universe, observe_source_universe};
pub use validation::{ObservedOwnerValidation, bind_owner_validation, observe_owner_validation};
pub use worktree::{GitPackage, ObservedGitRelease, bind_git_release, observe_git_release};

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Checks one release closure before a cold succession record is accepted.
pub(super) fn closure_precheck(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    closure::check_nv12_27(request)
}

/// Refuses an observation whose whole value is a publication or completion claim.
pub(super) fn refuse_claim_values(observations: &[CheckObservation]) -> Result<(), PackFailure> {
    closure::refuse_claim_values(observations)
}

/// Closure observations shared by the funded release phase scopes.
pub const CLOSURE_FACTS: [&str; 12] = [
    "release.owner-validation-passed",
    "release.owner-docs-passed",
    "release.packages-assembled",
    "release.audit-passed",
    "release.authorship-passed",
    "release.tags-exact",
    "release.pins-exact",
    "release.mirrors-current",
    "release.generated-converged",
    "release.boot-smoke-passed",
    "release.publication-confirmed",
    "release.standalone-ci-green",
];

/// Checks the exact requested release scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-release", request, |request| {
        if request.scope == "release/nv12-27" {
            return closure::check_nv12_27(request);
        }
        if request.scope == "release/nv12-28" {
            return closure::check_nv12_28(request);
        }
        if request.scope == "release/source-identity" {
            return source::observations(request);
        }
        if request.scope == "release/git-worktree" {
            return worktree::observations(request);
        }
        if request.scope == "release/observed-obligations" {
            return observed::observations(request);
        }
        if request.scope == "release/owner-validation" {
            return validation::observations(request);
        }
        if request.scope == "release/source-selection" {
            return selection::observations(request);
        }
        if request.scope == "release/source-roundtrip" {
            return roundtrip::observations(request);
        }
        if request.scope == "release/retirement-guard" {
            return retirement::observations(request);
        }
        if request.scope == "release/boot-guard" {
            return boot::observations(request);
        }
        if request.scope == "release/brochure-guard" {
            return brochure::observations(request);
        }
        if request.scope == "release/index-guard" {
            return index_guard::observations(request);
        }
        if request.scope == "release/recipe-guard" {
            return recipe_guard::observations(request);
        }
        if request.scope == "release/route-guard" {
            return route_guard::observations(request);
        }
        if request.scope == "release/authority-transfer" {
            return transfer::observations(request);
        }
        if request.scope == "release/binding-catalog" {
            return binding::binding_observations(request);
        }
        if request.scope == "release/identity-roundtrip" {
            return binding::identity_observations(request);
        }
        if request.scope == "completion/replay" {
            return replay::observations(request);
        }
        if request.scope == "completion/seams" {
            return seams::observations(request);
        }
        if request.scope == "completion/roadmap" {
            return roadmap::observations(request);
        }
        if request.scope == "completion/neutral-input" {
            return neutral::observations(request);
        }
        if request.scope == "completion/facet-transaction" {
            return facet_tx::observations(request);
        }
        if request.scope == "completion/cold-stage" {
            return cold::observations(request);
        }
        if request.scope == "completion/stage-one" {
            return stage_one::observations(request);
        }
        if request.scope == "completion/consumers" {
            return consumers::observations(request);
        }
        if request.scope == "completion/negatives" {
            return negatives::observations(request);
        }
        if request.scope == "completion/source-universe" {
            return universe::observations(request);
        }
        if request.scope == "completion/sealed-convergence" {
            return converge::observations(request);
        }
        CLOSURE_FACTS
            .into_iter()
            .map(|key| harness::expect_true(request, key))
            .collect::<Result<Vec<CheckObservation>, _>>()
    })
}
