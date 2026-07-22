use sim_kernel::testing::bare_cx;

use crate::runtime::watch::{
    WatchInstallMode, bool_field, install_watch_stack, prove_dual_quorum, prove_glance_pager,
    prove_hold_last, prove_privacy_reaper,
};

#[test]
fn watch_stack_installs_modeled_wrist_base_and_sdk_lib() {
    let mut cx = bare_cx();

    install_watch_stack(&mut cx, WatchInstallMode::Modeled).expect("watch stack installs");

    assert!(
        cx.registry()
            .lib(&crate::lib_stream_device::device_stream_base_manifest_symbol())
            .is_some()
    );
    assert!(
        cx.registry()
            .lib(&crate::lib_stream_wrist::wrist_stream_manifest_symbol())
            .is_some()
    );
    assert!(
        cx.registry()
            .lib(&crate::runtime::watch::watch_stack_manifest_symbol())
            .is_some()
    );
}

#[test]
fn watch_glance_pager_recipe_runs_modeled() {
    let proof = prove_glance_pager().expect("glance pager proof runs");

    assert!(proof.modeled_source_monotone);
    assert!(proof.notification_sent);
    assert_eq!(proof.notification_lines, 2);
    assert!(proof.adapter_cells >= 3);
}

#[test]
fn watch_hold_last_recipe_runs_modeled() {
    let proof = prove_hold_last().expect("hold-last proof runs");

    assert!(proof.held_last);
    assert!(proof.stale);
    assert!(proof.dropped >= 2);
}

#[test]
fn watch_privacy_reaper_recipe_runs_modeled() {
    let proof = prove_privacy_reaper().expect("privacy reaper proof runs");

    assert!(proof.hr_evicted);
    assert!(proof.location_evicted);
    assert!(proof.content_evicted);
    assert!(proof.evicted >= 4);
}

#[test]
fn watch_dual_quorum_recipe_runs_modeled() {
    let proof = prove_dual_quorum().expect("dual quorum proof runs");

    assert!(proof.low_confidence);
    assert!(proof.confidence < 8_800);
    assert_eq!(proof.delta_bpm, 22);
}

#[cfg(feature = "cookbook")]
#[test]
fn watch_recipes_run_from_the_cookbook_directory() {
    use std::sync::Arc;

    use sim_cookbook::recipes_from_embedded;
    use sim_kernel::{
        CapabilityName, Cx, DefaultFactory, EagerPolicy, Expr, Symbol,
        macro_expand_eval_capability, read_construct_capability, read_eval_capability,
    };
    use sim_lib_cookbook::{LoadableLibList, run_recipe_with_loadable_libs};

    let cards = recipes_from_embedded(crate::runtime::watch::RECIPES).expect("recipes parse");
    let recipe_ids = ["glance-pager", "hold-last", "privacy-reaper", "dual-quorum"];
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    cx.grant(read_eval_capability());
    cx.grant(read_construct_capability());
    cx.grant(macro_expand_eval_capability());
    cx.grant(CapabilityName::new("cookbook.run.offline"));
    cx.grant(CapabilityName::new("cookbook.run.deterministic"));
    cx.grant(CapabilityName::new("cookbook.run.pure"));
    crate::runtime::install_core_runtime(&mut cx);
    let lisp = sim_codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id())
        .expect("lisp codec builds");
    cx.load_lib(&lisp).expect("lisp codec loads");
    install_watch_stack(&mut cx, WatchInstallMode::Modeled).expect("watch stack loads");

    let direct = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::qualified("watch/sdk", "hold-last"))),
            args: Vec::new(),
        })
        .expect("direct watch proof callable evals");
    let direct_expr = direct
        .object()
        .as_expr(&mut cx)
        .expect("direct proof callable returns expr");
    assert!(bool_field(&direct_expr, "held-last"));

    let (directory, diags) = crate::runtime::cookbook_directory::default_loadable_libs();
    assert!(diags.is_empty(), "directory diagnostics: {diags:?}");
    assert!(LoadableLibList::is_loaded(&cx, "watch/sdk"));

    for recipe_id in recipe_ids {
        let card = cards
            .iter()
            .find(|card| card.id.ends_with(&format!("/{recipe_id}")))
            .unwrap_or_else(|| panic!("missing recipe {recipe_id}"));
        let run = run_recipe_with_loadable_libs(&mut cx, &directory, card)
            .unwrap_or_else(|err| panic!("recipe {recipe_id} failed: {err}"));
        assert!(run.ok, "recipe {recipe_id} run: {run:?}");
        assert_eq!(run.forms, 1);
    }
}
