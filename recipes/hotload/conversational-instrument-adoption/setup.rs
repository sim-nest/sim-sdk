use sim::lib_agent::atelier::{InstrumentAdoption, InstrumentProposal};

/// Chat history, model state, scratch trees, and derived views never cross this seam.
pub fn adopt_reproduced_instrument(
    adoption: &mut InstrumentAdoption,
    proposal: &InstrumentProposal,
) {
    let clean = proposal.recipe.artifacts.clone();
    adoption
        .adopt(
            "recipe-transaction",
            "next-generation",
            "registry/selected",
            proposal,
            &clean,
        )
        .expect("checked recipe must adopt atomically");
}
