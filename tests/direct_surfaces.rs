//! The batteries-included `sim` binary serves the wrapper surfaces directly.
//!
//! These prove the directive "run the wrapper surfaces with the base install, no
//! wrapper crate": `sim mcp` and `sim serve`/`sim webui` dispatch the same loaded
//! libraries (`sim-lib-mcp`, `sim-web-shell`) the standalone wrapper binaries use.
//! Gated on `serve-cli`, which is what builds the `sim` bin.
#![cfg(feature = "serve-cli")]

use std::io::Write;
use std::process::{Command, Stdio};

/// `sim mcp --stdio` answers a JSON-RPC `ping` without invoking `sim-mcp-server`.
#[test]
fn direct_mcp_answers_ping() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sim"))
        .args(["mcp", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sim mcp --stdio");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
            .unwrap();
        stdin.write_all(b"\n").unwrap();
    }
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("wait sim mcp");
    assert!(
        output.status.success(),
        "sim mcp --stdio failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#""id":1"#),
        "expected a JSON-RPC response for id 1, got: {stdout}"
    );
}

/// `sim webui --dry-run` (the `webui` alias -> `serve`) dispatches the web serve lib
/// and boots it without binding a socket or invoking `sim-web-shell`.
#[test]
fn direct_webui_dry_run_dispatches() {
    let output = Command::new(env!("CARGO_BIN_EXE_sim"))
        .args(["webui", "--addr", "127.0.0.1:0", "--dry-run"])
        .output()
        .expect("run sim webui --dry-run");
    assert!(
        output.status.success(),
        "sim webui --dry-run failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dry-run OK"),
        "expected the web serve dry-run marker, got: {stdout}"
    );
}

/// The `serve` verb also dry-runs (the canonical form the `webui` alias rewrites to).
#[test]
fn direct_serve_dry_run_dispatches() {
    let output = Command::new(env!("CARGO_BIN_EXE_sim"))
        .args(["serve", "--addr", "127.0.0.1:0", "--dry-run"])
        .output()
        .expect("run sim serve --dry-run");
    assert!(
        output.status.success(),
        "sim serve --dry-run failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("dry-run OK"));
}
