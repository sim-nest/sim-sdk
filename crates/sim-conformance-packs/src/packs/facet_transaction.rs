// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Model facet transactions through the published transaction law.
//!
//! The check calls `transact_facets` and `rollback_model`. An apply lands
//! the intended postimage and a second identical proposal does no further
//! commit. Wrong head, dirt, a private disclosure sent to a public sink, a
//! generated-owner bypass, a stale base, a region escape, and an escaping
//! path all refuse and leave the preimage. Overlap refuses. One repository
//! can commit while another refuses, and both live branch pointers stay on
//! the commits that recovery names. Rollback moves the live branch and head
//! back to the original tip and keeps the commit it leaves. The observation
//! is `facet.transaction`.
//! It is not `completion/claim-13`.

use std::collections::BTreeMap;

use sim_artifact_facet::{
    ArtifactFacet, ArtifactId, BaseImage, DisclosureDecisionId, FacetError, IntendedImage,
    MergePolicy, OwnerId, PortableImage, ProjectionId, RegionOwnership, RegionSelector,
};
use sim_lib_roadmap_runner::{
    ArtifactModel, BoundDisclosure, CleanKind, DisclosureClass, FacetEdit, GitModel, ModelBatch,
    ModelStore, ModelTip, RepoOutcome, RepoRequest, Rollback, rollback_model, transact_facets,
};

use crate::{CheckObservation, PackFailure, PackRequest};

const KEY: &str = "facet.transaction";

/// Runs the transaction law and returns its observation.
///
/// # Errors
///
/// Returns a refusal when the caller supplies a fact or the law leaves a
/// preimage, postimage, dirt, conflict, rollback, ownership, or disclosure
/// gap.
pub(crate) fn observe(request: &PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure> {
    if let Some(key) = request.evidence.fact_keys().into_iter().next() {
        return Err(gap("unexpected-fact", key));
    }
    let value = execute()?;
    if value.contains("completion/claim-") {
        return Err(gap("claim-relabelled", KEY));
    }
    Ok(vec![CheckObservation {
        key: KEY.to_owned(),
        value,
    }])
}

/// Executes the transaction cases and returns the tokens those cases proved.
///
/// # Errors
///
/// Returns a refusal when a case does not hold its preimage or postimage.
pub(crate) fn execute() -> Result<String, PackFailure> {
    let mut tokens = Vec::new();
    apply(&mut tokens)?;
    expect_refusal(&mut tokens, "wrong-head", wrong_head)?;
    expect_refusal(&mut tokens, "dirt", dirt)?;
    expect_refusal(&mut tokens, "private-flow", private_flow)?;
    expect_refusal(&mut tokens, "generated-owner-bypass", generated_bypass)?;
    expect_refusal(&mut tokens, "stale-base", stale_base)?;
    expect_refusal(&mut tokens, "region-escape", region_escape)?;
    path_escape(&mut tokens)?;
    overlap(&mut tokens)?;
    recover(&mut tokens)?;
    rollback(&mut tokens)?;
    Ok(tokens.join(" "))
}

fn apply(tokens: &mut Vec<&'static str>) -> Result<(), PackFailure> {
    let actor = owner("owner/src")?;
    let (mut store, tip) = seed("alpha", &[("src/a.txt", "old\n")])?;
    if store.image_at("alpha", &tip.head, "src/a.txt") != file("old\n")? {
        return Err(gap("transaction-gap", "apply-preimage"));
    }
    let batch = commit_exact(&mut store, &tip, &actor, "src/a.txt", "old\n", "new\n")?;
    let RepoOutcome::Applied {
        tip: next,
        previous,
        ..
    } = one(&batch)
    else {
        return Err(gap("transaction-gap", "apply-postimage"));
    };
    if *previous != tip || store.image_at("alpha", &next.head, "src/a.txt") != file("new\n")? {
        return Err(gap("transaction-gap", "apply-postimage"));
    }
    let again = transact(
        &mut store,
        &ask(
            "alpha",
            &actor,
            next,
            &next.head,
            public_disclosure()?,
            vec![change(&actor, "src/a.txt", "new\n", "new\n")?],
        ),
    )?;
    let RepoOutcome::Clean {
        kind: CleanKind::Unchanged,
        tip: current,
        ..
    } = one(&again)
    else {
        return Err(gap("transaction-gap", "apply-second"));
    };
    if current != next || store.tip("alpha").as_ref() != Some(next) {
        return Err(gap("transaction-gap", "apply-second"));
    }
    tokens.extend(["apply-preimage", "apply-postimage", "apply-second"]);
    Ok(())
}

fn wrong_head() -> Result<ModelBatch, PackFailure> {
    let actor = owner("owner/src")?;
    let (mut store, tip) = seed("alpha", &[("src/a.txt", "old\n")])?;
    let mut asked = ask(
        "alpha",
        &actor,
        &tip,
        &tip.head,
        public_disclosure()?,
        vec![change(&actor, "src/a.txt", "old\n", "new\n")?],
    );
    asked.expected_head = "commit/missing".to_owned();
    let batch = transact(&mut store, &asked)?;
    held(&store, &tip, "src/a.txt", "old\n")?;
    Ok(batch)
}

fn dirt() -> Result<ModelBatch, PackFailure> {
    let actor = owner("owner/src")?;
    let (mut store, tip) = seed("alpha", &[("src/a.txt", "old\n")])?;
    if !store.mark_dirt("alpha", "notes.txt") {
        return Err(gap("transaction-gap", "dirt"));
    }
    let batch = transact(
        &mut store,
        &ask(
            "alpha",
            &actor,
            &tip,
            &tip.head,
            public_disclosure()?,
            vec![change(&actor, "src/a.txt", "old\n", "new\n")?],
        ),
    )?;
    held(&store, &tip, "src/a.txt", "old\n")?;
    Ok(batch)
}

fn private_flow() -> Result<ModelBatch, PackFailure> {
    let actor = owner("owner/src")?;
    let (mut store, tip) = seed("alpha", &[("src/a.txt", "old\n")])?;
    let disclosure = BoundDisclosure {
        decision: decision("disclosure/model")?,
        class: DisclosureClass::Private,
        public_sink: true,
    };
    let batch = transact(
        &mut store,
        &ask(
            "alpha",
            &actor,
            &tip,
            &tip.head,
            disclosure,
            vec![change(&actor, "src/a.txt", "old\n", "new\n")?],
        ),
    )?;
    held(&store, &tip, "src/a.txt", "old\n")?;
    Ok(batch)
}

fn generated_bypass() -> Result<ModelBatch, PackFailure> {
    let generator = owner("owner/gen")?;
    let other = owner("owner/other")?;
    let (mut store, tip) = seed("alpha", &[("src/a.txt", "old\n")])?;
    let batch = transact(
        &mut store,
        &ask(
            "alpha",
            &other,
            &tip,
            &tip.head,
            public_disclosure()?,
            vec![edit(generated("src/a.txt", &generator)?, "old\n", "new\n")?],
        ),
    )?;
    held(&store, &tip, "src/a.txt", "old\n")?;
    Ok(batch)
}

fn stale_base() -> Result<ModelBatch, PackFailure> {
    let actor = owner("owner/src")?;
    let (mut store, tip) = seed("alpha", &[("src/a.txt", "old\n")])?;
    let batch = transact(
        &mut store,
        &ask(
            "alpha",
            &actor,
            &tip,
            &tip.head,
            public_disclosure()?,
            vec![change(&actor, "src/a.txt", "stale\n", "new\n")?],
        ),
    )?;
    held(&store, &tip, "src/a.txt", "old\n")?;
    Ok(batch)
}

fn region_escape() -> Result<ModelBatch, PackFailure> {
    let actor = owner("owner/src")?;
    let (mut store, tip) = seed(
        "alpha",
        &[("src/a.txt", "old\n"), ("src/b.txt", "a\nb\nc\n")],
    )?;
    let batch = transact(
        &mut store,
        &ask(
            "alpha",
            &actor,
            &tip,
            &tip.head,
            public_disclosure()?,
            vec![
                change(&actor, "src/a.txt", "old\n", "new\n")?,
                lines(&actor, "src/b.txt", "a\nb\nc\n", "Z\nb\nc\n")?,
            ],
        ),
    )?;
    held(&store, &tip, "src/a.txt", "old\n")?;
    held(&store, &tip, "src/b.txt", "a\nb\nc\n")?;
    Ok(batch)
}

fn path_escape(tokens: &mut Vec<&'static str>) -> Result<(), PackFailure> {
    let actor = owner("owner/src")?;
    let escaped = ArtifactFacet::new(
        ArtifactId::from_text("artifact/escape").map_err(law_error)?,
        actor,
        RegionSelector::ExactPath("../outside".to_owned()),
        ProjectionId::from_text("projection/file").map_err(law_error)?,
        MergePolicy::Exact,
        decision("disclosure/model")?,
        RegionOwnership::Authored,
    );
    if !matches!(escaped, Err(FacetError::InvalidRegion)) {
        return Err(gap("transaction-gap", "invalid-region"));
    }
    tokens.push("invalid-region");
    Ok(())
}

fn overlap(tokens: &mut Vec<&'static str>) -> Result<(), PackFailure> {
    let actor = owner("owner/src")?;
    let (mut store, base) = seed("alpha", &[("src/a.txt", "a\nb\nc\n")])?;
    let applied = commit_exact(
        &mut store,
        &base,
        &actor,
        "src/a.txt",
        "a\nb\nc\n",
        "A\nb\nc\n",
    )?;
    let tip = applied_tip(&applied)?;
    let batch = transact(
        &mut store,
        &ask(
            "alpha",
            &actor,
            &tip,
            &base.head,
            public_disclosure()?,
            vec![line_change(&actor, "src/a.txt", "a\nb\nc\n", "X\nb\nc\n")?],
        ),
    )?;
    if reason_at(&batch, 0) != "overlap" {
        return Err(gap("transaction-gap", "overlap"));
    }
    held(&store, &tip, "src/a.txt", "A\nb\nc\n")?;
    tokens.push("overlap");
    Ok(())
}

fn recover(tokens: &mut Vec<&'static str>) -> Result<(), PackFailure> {
    let actor = owner("owner/src")?;
    let (mut store, alpha) = seed("alpha", &[("src/a.txt", "old\n")])?;
    let beta = store
        .insert_repository("beta", "main", tree(&[("src/b.txt", "a\nb\n")])?)
        .map_err(law_error)?;
    let beta_applied = commit_exact(&mut store, &beta, &actor, "src/b.txt", "a\nb\n", "A\nb\n")?;
    let beta_tip = applied_tip(&beta_applied)?;
    let batch = transact_facets(
        &mut store,
        &[
            ask(
                "alpha",
                &actor,
                &alpha,
                &alpha.head,
                public_disclosure()?,
                vec![change(&actor, "src/a.txt", "old\n", "new\n")?],
            ),
            ask(
                "beta",
                &actor,
                &beta_tip,
                &beta.head,
                public_disclosure()?,
                vec![line_change(&actor, "src/b.txt", "a\nb\n", "B\nb\n")?],
            ),
        ],
    )
    .map_err(law_error)?;
    let RepoOutcome::Applied { tip: alpha_tip, .. } = outcome(&batch, 0) else {
        return Err(gap("transaction-gap", "recover-applied"));
    };
    let alpha_held = pointer_matches(&store, "alpha", alpha_tip);
    let beta_held = pointer_matches(&store, "beta", &beta_tip);
    if !alpha_held || !beta_held {
        return Err(gap(
            "transaction-gap",
            if !alpha_held {
                "recover-applied"
            } else {
                "recover-held"
            },
        ));
    }
    if batch.settled()
        || reason_at(&batch, 1) != "overlap"
        || store.image_at("alpha", &alpha_tip.head, "src/a.txt") != file("new\n")?
        || store.image_at("beta", &beta_tip.head, "src/b.txt") != file("A\nb\n")?
    {
        return Err(gap("transaction-gap", "recover-held"));
    }
    tokens.extend(["recover-applied", "recover-held"]);
    Ok(())
}

fn rollback(tokens: &mut Vec<&'static str>) -> Result<(), PackFailure> {
    let actor = owner("owner/src")?;
    let (mut store, tip) = seed("alpha", &[("src/a.txt", "old\n")])?;
    let applied = commit_exact(&mut store, &tip, &actor, "src/a.txt", "old\n", "new\n")?;
    let next = applied_tip(&applied)?;
    let restored = rollback_model(&mut store, "alpha", &next, &tip.head).map_err(law_error)?;
    let Rollback::Restored { tip: current } = restored else {
        return Err(gap("transaction-gap", "rollback-restored"));
    };
    if !pointer_matches(&store, "alpha", &tip)
        || current.branch != tip.branch
        || current.head != tip.head
    {
        return Err(gap("transaction-gap", "rollback-restored"));
    }
    if store.image_at("alpha", &current.head, "src/a.txt") != file("old\n")?
        || store.image_at("alpha", &next.head, "src/a.txt") != file("new\n")?
    {
        return Err(gap("transaction-gap", "rollback-kept"));
    }
    tokens.extend(["rollback-restored", "rollback-kept"]);
    Ok(())
}

fn expect_refusal(
    tokens: &mut Vec<&'static str>,
    expected: &'static str,
    run: fn() -> Result<ModelBatch, PackFailure>,
) -> Result<(), PackFailure> {
    let batch = run()?;
    if reason_at(&batch, 0) != expected {
        return Err(gap("transaction-gap", expected));
    }
    tokens.push(expected);
    Ok(())
}

fn commit_exact(
    store: &mut ModelStore,
    tip: &ModelTip,
    actor: &OwnerId,
    path: &str,
    base: &str,
    intended: &str,
) -> Result<ModelBatch, PackFailure> {
    let name = repository_of(store, tip)?;
    transact(
        store,
        &ask(
            &name,
            actor,
            tip,
            &tip.head,
            public_disclosure()?,
            vec![change(actor, path, base, intended)?],
        ),
    )
}

fn transact(store: &mut ModelStore, request: &RepoRequest) -> Result<ModelBatch, PackFailure> {
    transact_facets(store, std::slice::from_ref(request)).map_err(law_error)
}

fn pointer_matches(store: &ModelStore, repository: &str, expected: &ModelTip) -> bool {
    store
        .tip(repository)
        .is_some_and(|live| live.branch == expected.branch && live.head == expected.head)
}

fn held(store: &ModelStore, tip: &ModelTip, path: &str, body: &str) -> Result<(), PackFailure> {
    if store.tip(repository_of(store, tip)?.as_str()).as_ref() != Some(tip)
        || store.image_at(&repository_of(store, tip)?, &tip.head, path) != file(body)?
    {
        return Err(gap("transaction-gap", "preimage"));
    }
    Ok(())
}

fn repository_of(store: &ModelStore, tip: &ModelTip) -> Result<String, PackFailure> {
    for name in ["alpha", "beta"] {
        if store.tip(name).as_ref() == Some(tip) {
            return Ok(name.to_owned());
        }
    }
    Err(gap("transaction-gap", "repository"))
}

fn applied_tip(batch: &ModelBatch) -> Result<ModelTip, PackFailure> {
    match one(batch) {
        RepoOutcome::Applied { tip, .. } => Ok(tip.clone()),
        _ => Err(gap("transaction-gap", "apply-postimage")),
    }
}

fn one(batch: &ModelBatch) -> &RepoOutcome {
    outcome(batch, 0)
}

fn outcome(batch: &ModelBatch, index: usize) -> &RepoOutcome {
    &batch.outcomes()[index]
}

fn reason_at(batch: &ModelBatch, index: usize) -> &'static str {
    match batch.outcomes().get(index) {
        Some(RepoOutcome::Refused { reason, .. }) => reason,
        _ => "",
    }
}

fn ask(
    repository: &str,
    actor: &OwnerId,
    tip: &ModelTip,
    base_revision: &str,
    disclosure: BoundDisclosure,
    edits: Vec<FacetEdit>,
) -> RepoRequest {
    RepoRequest {
        repository: repository.to_owned(),
        actor: actor.clone(),
        expected_branch: tip.branch.clone(),
        expected_head: tip.head.clone(),
        base_revision: base_revision.to_owned(),
        disclosure,
        edits,
    }
}

fn change(
    actor: &OwnerId,
    path: &str,
    base: &str,
    intended: &str,
) -> Result<FacetEdit, PackFailure> {
    edit(exact(path, actor)?, base, intended)
}

fn line_change(
    actor: &OwnerId,
    path: &str,
    base: &str,
    intended: &str,
) -> Result<FacetEdit, PackFailure> {
    edit(line_spec(path, actor)?, base, intended)
}

fn lines(
    actor: &OwnerId,
    path: &str,
    base: &str,
    intended: &str,
) -> Result<FacetEdit, PackFailure> {
    let spec = facet(
        path,
        actor,
        RegionSelector::LineRange {
            path: path.to_owned(),
            start: 2,
            end: 3,
        },
        linewise(),
        RegionOwnership::Authored,
    )?;
    edit(spec, base, intended)
}

fn edit(spec: ArtifactFacet, base: &str, intended: &str) -> Result<FacetEdit, PackFailure> {
    Ok(FacetEdit {
        base: BaseImage::check(&spec, file(base)?).map_err(law_error)?,
        intended: IntendedImage::check(&spec, file(intended)?).map_err(law_error)?,
        spec,
    })
}

fn exact(path: &str, actor: &OwnerId) -> Result<ArtifactFacet, PackFailure> {
    facet(
        path,
        actor,
        RegionSelector::ExactPath(path.to_owned()),
        MergePolicy::Exact,
        RegionOwnership::Authored,
    )
}

fn line_spec(path: &str, actor: &OwnerId) -> Result<ArtifactFacet, PackFailure> {
    facet(
        path,
        actor,
        RegionSelector::ExactPath(path.to_owned()),
        linewise(),
        RegionOwnership::Authored,
    )
}

fn generated(path: &str, actor: &OwnerId) -> Result<ArtifactFacet, PackFailure> {
    facet(
        path,
        actor,
        RegionSelector::ExactPath(path.to_owned()),
        MergePolicy::Exact,
        RegionOwnership::Generated {
            generator: actor.clone(),
        },
    )
}

fn facet(
    path: &str,
    actor: &OwnerId,
    region: RegionSelector,
    policy: MergePolicy,
    ownership: RegionOwnership,
) -> Result<ArtifactFacet, PackFailure> {
    ArtifactFacet::new(
        ArtifactId::from_text(&format!("artifact/{path}")).map_err(law_error)?,
        actor.clone(),
        region,
        ProjectionId::from_text("projection/file").map_err(law_error)?,
        policy,
        decision("disclosure/model")?,
        ownership,
    )
    .map_err(law_error)
}

fn linewise() -> MergePolicy {
    MergePolicy::Linewise {
        max_lines: 32,
        max_cells: 4_096,
    }
}

fn public_disclosure() -> Result<BoundDisclosure, PackFailure> {
    Ok(BoundDisclosure {
        decision: decision("disclosure/model")?,
        class: DisclosureClass::Public,
        public_sink: true,
    })
}

fn seed(name: &str, files: &[(&str, &str)]) -> Result<(ModelStore, ModelTip), PackFailure> {
    let mut store = ModelStore::new();
    let tip = store
        .insert_repository(name, "main", tree(files)?)
        .map_err(law_error)?;
    Ok((store, tip))
}

fn tree(files: &[(&str, &str)]) -> Result<BTreeMap<String, PortableImage>, PackFailure> {
    let mut tree = BTreeMap::new();
    for (path, body) in files {
        tree.insert((*path).to_owned(), file(body)?);
    }
    Ok(tree)
}

fn file(body: &str) -> Result<PortableImage, PackFailure> {
    PortableImage::file(body.as_bytes(), 0o644).map_err(law_error)
}

fn owner(name: &str) -> Result<OwnerId, PackFailure> {
    OwnerId::from_text(name).map_err(law_error)
}

fn decision(name: &str) -> Result<DisclosureDecisionId, PackFailure> {
    DisclosureDecisionId::from_text(name).map_err(law_error)
}

fn law_error(error: impl ToString) -> PackFailure {
    gap("transaction-refused", &error.to_string())
}

fn gap(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_transaction_law_records_its_cases_without_a_claim_label() {
        let value = super::execute().expect("transaction");
        assert!(!value.contains("completion/claim-"));
        for token in [
            "apply-preimage",
            "apply-postimage",
            "apply-second",
            "wrong-head",
            "dirt",
            "private-flow",
            "generated-owner-bypass",
            "stale-base",
            "region-escape",
            "invalid-region",
            "overlap",
            "recover-applied",
            "recover-held",
            "rollback-restored",
            "rollback-kept",
        ] {
            assert!(value.contains(token), "{token} missing from {value}");
        }
    }
}
