#![cfg(all(feature = "cookbook", feature = "core", feature = "shape"))]

use std::sync::Arc;

use sim::kernel::{Args, DefaultFactory, EagerPolicy, Expr, Symbol};

#[test]
fn seeded_cookbook_is_visible_in_runtime() {
    let mut cx = sim::kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    sim::runtime::install_core_runtime(&mut cx);
    sim_lib_cookbook::install_seeded_cookbook_lib(&mut cx).unwrap();

    let value = cx
        .call_function(
            &Symbol::qualified("cookbook", "list"),
            Args::new(Vec::new()),
        )
        .unwrap();
    let Expr::List(items) = value.object().as_expr(&mut cx).unwrap() else {
        panic!("cookbook:list should return a list");
    };
    assert!(!items.is_empty());
}

#[cfg(all(
    feature = "codec-json",
    feature = "codec-lisp",
    feature = "cookbook-all",
    feature = "numbers-arith",
    feature = "numbers-f64",
    feature = "stream-core"
))]
#[test]
fn cookbook_all_seeded_recipes_use_product_directory_under_all_features() {
    let mut cx = sim::kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    sim::runtime::install_core_runtime(&mut cx);
    let lisp = sim::codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    let json = sim::codec_json::JsonCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&json).unwrap();
    sim::lib_stream_core::install_stream_core_shapes_lib(&mut cx).unwrap();
    cx.grant(sim::kernel::read_eval_capability());
    cx.grant(sim::kernel::read_construct_capability());

    let (directory, diagnostics) = sim::runtime::cookbook_directory::default_loadable_libs();
    assert!(
        diagnostics.is_empty(),
        "unresolved directory rows: {diagnostics:?}"
    );

    let store = sim_lib_cookbook::seeded_recipe_store().unwrap();
    assert!(!store.is_empty());
    let mut ran = 0usize;
    for card in store.cards() {
        match sim_lib_cookbook::run_recipe_with_loadable_libs(&mut cx, &directory, card) {
            Ok(run) => {
                assert!(run.ok, "seed recipe {} failed: {run:?}", card.id);
                ran += 1;
            }
            Err(sim::kernel::Error::Eval(message))
                if message.contains("descriptor: requires not in catalog") => {}
            Err(err) => panic!("seed recipe {} errored: {err:?}", card.id),
        }
    }
    assert!(ran > 0);
}
