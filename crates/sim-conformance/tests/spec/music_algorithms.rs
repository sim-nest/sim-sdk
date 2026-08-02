use sim::{discrete_search, music_consonance, music_counterpoint, pitch_ratio, signal};

use crate::support::CONFORMANCE_CONTRACT;

pub(crate) const MUSIC_ALGORITHMS_PATH: &str =
    "crates/sim-conformance/tests/spec/music_algorithms.rs";

const FOUNDRY_LOAD_RECIPE: &str =
    include_str!("../../../../recipes/music-algorithms/foundry-plan/input.lisp");

#[test]
fn frozen_music_candidate_graph_composes_through_the_sdk() {
    assert!(CONFORMANCE_CONTRACT.contains(MUSIC_ALGORITHMS_PATH));

    let transform = signal::TransformPlan::new(signal::TransformKind::Fft, 4);
    transform
        .validate()
        .expect("explicit signal transform plan");

    let control = discrete_search::SearchControl::default()
        .with_max_work(500_000)
        .with_max_frontier(20_000)
        .with_max_results(8)
        .with_seed(42);
    assert_eq!(control.max_work, Some(500_000));
    assert_eq!(control.max_frontier, Some(20_000));
    assert_eq!(control.max_results, Some(8));
    assert_eq!(control.seed, 42);

    let fifth = pitch_ratio::PitchRatio::new(3, 2).expect("exact fifth");
    assert_eq!((fifth.numerator(), fifth.denominator()), (3, 2));
    assert_eq!(
        music_consonance::ConsonancePolicy::default()
            .pitch_models
            .len(),
        4
    );
    music_counterpoint::RuleSet::open()
        .validate()
        .expect("open counterpoint rules");

    for library in [
        "sim-lib-numbers-signal",
        "sim-lib-discrete-search",
        "sim-lib-pitch-ratio",
        "sim-lib-music-consonance",
        "sim-lib-music-counterpoint",
    ] {
        assert!(
            FOUNDRY_LOAD_RECIPE.contains(&format!("(load \"{library}\")")),
            "foundry recipe must load {library}"
        );
    }
    for plan_fact in [
        "(music/algorithm-plan",
        ":analysis '(pitch-track beat key chords)",
        ":transform '(voice-lead harmonize counterpoint)",
        ":render '(smf wav)",
        ":budget {:work 500000 :frontier 20000 :results 8 :seed 42}",
        "(realize plan :at 'local)",
    ] {
        assert!(
            FOUNDRY_LOAD_RECIPE.contains(plan_fact),
            "foundry recipe must retain {plan_fact}"
        );
    }
}
