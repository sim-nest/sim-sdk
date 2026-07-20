use sim_kernel::testing::bare_cx;
use sim_lib_view_device::{DeviceCapability, DeviceTier};

use crate::runtime::reference_device::{
    bool_field, install_reference_device, prove_consent_without_kernel_grant, prove_route_swap,
    prove_two_rate, reference_glance_profile, reference_pose_receipt, reference_rich_profile,
    require_reference_pose,
};

#[test]
fn reference_profiles_have_rich_and_glance_tiers() {
    let rich = reference_rich_profile();
    let glance = reference_glance_profile();

    assert_eq!(rich.tier, DeviceTier::Rich);
    assert_eq!(glance.tier, DeviceTier::Actuator);
    assert!(
        rich.streams
            .iter()
            .any(|symbol| symbol.name.as_ref() == "pose")
    );
    assert!(
        glance
            .output
            .iter()
            .any(|symbol| symbol.name.as_ref() == "haptic")
    );
}

#[test]
fn reference_device_installs_profiles_and_stream_base() {
    let mut cx = bare_cx();

    install_reference_device(&mut cx).expect("reference device installs");

    assert!(
        cx.registry()
            .lib(&sim_lib_stream_device::device_stream_base_manifest_symbol())
            .is_some()
    );
    assert!(
        cx.registry()
            .lib(&crate::runtime::reference_device::reference_device_manifest_symbol())
            .is_some()
    );
    assert!(
        cx.registry()
            .value_by_symbol(&crate::runtime::reference_device::reference_rich_profile_symbol())
            .is_some()
    );
}

#[test]
fn device_two_rate_recipe_drops_and_holds() {
    let proof = prove_two_rate().expect("two-rate proof runs");

    assert_eq!(proof.encoder_calls, 1);
    assert!(
        proof.rich_dropped >= 2,
        "modeled samples should coalesce before one frame"
    );
    assert!(proof.rich_stale);
    assert_eq!(
        proof.glance_cells,
        sim_lib_view_device::GlanceBudget::mono_hud().cells
    );
    assert!(proof.modeled_stream_monotone);
}

#[test]
fn device_consent_recipe_fails_closed_and_reaps() {
    let mut cx = bare_cx();
    let receipt = reference_pose_receipt(7, 5);
    let empty_receipt = sim_lib_view_device::ConsentReceipt::new(
        Vec::new(),
        5,
        Vec::new(),
        crate::runtime::reference_device::reference_edge_id(),
        8,
    );

    assert!(require_reference_pose(&cx, &receipt).is_err());
    cx.grant(DeviceCapability::Pose.capability_name());
    assert!(require_reference_pose(&cx, &empty_receipt).is_err());
    assert!(require_reference_pose(&cx, &receipt).is_ok());

    let proof = prove_consent_without_kernel_grant(&bare_cx());
    assert!(proof.denied_without_kernel_grant);
    assert!(proof.denied_without_visible_grant);
    assert!(proof.sample_evicted);
    assert!(proof.content_evicted);
}

#[test]
fn device_route_swap_recipe_survives() {
    let proof = prove_route_swap().expect("route swap proof runs");

    assert!(proof.same_session_id);
    assert!(proof.same_ledger);
    assert!(proof.consent_survived);
    assert!(proof.events_advanced);
    assert!(proof.peer_surface_registered);
}

#[cfg(feature = "cookbook")]
#[test]
fn device_reference_recipes_run_from_the_cookbook_directory() {
    use std::sync::Arc;

    use sim_cookbook::recipes_from_embedded;
    use sim_kernel::{
        CapabilityName, Cx, DefaultFactory, EagerPolicy, Expr, Symbol,
        macro_expand_eval_capability, read_construct_capability, read_eval_capability,
    };
    use sim_lib_cookbook::{LoadableLibList, run_recipe_with_loadable_libs};

    let cards =
        recipes_from_embedded(crate::runtime::reference_device::RECIPES).expect("recipes parse");
    let recipe_ids = ["two-rate", "consent", "route-swap"];
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
    cx.load_lib(&crate::runtime::reference_device::ReferenceDeviceLib)
        .expect("reference lib loads");
    let direct = cx
        .eval_expr(Expr::Call {
            operator: Box::new(Expr::Symbol(Symbol::qualified(
                "device/reference",
                "two-rate",
            ))),
            args: Vec::new(),
        })
        .expect("direct proof callable evals");
    let direct_expr = direct
        .object()
        .as_expr(&mut cx)
        .expect("direct proof callable returns expr");
    assert!(bool_field(&direct_expr, "modeled-stream-monotone"));
    let (directory, diags) = crate::runtime::cookbook_directory::default_loadable_libs();
    assert!(diags.is_empty(), "directory diagnostics: {diags:?}");
    assert!(LoadableLibList::is_loaded(&cx, "device/reference"));

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

    let consent = cards
        .iter()
        .find(|card| card.id.ends_with("/consent"))
        .expect("consent recipe");
    let run = run_recipe_with_loadable_libs(&mut cx, &directory, consent).unwrap();
    let rendered = run.results.first().expect("rendered consent proof");
    assert!(
        rendered.contains("denied-without-kernel-grant"),
        "rendered proof should name fail-closed consent: {rendered}"
    );
}

#[test]
fn proof_callables_return_proof_maps() {
    let mut cx = bare_cx();
    install_reference_device(&mut cx).expect("reference device installs");
    let value = cx
        .registry()
        .value_by_symbol(&crate::runtime::reference_device::reference_rich_profile_symbol())
        .expect("profile value")
        .clone();
    let expr = value
        .object()
        .as_expr(&mut cx)
        .expect("profile value encodes");
    assert!(matches!(expr, sim_kernel::Expr::Map(_)));

    let proof_expr = prove_consent_without_kernel_grant(&bare_cx()).to_expr();
    assert!(bool_field(&proof_expr, "sample-evicted"));
}
