#[cfg(any(
    feature = "table-hash",
    feature = "table-lazy",
    feature = "table-override"
))]
use super::support::eval_cx;

#[cfg(all(
    feature = "table-override",
    any(feature = "codec-lisp", feature = "codec-algol")
))]
use sim_codec::{Input, decode_with_codec};
#[cfg(all(feature = "table-override", feature = "codec-algol"))]
use sim_codec_algol::AlgolCodecLib;
#[cfg(all(feature = "table-override", feature = "codec-lisp"))]
use sim_codec_lisp::{LispCodecLib, encode_object_lisp};
#[cfg(feature = "table-override")]
use sim_kernel::{
    CapabilitySet, EncodeOptions, EncodePosition, Expr, ObjectCompat, ReadPolicy, Symbol, Table,
    TrustLevel, read_construct_capability,
};
#[cfg(all(feature = "table-lazy", not(feature = "table-override")))]
use sim_kernel::{Expr, ObjectCompat, Symbol, Table};

#[test]
#[cfg(feature = "table-hash")]
fn core_runtime_installs_hash_table_backend() {
    let mut cx = eval_cx();
    cx.table_registry_mut().set_active("hash").unwrap();
    assert_eq!(cx.table_registry().active(), "hash");
}

#[test]
#[cfg(feature = "table-lazy")]
fn core_runtime_installs_lazy_table_backend() {
    let mut cx = eval_cx();
    cx.table_registry_mut().set_active("lazy").unwrap();
    assert_eq!(cx.table_registry().active(), "lazy");
}

#[test]
#[cfg(feature = "table-lazy")]
fn lazy_table_backend_caches_loader_results_when_accessed() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let mut cx = eval_cx();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let table = sim_table_lazy::LazyTable::with_loaders(vec![(
        Symbol::new("x"),
        Arc::new(move |cx: &mut sim_kernel::Cx| {
            counter.fetch_add(1, Ordering::SeqCst);
            cx.factory()
                .number_literal(Symbol::qualified("numbers", "f64"), "7".to_owned())
        }),
    )]);

    assert!(table.has(&mut cx, Symbol::new("x")).unwrap());
    assert_eq!(table.len(&mut cx).unwrap(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let first = table.get(&mut cx, Symbol::new("x")).unwrap();
    let second = table.get(&mut cx, Symbol::new("x")).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(first, second);

    let expr = table.as_expr(&mut cx).unwrap();
    assert!(matches!(expr, Expr::Map(entries) if entries.len() == 1));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "table-override")]
fn read_policy(capabilities: &[sim_kernel::CapabilityName]) -> ReadPolicy {
    ReadPolicy {
        trust: TrustLevel::Untrusted,
        capabilities: capabilities
            .iter()
            .cloned()
            .fold(CapabilitySet::new(), |set, capability| {
                set.grant(capability)
            }),
    }
}

#[cfg(feature = "codec-lisp")]
#[cfg(feature = "table-override")]
fn lower_lisp_eval_surface(expr: Expr) -> Expr {
    match expr {
        Expr::List(items) if items.len() > 1 => {
            let mut items = items
                .into_iter()
                .map(lower_lisp_eval_surface)
                .collect::<Vec<_>>();
            let operator = Box::new(items.remove(0));
            Expr::Call {
                operator,
                args: items,
            }
        }
        Expr::List(items) => Expr::List(items.into_iter().map(lower_lisp_eval_surface).collect()),
        Expr::Vector(items) => {
            Expr::Vector(items.into_iter().map(lower_lisp_eval_surface).collect())
        }
        Expr::Map(entries) => Expr::Map(
            entries
                .into_iter()
                .map(|(key, value)| (lower_lisp_eval_surface(key), lower_lisp_eval_surface(value)))
                .collect(),
        ),
        Expr::Set(items) => Expr::Set(items.into_iter().map(lower_lisp_eval_surface).collect()),
        Expr::Block(items) => Expr::Block(items.into_iter().map(lower_lisp_eval_surface).collect()),
        Expr::Annotated { expr, annotations } => Expr::Annotated {
            expr: Box::new(lower_lisp_eval_surface(*expr)),
            annotations: annotations
                .into_iter()
                .map(|(name, value)| (name, lower_lisp_eval_surface(value)))
                .collect(),
        },
        Expr::Extension { tag, payload } => Expr::Extension {
            tag,
            payload: Box::new(lower_lisp_eval_surface(*payload)),
        },
        other => other,
    }
}

#[cfg(feature = "table-override")]
fn number_canonical(expr: Expr) -> String {
    match expr {
        Expr::Number(number) => number.canonical,
        other => panic!("expected number expression, found {other:?}"),
    }
}

#[test]
#[cfg(feature = "table-override")]
fn core_runtime_installs_override_table_class() {
    let cx = eval_cx();
    assert!(
        cx.registry()
            .class_by_symbol(&Symbol::new("OverrideTable"))
            .is_some()
    );
}

#[test]
#[cfg(feature = "table-override")]
fn override_table_class_eval_front_shadows_and_writes_to_front() {
    let mut cx = eval_cx();
    let front = cx
        .new_table(vec![(
            Symbol::new("b"),
            cx.factory()
                .number_literal(Symbol::qualified("numbers", "f64"), "10".to_owned())
                .unwrap(),
        )])
        .unwrap();
    let back = cx
        .new_table(vec![
            (
                Symbol::new("a"),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "1".to_owned())
                    .unwrap(),
            ),
            (
                Symbol::new("b"),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "2".to_owned())
                    .unwrap(),
            ),
        ])
        .unwrap();
    cx.env_mut().define(Symbol::new("front"), front.clone());
    cx.env_mut().define(Symbol::new("back"), back.clone());

    let view = cx
        .eval_expr(super::support::call_expr(
            Symbol::new("OverrideTable"),
            vec![
                Expr::Symbol(Symbol::new("front")),
                Expr::Symbol(Symbol::new("back")),
            ],
        ))
        .unwrap();
    let table = view.object().as_table_impl().unwrap();
    assert_eq!(
        table
            .get(&mut cx, Symbol::new("b"))
            .unwrap()
            .object()
            .as_expr(&mut cx)
            .unwrap(),
        Expr::Number(sim_kernel::NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "10".to_owned(),
        })
    );
    assert_eq!(
        table
            .get(&mut cx, Symbol::new("a"))
            .unwrap()
            .object()
            .as_expr(&mut cx)
            .unwrap(),
        Expr::Number(sim_kernel::NumberLiteral {
            domain: Symbol::qualified("numbers", "f64"),
            canonical: "1".to_owned(),
        })
    );

    let value_c = cx
        .factory()
        .number_literal(Symbol::qualified("numbers", "f64"), "3".to_owned())
        .unwrap();
    table.set(&mut cx, Symbol::new("c"), value_c).unwrap();
    assert!(
        front
            .object()
            .as_table_impl()
            .unwrap()
            .has(&mut cx, Symbol::new("c"))
            .unwrap()
    );
    assert!(
        !back
            .object()
            .as_table_impl()
            .unwrap()
            .has(&mut cx, Symbol::new("c"))
            .unwrap()
    );
}

#[test]
#[cfg(all(
    feature = "table-override",
    feature = "codec-lisp",
    feature = "codec-algol"
))]
fn override_table_class_works_from_lisp_algol_and_read_construct() {
    let mut cx = eval_cx();
    let lisp_codec_id = cx.registry_mut().fresh_codec_id();
    let lisp = LispCodecLib::new(lisp_codec_id).unwrap();
    cx.load_lib(&lisp).unwrap();
    let algol = AlgolCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&algol).unwrap();

    let front = cx
        .new_table(vec![(
            Symbol::new("b"),
            cx.factory()
                .number_literal(Symbol::qualified("numbers", "f64"), "10".to_owned())
                .unwrap(),
        )])
        .unwrap();
    let back = cx
        .new_table(vec![
            (
                Symbol::new("a"),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "1".to_owned())
                    .unwrap(),
            ),
            (
                Symbol::new("b"),
                cx.factory()
                    .number_literal(Symbol::qualified("numbers", "f64"), "2".to_owned())
                    .unwrap(),
            ),
        ])
        .unwrap();
    cx.env_mut().define(Symbol::new("front"), front.clone());
    cx.env_mut().define(Symbol::new("back"), back.clone());

    let lisp_expr = decode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text("(OverrideTable front back)".to_owned()),
        ReadPolicy::default(),
    )
    .unwrap();
    let lisp_value = cx.eval_expr(lower_lisp_eval_surface(lisp_expr)).unwrap();
    assert!(lisp_value.object().as_table_impl().is_some());

    let algol_expr = decode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "algol"),
        Input::Text("OverrideTable(front, back)".to_owned()),
        ReadPolicy::default(),
    )
    .unwrap();
    let algol_value = cx.eval_expr(algol_expr).unwrap();
    assert!(algol_value.object().as_table_impl().is_some());

    cx.grant(read_construct_capability());
    let encoded = encode_object_lisp(
        &mut sim_kernel::WriteCx {
            cx: &mut cx,
            codec: lisp_codec_id,
            options: EncodeOptions {
                position: EncodePosition::Quote,
                ..Default::default()
            },
        },
        lisp_value,
    )
    .unwrap();
    assert!(encoded.starts_with("#(OverrideTable "));

    let decoded = decode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text(encoded),
        read_policy(&[read_construct_capability()]),
    )
    .unwrap();
    let Expr::Map(_) = decoded else {
        panic!("expected merged override table to decode as a map");
    };
    let decoded_table = cx.eval_expr(decoded).unwrap();
    let decoded_table = decoded_table.object().as_table_impl().unwrap();
    assert_eq!(
        number_canonical(
            decoded_table
                .get(&mut cx, Symbol::new("a"))
                .unwrap()
                .object()
                .as_expr(&mut cx)
                .unwrap()
        ),
        "1"
    );
    assert_eq!(
        number_canonical(
            decoded_table
                .get(&mut cx, Symbol::new("b"))
                .unwrap()
                .object()
                .as_expr(&mut cx)
                .unwrap()
        ),
        "10"
    );
}

#[test]
fn registry_catalog_view_exposes_runtime_table_dir() {
    let mut cx = super::support::eval_cx();
    let symbol = sim_kernel::Symbol::new("runtime-catalog-view-value");
    let value = cx.factory().bool(true).unwrap();
    cx.registry_mut()
        .register_value(symbol.clone(), value)
        .unwrap();

    assert!(sim_kernel::catalog::registry_catalog_view(&mut cx).is_err());
    cx.grant(sim_kernel::registry_catalog_read_capability());

    let view = sim_kernel::catalog::registry_catalog_view(&mut cx).unwrap();
    let root = view.object().as_table_impl().unwrap();
    assert!(
        root.has(&mut cx, sim_kernel::Symbol::new("registry"))
            .unwrap()
    );

    let registry = view
        .object()
        .as_dir()
        .unwrap()
        .opendir(&mut cx, sim_kernel::Symbol::new("registry"))
        .unwrap()
        .unwrap();
    let exports = registry
        .object()
        .as_dir()
        .unwrap()
        .opendir(&mut cx, sim_kernel::Symbol::new("exports"))
        .unwrap()
        .unwrap();
    let export_rows = exports
        .object()
        .as_table_impl()
        .unwrap()
        .entries(&mut cx)
        .unwrap();

    assert!(export_rows.into_iter().any(|(_, row)| {
        matches!(
            row.object().as_expr(&mut cx).unwrap(),
            sim_kernel::Expr::Map(entries)
                if entries.iter().any(|(key, value)| {
                    key == &sim_kernel::Expr::Symbol(sim_kernel::Symbol::new("symbol"))
                        && value == &sim_kernel::Expr::Symbol(symbol.clone())
                })
        )
    }));
}
