//! Curated SDK composition for the media-edge-host music vertical.

pub use sim_lib_music_core::{
    MediaEdgeMusicPlan, MusicRouteEndpoint, MusicRouteRole, RouteEvidence,
};
pub use sim_lib_stream_host::{FakeEffectSession, MusicEffect, music_effect_registry};

/// Builds the standard stable-identity vertical plan.
pub fn media_edge_music_plan(session: sim_kernel::Symbol) -> MediaEdgeMusicPlan {
    MediaEdgeMusicPlan::standard(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdk_vertical_composes_only_reviewed_effects_and_resilient_routes() {
        let plan =
            media_edge_music_plan(sim_kernel::Symbol::qualified("music/session", "sdk-recipe"));
        assert!(plan.survives(&[sim_kernel::Symbol::qualified("music/route", "oasys-audio")]));
        let registry = music_effect_registry().unwrap();
        assert!(
            registry
                .get(&MusicEffect::MidiSend.descriptor().id)
                .is_some()
        );
        assert!(
            registry
                .get(&sim_kernel::Symbol::qualified(
                    "device/effect",
                    "vehicle-start"
                ))
                .is_none()
        );
    }
}
