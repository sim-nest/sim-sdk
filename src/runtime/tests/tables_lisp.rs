#[cfg(any(feature = "table-hash", feature = "table-remote", feature = "table-fs"))]
use sim_codec::{Input, decode_with_codec};
#[cfg(all(feature = "codec-binary", feature = "table-remote"))]
use sim_codec_binary::BinaryCodecLib;
#[cfg(all(
    feature = "codec-lisp",
    any(feature = "table-hash", feature = "table-remote", feature = "table-fs")
))]
use sim_codec_lisp::LispCodecLib;
#[cfg(any(feature = "table-fs", feature = "table-remote"))]
use sim_kernel::CapabilityName;
use sim_kernel::Symbol;
#[cfg(feature = "table-remote")]
use sim_kernel::eval_remote_capability;
#[cfg(any(feature = "table-hash", feature = "table-remote", feature = "table-fs"))]
use sim_kernel::{Cx, Expr, ReadPolicy};
#[cfg(feature = "table-remote")]
use sim_lib_server::{Connection, EvalSite, LocalEvalSite, ServerAddress};
#[cfg(feature = "table-remote")]
use sim_table_db::install_db_dir_lib;
#[cfg(feature = "table-remote")]
use sim_table_remote::wrap_remote_table_site;
#[cfg(feature = "table-remote")]
use std::sync::Arc;

use super::support::eval_cx;
#[cfg(all(feature = "table-fs", feature = "codec-lisp"))]
use super::support::table_value;

#[cfg(feature = "table-fs")]
fn table_fs_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("table.fs")
}

#[cfg(feature = "table-fs")]
fn table_fs_read_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("fs/read")
}

#[cfg(feature = "table-fs")]
fn table_fs_write_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("fs/write")
}

#[cfg(feature = "table-fs")]
fn table_fs_edit_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("edit")
}

#[cfg(feature = "table-fs")]
fn table_fs_find_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("find")
}

#[cfg(any(feature = "table-db", feature = "table-remote"))]
fn table_db_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("table.db")
}

#[cfg(feature = "table-remote")]
fn table_db_read_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("table.db.read")
}

#[cfg(feature = "table-remote")]
fn table_db_write_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("table.db.write")
}

#[cfg(feature = "table-remote")]
fn table_db_mkdir_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("table.db.mkdir")
}

#[cfg(feature = "table-remote")]
fn table_db_rmdir_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("table.db.rmdir")
}

#[cfg(feature = "table-remote")]
fn table_remote_capability() -> sim_kernel::CapabilityName {
    sim_kernel::CapabilityName::new("table.remote")
}

#[cfg(all(
    feature = "codec-lisp",
    any(feature = "table-hash", feature = "table-remote", feature = "table-fs")
))]
fn install_lisp_codec(cx: &mut Cx) {
    let symbol = Symbol::qualified("codec", "lisp");
    if cx.registry().codec_by_symbol(&symbol).is_none() {
        let lib = LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
        cx.load_lib(&lib).unwrap();
    }
}

#[cfg(all(feature = "codec-binary", feature = "table-remote"))]
fn install_binary_codec(cx: &mut Cx) {
    let symbol = Symbol::qualified("codec", "binary");
    if cx.registry().codec_by_symbol(&symbol).is_none() {
        let lib = BinaryCodecLib::new(cx.registry_mut().fresh_codec_id());
        cx.load_lib(&lib).unwrap();
    }
}

#[cfg(feature = "codec-lisp")]
#[cfg(any(feature = "table-hash", feature = "table-remote", feature = "table-fs"))]
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

#[cfg(feature = "codec-lisp")]
#[cfg(any(feature = "table-hash", feature = "table-remote", feature = "table-fs"))]
fn eval_lisp(cx: &mut Cx, text: &str) -> sim_kernel::Value {
    install_lisp_codec(cx);
    let expr = decode_with_codec(
        cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text(text.to_owned()),
        ReadPolicy::default(),
    )
    .unwrap();
    cx.eval_expr(lower_lisp_eval_surface(expr)).unwrap()
}

#[cfg(any(feature = "table-hash", feature = "table-remote"))]
fn number_text(expr: Expr) -> String {
    match expr {
        Expr::Number(number) => number.canonical,
        other => panic!("expected number expression, found {other:?}"),
    }
}

#[cfg(any(feature = "table-fs", feature = "table-remote"))]
fn grant(cx: &mut Cx, capabilities: &[CapabilityName]) {
    for capability in capabilities {
        cx.grant(capability.clone());
    }
}

#[cfg(all(feature = "table-fs", feature = "codec-lisp"))]
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[test]
#[cfg(all(feature = "table-hash", feature = "codec-lisp"))]
fn lisp_table_ops_roundtrip() {
    let mut cx = eval_cx();
    let table = eval_lisp(&mut cx, "(table/hash 'a 1)");
    cx.env_mut().define(Symbol::new("t"), table);

    eval_lisp(&mut cx, "(set t 'b 2)");
    let value = eval_lisp(&mut cx, "(get t 'b)");
    assert_eq!(number_text(value.object().as_expr(&mut cx).unwrap()), "2");

    let len = eval_lisp(&mut cx, "(len t)");
    assert_eq!(number_text(len.object().as_expr(&mut cx).unwrap()), "2");

    let entries = eval_lisp(&mut cx, "(entries t)");
    let list = entries.object().as_list().unwrap();
    assert_eq!(list.to_vec(&mut cx, None).unwrap().len(), 2);
}

#[test]
#[cfg(feature = "table-db")]
fn table_db_constructor_requires_capability() {
    let mut cx = eval_cx();
    let err = cx
        .call_function(
            &Symbol::qualified("table", "db"),
            sim_kernel::Args::new(Vec::new()),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        sim_kernel::Error::CapabilityDenied { capability }
            if capability == table_db_capability()
    ));
}

#[test]
#[cfg(feature = "table-fs")]
fn table_fs_constructor_builds_directory_backend() {
    let mut cx = eval_cx();
    let path = std::env::temp_dir().join(format!("sim-table-fs-{}", std::process::id()));
    let path_value = cx.factory().string(path.display().to_string()).unwrap();
    let err = cx
        .call_function(
            &Symbol::qualified("table", "fs"),
            sim_kernel::Args::new(vec![path_value.clone()]),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        sim_kernel::Error::CapabilityDenied { capability }
            if capability == table_fs_capability()
    ));

    cx.grant(table_fs_capability());
    let dir = cx
        .call_function(
            &Symbol::qualified("table", "fs"),
            sim_kernel::Args::new(vec![path_value]),
        )
        .unwrap();
    assert!(dir.object().as_dir().is_some());
    let _ = std::fs::remove_dir_all(path);
}

#[test]
#[cfg(all(feature = "table-fs", feature = "codec-lisp"))]
fn lisp_dir_edit_patches_filesystem_leaf() {
    let mut cx = eval_cx();
    grant(
        &mut cx,
        &[
            table_fs_capability(),
            table_fs_read_capability(),
            table_fs_write_capability(),
            table_fs_edit_capability(),
        ],
    );
    let path = std::env::temp_dir().join(format!(
        "sim-sdk-table-edit-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let source = format!(
        r#"{{
  (set (table/fs "{}") 'note "alpha beta")
  (dir/edit (table/fs "{}") 'note "beta" "gamma")
  (get (table/fs "{}") 'note)
}}"#,
        path.display(),
        path.display(),
        path.display()
    );

    let value = eval_lisp(&mut cx, &source);

    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::String("alpha gamma".to_owned())
    );
    let _ = std::fs::remove_dir_all(path);
}

#[test]
#[cfg(all(feature = "table-fs", feature = "codec-lisp"))]
fn lisp_find_grep_returns_matches_as_data() {
    let mut cx = eval_cx();
    grant(
        &mut cx,
        &[
            table_fs_capability(),
            table_fs_read_capability(),
            table_fs_write_capability(),
            table_fs_find_capability(),
        ],
    );
    let path = std::env::temp_dir().join(format!(
        "sim-sdk-table-find-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let source = format!(
        r#"{{
  (set (table/fs "{}") 'alpha "one needle")
  (set (table/fs "{}") 'beta "plain hay")
  (find/grep (table/fs "{}") "needle" "*.siml" 10)
}}"#,
        path.display(),
        path.display(),
        path.display()
    );

    let value = eval_lisp(&mut cx, &source);
    let expr = value.object().as_expr(&mut cx).unwrap();
    let matches = table_value(&expr, &Symbol::new("matches")).expect("matches");
    let Expr::List(items) = matches else {
        panic!("matches should be a list");
    };

    assert_eq!(items.len(), 1);
    assert_eq!(
        table_value(&items[0], &Symbol::new("path")),
        Some(&Expr::String("alpha.siml".to_owned()))
    );
    assert_eq!(
        table_value(&items[0], &Symbol::new("text")),
        Some(&Expr::String("one needle".to_owned()))
    );
    assert_eq!(
        table_value(&expr, &Symbol::new("truncated")),
        Some(&Expr::Bool(false))
    );
    let _ = std::fs::remove_dir_all(path);
}

#[test]
#[cfg(feature = "table-remote")]
fn table_remote_constructor_uses_connection_site() {
    let mut cx = eval_cx();
    install_binary_codec(&mut cx);
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
    let inner: Arc<dyn EvalSite> = Arc::new(LocalEvalSite::new(
        ServerAddress::Local,
        vec![Symbol::qualified("codec", "binary")],
    ));
    let wrapped = wrap_remote_table_site(inner, root);
    let connection = Connection::new(
        ServerAddress::Local,
        Symbol::qualified("codec", "binary"),
        vec![Symbol::qualified("codec", "binary")],
        wrapped,
    )
    .unwrap();
    let connection_value = cx.factory().opaque(Arc::new(connection)).unwrap();

    let remote = cx
        .call_function(
            &Symbol::qualified("table", "remote"),
            sim_kernel::Args::new(vec![connection_value]),
        )
        .unwrap();
    cx.env_mut().define(Symbol::new("remote"), remote);

    eval_lisp(&mut cx, "(set remote 'a 7)");
    let value = eval_lisp(&mut cx, "(get remote 'a)");
    assert_eq!(number_text(value.object().as_expr(&mut cx).unwrap()), "7");
}
