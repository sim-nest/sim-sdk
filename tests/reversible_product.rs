use sim_lib_view_worktable::{
    Availability, EffectState, RoomCard, ValueConclusion, ValueObservation, Worktable,
    WorktableEdit,
};

const PILOTS: &str = include_str!("../recipes/atelier/reversible-product/pilots.toml");

#[test]
fn every_public_memo_pilot_is_explicit_and_offline() {
    for memo in [
        "standard",
        "ai",
        "support-crew",
        "vibe",
        "devices-revision-4",
    ] {
        assert!(
            PILOTS.contains(&format!("memo = \"{memo}\"")),
            "missing {memo}"
        );
    }
    for required in [
        "refusal",
        "unsupported",
        "manual",
        "fake-model",
        "fake-device",
    ] {
        assert!(PILOTS.contains(required), "missing pilot state {required}");
    }
}

#[test]
fn composition_deletes_and_rebuilds_without_effect_authority() {
    let edits = vec![
        WorktableEdit::Open {
            expedition: "content:offline-pilot".into(),
            pack_closure: "sha256:private-root-closure".into(),
        },
        WorktableEdit::PutRoom(RoomCard {
            pack: "model-portfolio".into(),
            summary: "fake model refusal retained".into(),
            route: Availability::Modeled,
            device: Availability::Unsupported,
            fallback: "inspect cassette manually".into(),
            effect: EffectState::Unavailable,
        }),
        WorktableEdit::CiteEvidence("content:fake-model-cassette".into()),
        WorktableEdit::Object("model-is-not-the-judge".into()),
        WorktableEdit::Attend("silent-offline-success".into()),
        WorktableEdit::Export("content:expedition-export".into()),
    ];
    let first = Worktable::replay(&edits);
    drop(first);
    let rebuilt = Worktable::replay(&edits);
    assert_eq!(rebuilt.edits, edits);
    assert_eq!(
        rebuilt.rooms["model-portfolio"].effect,
        EffectState::Unavailable
    );
    let evidence = ValueObservation {
        comparison: None,
        setup_minutes: 4,
        creative_block_minutes: 55,
        recoveries: 1,
        discarded_tools: vec!["unreproducible-candidate".into()],
        adopted_tools: vec![],
    };
    assert_eq!(evidence.conclusion(), ValueConclusion::InsufficientEvidence);
}
