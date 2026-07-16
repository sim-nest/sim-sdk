use sim_kernel::testing::bare_cx as cx;
use sim_lib_forge::{run_eval, standard_eval_arms, standard_eval_corpus};

#[test]
fn forge_eval_report_records_cache_replay_and_downshift_claims() {
    let mut cx = cx();
    let corpus = standard_eval_corpus();
    let report = run_eval(&mut cx, &corpus, &standard_eval_arms()).unwrap();
    let raw = report.metrics("raw-prose-baseline").unwrap();
    let cached = report.metrics("compiled-cached").unwrap();
    let replay = report.metrics("identical-request-replay").unwrap();
    let downshift = report.metrics("compiled-cached-downshifted").unwrap();

    assert_eq!(corpus.len(), 4);
    assert_eq!(cached.compiler_calls, 0);
    assert_eq!(replay.execution_calls, 0);
    assert_eq!(replay.replay_hits, corpus.len() as u64);
    assert!(downshift.tokens < raw.tokens);
    assert!(downshift.accuracy >= raw.accuracy);
}
