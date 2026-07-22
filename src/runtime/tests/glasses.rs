use sim_kernel::testing::bare_cx;

use crate::runtime::glasses::{
    GlassesInstallMode, install_glasses_stack, modeled_asr_site_symbol, prove_co_use,
    prove_halo_glance, prove_review_in_space, prove_two_rate, prove_voice_site,
};

#[test]
fn glasses_stack_installs_device_xr_sdk_and_modeled_asr_site() {
    let mut cx = bare_cx();

    install_glasses_stack(&mut cx, GlassesInstallMode::Modeled).expect("glasses stack installs");

    assert!(
        cx.registry()
            .lib(&crate::lib_stream_device::device_stream_base_manifest_symbol())
            .is_some()
    );
    assert!(
        cx.registry()
            .lib(&crate::lib_stream_xr::xr_stream_manifest_symbol())
            .is_some()
    );
    assert!(
        cx.registry()
            .lib(&crate::runtime::glasses::glasses_stack_manifest_symbol())
            .is_some()
    );
    assert!(
        cx.registry()
            .site_by_symbol(&modeled_asr_site_symbol())
            .and_then(|site| site.object().as_eval_fabric())
            .is_some()
    );
}

#[test]
fn glasses_viture_two_rate_recipe_runs_modeled() {
    let proof = prove_two_rate().expect("two-rate proof runs");

    assert_eq!(proof.content_encodes, 1);
    assert!(proof.dropped >= 2);
    assert!(proof.stale);
    assert_eq!(proof.clamped_predict_ms, 12);
    assert!(proof.held_after_clamp);
}

#[test]
fn glasses_halo_glance_pager_recipe_runs_modeled() {
    let proof = prove_halo_glance().expect("Halo glance proof runs");

    assert!(proof.glance_scene);
    assert!(proof.small_delta);
    assert!(proof.delta_cells <= 2);
    assert!(proof.delta_bytes < proof.budget_bytes);
    assert!(proof.glyph_flash_ack);
}

#[test]
fn glasses_voice_site_recipe_runs_modeled() {
    let mut cx = bare_cx();
    cx.grant(crate::lib_view_spatial::glasses_mic_capability());
    install_glasses_stack(&mut cx, GlassesInstallMode::Modeled).expect("glasses stack installs");

    let proof = prove_voice_site(&mut cx).expect("voice-site proof runs");

    assert!(proof.site_exported);
    assert!(proof.session_bound);
    assert!(proof.by_reference);
    assert_eq!(proof.intent_kind, "invoke");
}

#[test]
fn glasses_co_use_recipe_runs_modeled() {
    let proof = prove_co_use().expect("co-use proof runs");

    assert_eq!(proof.peers, 2);
    assert_eq!(proof.broadcasts, 2);
    assert_eq!(proof.ledger_rows, 1);
    assert!(proof.viture_panel_acked);
}

#[test]
fn glasses_review_in_space_recipe_runs_modeled() {
    let proof = prove_review_in_space().expect("review proof runs");

    assert!(proof.warrant_pager);
    assert!(proof.approved);
    assert!(proof.packet_bound);
}

#[cfg(feature = "cookbook")]
#[test]
fn glasses_recipes_run_from_the_cookbook_directory() {
    use std::sync::Arc;

    use sim_cookbook::recipes_from_embedded;
    use sim_kernel::{
        CapabilityName, Cx, DefaultFactory, EagerPolicy, Expr, Symbol,
        macro_expand_eval_capability, read_construct_capability, read_eval_capability,
    };
    use sim_lib_cookbook::{LoadableLibList, run_recipe_with_loadable_libs};

    let cards = recipes_from_embedded(crate::runtime::glasses::RECIPES).expect("recipes parse");
    let recipe_ids = [
        "viture-two-rate",
        "halo-glance-pager",
        "voice-site",
        "co-use",
        "review-in-space",
    ];
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    cx.grant(read_eval_capability());
    cx.grant(read_construct_capability());
    cx.grant(macro_expand_eval_capability());
    cx.grant(CapabilityName::new("cookbook.run.offline"));
    cx.grant(CapabilityName::new("cookbook.run.deterministic"));
    cx.grant(CapabilityName::new("cookbook.run.pure"));
    cx.grant(crate::lib_view_spatial::glasses_mic_capability());
    crate::runtime::install_core_runtime(&mut cx);
    let lisp = sim_codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id())
        .expect("lisp codec builds");
    cx.load_lib(&lisp).expect("lisp codec loads");
    install_glasses_stack(&mut cx, GlassesInstallMode::Modeled).expect("glasses stack loads");

    let direct = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::qualified("glasses/sdk", "co-use"))),
            args: Vec::new(),
        })
        .expect("direct glasses proof callable evals");
    let direct_expr = direct
        .object()
        .as_expr(&mut cx)
        .expect("direct proof callable returns expr");
    assert_eq!(
        sim_value::access::field_f64(&direct_expr, "peers"),
        Some(2.0)
    );

    let (directory, diags) = crate::runtime::cookbook_directory::default_loadable_libs();
    assert!(diags.is_empty(), "directory diagnostics: {diags:?}");
    assert!(LoadableLibList::is_loaded(&cx, "glasses/sdk"));

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

#[cfg(all(feature = "glasses-viture", feature = "glasses-halo"))]
#[test]
fn glasses_hardware_modes_are_explicitly_feature_gated() {
    let mut cx = bare_cx();
    install_glasses_stack(&mut cx, GlassesInstallMode::Both)
        .expect("both provider features accept the combined install mode");
}
