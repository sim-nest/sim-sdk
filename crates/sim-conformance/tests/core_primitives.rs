//! CORE host primitive conformance through the public `sim` facade.

#![allow(dead_code)]

#[path = "spec/support.rs"]
mod support;

use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

use sim::{
    kernel::{CapabilityName, Error, Expr, Symbol, Table},
    lib_exec::{ExecOptions, exec, exec_capability},
    table_fs::{
        FsDir, table_fs_edit_capability, table_fs_find_capability, table_fs_read_capability,
        table_fs_write_capability,
    },
    table_http::{HttpDir, HttpDirOptions, table_http_capability},
};

#[test]
fn fs_edit_semantics_atomicity_and_find_are_public_primitives() {
    let mut cx = support::cx();
    grant(
        &mut cx,
        [
            table_fs_read_capability(),
            table_fs_write_capability(),
            table_fs_edit_capability(),
            table_fs_find_capability(),
        ],
    );
    let root = temp_dir("fs-edit-find");
    let dir = FsDir::open(root.clone()).unwrap();
    let value = cx.factory().string("alpha beta\nomega\n".to_owned()).unwrap();

    dir.set(&mut cx, Symbol::new("note"), value).unwrap();
    dir.edit(&mut cx, Symbol::new("note"), "beta", "gamma", false)
        .unwrap();
    let edited = dir.get(&mut cx, Symbol::new("note")).unwrap();
    assert_eq!(
        edited.object().as_expr(&mut cx).unwrap(),
        Expr::String("alpha gamma\nomega\n".to_owned())
    );

    let raw_before_failed_edit = fs::read_to_string(root.join("note.siml")).unwrap();
    let err = dir
        .edit(&mut cx, Symbol::new("note"), "missing", "changed", false)
        .unwrap_err();
    assert!(err.to_string().contains("pattern not found"));
    assert_eq!(
        fs::read_to_string(root.join("note.siml")).unwrap(),
        raw_before_failed_edit
    );

    let matches = dir
        .find_grep(&mut cx, "gamma", Some("*.siml"), 10)
        .unwrap();
    assert_eq!(matches.matches.len(), 1);
    assert_eq!(matches.matches[0].path, "note.siml");
    assert_eq!(matches.matches[0].line, 1);

    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn find_refuses_paths_that_escape_the_directory_root() {
    let mut cx = support::cx();
    grant(
        &mut cx,
        [table_fs_read_capability(), table_fs_find_capability()],
    );
    let root = temp_dir("find-root");
    let outside = temp_file("find-outside", "siml");
    fs::write(&outside, "\"outside\"").unwrap();
    let dir = FsDir::open(root.clone()).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("escape.siml")).unwrap();

    let err = dir.find_glob(&mut cx, "*.siml", 10).unwrap_err();

    assert!(err.to_string().contains("escapes root"));
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_file(outside);
}

#[test]
fn exec_denial_and_output_bounds_are_public_primitives() {
    let mut cx = support::cx();
    let denied = exec(
        &mut cx,
        &argv(&["sim-conformance-missing-command"]),
        &ExecOptions::new(1_000, 1_024),
    )
    .unwrap_err();
    assert!(matches!(
        denied,
        Error::CapabilityDenied { capability } if capability == exec_capability()
    ));

    cx.grant(exec_capability());
    let result = exec(
        &mut cx,
        &argv(&["env", "printf", "1234567890"]),
        &ExecOptions::new(1_000, 4),
    )
    .unwrap();

    assert_eq!(result.stdout, "1234");
    assert_eq!(result.stderr, "");
    assert_eq!(result.exit_code, 0);
    assert!(result.truncated);
}

#[test]
fn http_dir_get_is_a_public_table_primitive() {
    let (base_url, handle) = serve_once(|request_line, stream| {
        assert_eq!(request_line, "GET /items/alpha HTTP/1.1");
        write_response(stream, "200 OK", br#""hello""#);
    });
    let mut cx = support::cx();
    cx.grant(table_http_capability());
    let dir = http_dir(base_url);

    let value = dir.get(&mut cx, Symbol::new("alpha")).unwrap();

    assert_eq!(
        value.object().as_expr(&mut cx).unwrap(),
        Expr::String("hello".to_owned())
    );
    handle.join().unwrap();
}

#[test]
fn legacy_fs_and_net_capability_aliases_still_work() {
    let mut cx = support::cx();
    grant(
        &mut cx,
        [
            CapabilityName::new("table.fs.read"),
            CapabilityName::new("table.fs.write"),
        ],
    );
    let root = temp_dir("legacy-fs-alias");
    let dir = FsDir::open(root.clone()).unwrap();
    let value = cx.factory().string("alias read".to_owned()).unwrap();
    dir.set(&mut cx, Symbol::new("note"), value).unwrap();
    assert_eq!(
        dir.get(&mut cx, Symbol::new("note"))
            .unwrap()
            .object()
            .as_expr(&mut cx)
            .unwrap(),
        Expr::String("alias read".to_owned())
    );

    let (base_url, handle) = serve_once(|request_line, stream| {
        assert_eq!(request_line, "GET /items/legacy HTTP/1.1");
        write_response(stream, "200 OK", br#""alias http""#);
    });
    cx.grant(CapabilityName::new("net.http"));
    let http = http_dir(base_url);
    assert_eq!(
        http.get(&mut cx, Symbol::new("legacy"))
            .unwrap()
            .object()
            .as_expr(&mut cx)
            .unwrap(),
        Expr::String("alias http".to_owned())
    );
    handle.join().unwrap();
    let _ = fs::remove_dir_all(root);
}

fn grant(cx: &mut sim::kernel::Cx, capabilities: impl IntoIterator<Item = CapabilityName>) {
    for capability in capabilities {
        cx.grant(capability);
    }
}

fn argv(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn http_dir(base_url: String) -> HttpDir {
    HttpDir::new(
        HttpDirOptions::new(base_url)
            .with_timeout_ms(1_000)
            .with_max_body_bytes(1_024),
    )
    .unwrap()
}

fn serve_once<F>(handler: F) -> (String, JoinHandle<()>)
where
    F: FnOnce(String, &mut TcpStream) + Send + 'static,
{
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request_line = read_request_line(&mut stream);
        handler(request_line, &mut stream);
    });
    (format!("http://127.0.0.1:{port}/items"), handle)
}

fn read_request_line(stream: &mut TcpStream) -> String {
    let mut head = Vec::new();
    let mut byte = [0];
    while !head.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        head.push(byte[0]);
    }
    let head = String::from_utf8(head).unwrap();
    head.lines().next().unwrap().to_owned()
}

fn write_response(stream: &mut TcpStream, status: &str, body: &[u8]) {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .unwrap();
    stream.write_all(body).unwrap();
    stream.flush().unwrap();
}

fn temp_dir(label: &str) -> PathBuf {
    let path = temp_path(label, "");
    fs::create_dir_all(&path).unwrap();
    path
}

fn temp_file(label: &str, extension: &str) -> PathBuf {
    temp_path(label, extension)
}

fn temp_path(label: &str, extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = std::env::temp_dir().join(format!(
        "sim-conformance-{label}-{}-{nanos}",
        std::process::id()
    ));
    if !extension.is_empty() {
        path.set_extension(extension);
    }
    path
}
