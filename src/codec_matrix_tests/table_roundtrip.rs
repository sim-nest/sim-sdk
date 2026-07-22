#[cfg(any(
    feature = "table-lazy",
    feature = "table-override",
    feature = "table-remote"
))]
use std::sync::Arc;
#[cfg(feature = "table-lazy")]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "table-fs")]
use std::time::{SystemTime, UNIX_EPOCH};

use sim_codec::{encode_value_with_codec, encode_with_codec};
#[cfg(any(feature = "table-db", feature = "table-fs", feature = "table-remote"))]
use sim_kernel::CapabilityName;
use sim_kernel::{AssocTable, Cx, EncodeOptions, Error, Expr, Result, Symbol, Value};

#[cfg(any(feature = "table-db", feature = "table-remote"))]
use sim_table_db::install_db_dir_lib;
#[cfg(feature = "table-fs")]
use sim_table_fs::install_fs_dir_lib;
#[cfg(feature = "table-remote")]
use sim_table_remote::{remote_dir_value, wrap_remote_table_site};
#[cfg(feature = "table-remote")]
use sim_kernel::eval_remote_capability;
#[cfg(feature = "table-override")]
use sim_table_override::OverrideTable;

#[cfg(feature = "table-remote")]
use sim_lib_server::{EvalSite, LocalEvalSite, ServerAddress};

#[cfg(feature = "table-lazy")]
use sim_table_lazy::{LazyTable, ValueLoader};

use super::support::{codec_symbols, cx as test_cx, decode_once};

#[cfg(feature = "table-fs")]
fn table_fs_capability() -> CapabilityName {
    CapabilityName::new("table.fs")
}

#[cfg(feature = "table-fs")]
fn table_fs_read_capability() -> CapabilityName {
    CapabilityName::new("table.fs.read")
}

#[cfg(feature = "table-fs")]
fn table_fs_write_capability() -> CapabilityName {
    CapabilityName::new("table.fs.write")
}

#[cfg(feature = "table-fs")]
fn table_fs_mkdir_capability() -> CapabilityName {
    CapabilityName::new("table.fs.mkdir")
}

#[cfg(feature = "table-fs")]
fn table_fs_rmdir_capability() -> CapabilityName {
    CapabilityName::new("table.fs.rmdir")
}

#[cfg(any(feature = "table-db", feature = "table-remote"))]
fn table_db_capability() -> CapabilityName {
    CapabilityName::new("table.db")
}

#[cfg(any(feature = "table-db", feature = "table-remote"))]
fn table_db_read_capability() -> CapabilityName {
    CapabilityName::new("table.db.read")
}

#[cfg(any(feature = "table-db", feature = "table-remote"))]
fn table_db_write_capability() -> CapabilityName {
    CapabilityName::new("table.db.write")
}

#[cfg(any(feature = "table-db", feature = "table-remote"))]
fn table_db_mkdir_capability() -> CapabilityName {
    CapabilityName::new("table.db.mkdir")
}

#[cfg(any(feature = "table-db", feature = "table-remote"))]
fn table_db_rmdir_capability() -> CapabilityName {
    CapabilityName::new("table.db.rmdir")
}

#[cfg(feature = "table-remote")]
fn table_remote_capability() -> CapabilityName {
    CapabilityName::new("table.remote")
}

fn number_expr(text: &str) -> Expr {
    Expr::Number(sim_kernel::NumberLiteral {
        domain: Symbol::qualified("numbers", "f64"),
        canonical: text.to_owned(),
    })
}

fn number_value(cx: &mut Cx, text: &str) -> Value {
    cx.factory()
        .number_literal(Symbol::qualified("numbers", "f64"), text.to_owned())
        .unwrap()
}

fn symbol_expr(name: &str) -> Expr {
    Expr::Symbol(Symbol::new(name))
}

fn table_expr(entries: &[(&str, &str)]) -> Expr {
    Expr::Map(
        entries
            .iter()
            .map(|(key, value)| (symbol_expr(key), number_expr(value)))
            .collect(),
    )
}

fn assert_table_roundtrip(cx: &mut Cx, value: &Value, expected: &Expr) {
    for codec in codec_symbols() {
        let encoded = encode_value_with_codec(cx, &codec, value, EncodeOptions::default()).unwrap();
        let baseline = encode_with_codec(cx, &codec, expected, EncodeOptions::default()).unwrap();
        assert_eq!(
            encoded, baseline,
            "backend value changed wire form for {codec}"
        );

        let decoded = decode_once(cx, &codec, encoded);
        assert!(
            decoded.canonical_eq(expected),
            "codec {codec} decoded {decoded:?} instead of {expected:?}"
        );
        let Expr::Map(_) = decoded else {
            panic!("codec {codec} did not decode the table back to Expr::Map");
        };

        let rebuilt = expr_to_dense_value(cx, &decoded).unwrap();
        assert!(rebuilt.object().as_table_impl().is_some());
        assert!(rebuilt.object().downcast_ref::<AssocTable>().is_some());
    }
}

#[test]
fn assoc_table_roundtrips_through_every_codec() {
    let mut cx = test_cx();
    let two = number_value(&mut cx, "2.5");
    let one = number_value(&mut cx, "1.25");
    let value = cx
        .new_table(vec![(Symbol::new("b"), two), (Symbol::new("a"), one)])
        .unwrap();
    let expected = table_expr(&[("b", "2.5"), ("a", "1.25")]);
    assert_table_roundtrip(&mut cx, &value, &expected);
}

#[test]
#[cfg(feature = "table-hash")]
fn hash_table_roundtrips_like_assoc_with_canonical_order() {
    let mut cx = test_cx();
    cx.table_registry_mut().set_active("hash").unwrap();
    let two = number_value(&mut cx, "2.5");
    let one = number_value(&mut cx, "1.25");
    let value = cx
        .new_table(vec![(Symbol::new("b"), two), (Symbol::new("a"), one)])
        .unwrap();
    let expected = table_expr(&[("b", "2.5"), ("a", "1.25")]);
    assert_table_roundtrip(&mut cx, &value, &expected);
}

#[test]
#[cfg(feature = "table-override")]
fn override_table_roundtrips_as_merged_front_wins_view() {
    let mut cx = test_cx();
    let front_ten = number_value(&mut cx, "10.5");
    let front = cx.new_table(vec![(Symbol::new("b"), front_ten)]).unwrap();
    let back_one = number_value(&mut cx, "1.25");
    let back_two = number_value(&mut cx, "2.5");
    let back = cx
        .new_table(vec![
            (Symbol::new("a"), back_one),
            (Symbol::new("b"), back_two),
        ])
        .unwrap();
    let value = cx
        .factory()
        .opaque(Arc::new(OverrideTable::new(vec![front, back]).unwrap()))
        .unwrap();
    let expected = table_expr(&[("b", "10.5"), ("a", "1.25")]);
    assert_table_roundtrip(&mut cx, &value, &expected);
}

#[test]
#[cfg(feature = "table-lazy")]
fn lazy_table_roundtrips_and_forces_values_on_encode() {
    let mut cx = test_cx();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let loader: ValueLoader = Arc::new(move |cx: &mut Cx| {
        counter.fetch_add(1, Ordering::SeqCst);
        cx.factory()
            .number_literal(Symbol::qualified("numbers", "f64"), "7.5".to_owned())
    });
    let value = cx
        .factory()
        .opaque(Arc::new(LazyTable::with_loaders(vec![
            (Symbol::new("b"), loader),
            (
                Symbol::new("a"),
                Arc::new(|cx: &mut Cx| {
                    cx.factory()
                        .number_literal(Symbol::qualified("numbers", "f64"), "3.25".to_owned())
                }),
            ),
        ])))
        .unwrap();

    let expected = table_expr(&[("b", "7.5"), ("a", "3.25")]);
    assert_table_roundtrip(&mut cx, &value, &expected);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
#[cfg(feature = "table-lazy")]
fn lazy_table_loader_runs_exactly_once_during_encode() {
    let mut cx = test_cx();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let loader: ValueLoader = Arc::new(move |cx: &mut Cx| {
        counter.fetch_add(1, Ordering::SeqCst);
        cx.factory()
            .number_literal(Symbol::qualified("numbers", "f64"), "11.5".to_owned())
    });
    let value = cx
        .factory()
        .opaque(Arc::new(LazyTable::with_loaders(vec![(
            Symbol::new("x"),
            loader,
        )])))
        .unwrap();

    let encoded = encode_value_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        &value,
        EncodeOptions::default(),
    )
    .unwrap();
    let baseline = encode_with_codec(
        &mut cx,
        &Symbol::qualified("codec", "lisp"),
        &table_expr(&[("x", "11.5")]),
        EncodeOptions::default(),
    )
    .unwrap();
    assert_eq!(encoded, baseline);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
#[cfg(feature = "table-fs")]
fn fs_dir_leaf_entries_roundtrip_without_subtables() {
    let mut cx = test_cx();
    grant(
        &mut cx,
        &[
            table_fs_capability(),
            table_fs_read_capability(),
            table_fs_write_capability(),
            table_fs_mkdir_capability(),
            table_fs_rmdir_capability(),
        ],
    );

    let root = temp_path("codec-matrix-fs");
    let value = install_fs_dir_lib(&mut cx, root.to_str().unwrap()).unwrap();
    let table = value.object().as_table_impl().unwrap();
    let dir = value.object().as_dir().unwrap();
    let two = number_value(&mut cx, "2.5");
    table.set(&mut cx, Symbol::new("b"), two).unwrap();
    let one = number_value(&mut cx, "1.25");
    table.set(&mut cx, Symbol::new("a"), one).unwrap();
    let child = dir.mkdir(&mut cx, Symbol::new("child")).unwrap();
    let nine = number_value(&mut cx, "9.75");
    child
        .object()
        .as_table_impl()
        .unwrap()
        .set(&mut cx, Symbol::new("z"), nine)
        .unwrap();

    let expected = table_expr(&[("b", "2.5"), ("a", "1.25")]);
    assert_table_roundtrip(&mut cx, &value, &expected);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[cfg(feature = "table-db")]
fn db_dir_leaf_entries_roundtrip_without_subtables() {
    let mut cx = test_cx();
    grant(
        &mut cx,
        &[
            table_db_capability(),
            table_db_read_capability(),
            table_db_write_capability(),
            table_db_mkdir_capability(),
            table_db_rmdir_capability(),
        ],
    );

    let value = install_db_dir_lib(&mut cx).unwrap();
    let table = value.object().as_table_impl().unwrap();
    let dir = value.object().as_dir().unwrap();
    let two = number_value(&mut cx, "2.5");
    table.set(&mut cx, Symbol::new("b"), two).unwrap();
    let one = number_value(&mut cx, "1.25");
    table.set(&mut cx, Symbol::new("a"), one).unwrap();
    let child = dir.mkdir(&mut cx, Symbol::new("child")).unwrap();
    let nine = number_value(&mut cx, "9.75");
    child
        .object()
        .as_table_impl()
        .unwrap()
        .set(&mut cx, Symbol::new("z"), nine)
        .unwrap();

    let expected = table_expr(&[("b", "2.5"), ("a", "1.25")]);
    assert_table_roundtrip(&mut cx, &value, &expected);
}

#[test]
#[cfg(feature = "table-remote")]
fn remote_dir_roundtrips_like_leaf_view() {
    let mut cx = test_cx();
    grant(
        &mut cx,
        &[
            table_db_capability(),
            table_db_read_capability(),
            table_db_write_capability(),
            table_db_mkdir_capability(),
            table_db_rmdir_capability(),
            table_remote_capability(),
            eval_remote_capability(),
        ],
    );

    let root = install_db_dir_lib(&mut cx).unwrap();
    let table = root.object().as_table_impl().unwrap();
    let two = number_value(&mut cx, "2.5");
    table.set(&mut cx, Symbol::new("b"), two).unwrap();
    let one = number_value(&mut cx, "1.25");
    table.set(&mut cx, Symbol::new("a"), one).unwrap();
    root.object()
        .as_dir()
        .unwrap()
        .mkdir(&mut cx, Symbol::new("child"))
        .unwrap();

    let inner: Arc<dyn EvalSite> = Arc::new(LocalEvalSite::new(
        ServerAddress::Local,
        vec![Symbol::qualified("codec", "binary")],
    ));
    let wrapped = wrap_remote_table_site(inner, root);
    let value = remote_dir_value(&mut cx, wrapped, Symbol::qualified("codec", "binary")).unwrap();

    let expected = table_expr(&[("b", "2.5"), ("a", "1.25")]);
    assert_table_roundtrip(&mut cx, &value, &expected);
}

fn expr_to_dense_value(cx: &mut Cx, expr: &Expr) -> Result<Value> {
    match expr {
        Expr::Nil => cx.factory().nil(),
        Expr::Bool(value) => cx.factory().bool(*value),
        Expr::Number(number) => cx
            .factory()
            .number_literal(number.domain.clone(), number.canonical.clone()),
        Expr::Symbol(symbol) => cx.factory().symbol(symbol.clone()),
        Expr::String(text) => cx.factory().string(text.clone()),
        Expr::Bytes(bytes) => cx.factory().bytes(bytes.clone()),
        Expr::List(items) | Expr::Vector(items) => {
            let values = items
                .iter()
                .map(|item| expr_to_dense_value(cx, item))
                .collect::<Result<Vec<_>>>()?;
            cx.factory().list(values)
        }
        Expr::Map(entries) => {
            let values = entries
                .iter()
                .map(|(key, value)| {
                    let Expr::Symbol(symbol) = key else {
                        return Err(Error::TypeMismatch {
                            expected: "symbol table key",
                            found: "non-symbol",
                        });
                    };
                    Ok((symbol.clone(), expr_to_dense_value(cx, value)?))
                })
                .collect::<Result<Vec<_>>>()?;
            cx.factory().table(values)
        }
        _ => cx.factory().expr(expr.clone()),
    }
}

#[cfg(any(feature = "table-db", feature = "table-fs", feature = "table-remote"))]
fn grant(cx: &mut Cx, capabilities: &[CapabilityName]) {
    for capability in capabilities {
        cx.grant(capability.clone());
    }
}

#[cfg(feature = "table-fs")]
fn temp_path(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("sim-say-{label}-{}-{nanos}", std::process::id()))
}
