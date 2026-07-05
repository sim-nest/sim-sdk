//! The batteries-included `sim` binary.
//!
//! `cargo install sim-nest --features serve-cli` installs a `sim` that serves both
//! MCP (`sim mcp --stdio`) and the web shell (`sim serve --addr HOST:PORT`) through the
//! same `sim_run_core::Bootloader` the wrapper binaries use -- no separate
//! `sim-mcp-server` / `sim-web-shell` install required. Server behavior stays in the
//! loaded libraries (`sim-lib-mcp`, `sim-web-shell`); this binary only composes their
//! host factories and normalizes user-facing verb aliases.
//!
//! This composition lives in the downstream facade on purpose: `sim-run` is the
//! upstream bootloader, and the serve libs depend on `sim-run-core`, so composing them
//! here (a forward edge) keeps the constellation free of a repo-boundary dependency
//! cycle (R7).

use std::ffi::OsString;
use std::process::ExitCode;

use sim_run_core::Bootloader;

fn main() -> ExitCode {
    let args = normalize_aliases(std::env::args_os().collect());
    let bootloader = sim_web_shell::configure_web_bootloader(
        sim_lib_mcp::configure_mcp_bootloader(Bootloader::standard()),
    );
    match bootloader.run(args) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("sim: {err}");
            ExitCode::from(2)
        }
    }
}

/// Canonical verbs are `mcp` and `serve`. Inject the boot codec each verb needs and
/// accept the front-page aliases `serve mcp` -> `mcp` and `webui` -> `serve`.
fn normalize_aliases(mut args: Vec<OsString>) -> Vec<OsString> {
    // Skip argv[0] (the program name); find the first payload token (the verb).
    let Some(i) = args
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, arg)| !arg.to_string_lossy().starts_with('-'))
        .map(|(i, _)| i)
    else {
        return args;
    };
    match args[i].to_string_lossy().as_ref() {
        "mcp" if !has_codec(&args) => splice_codec(&mut args, i, "mcp"),
        "serve" if args.get(i + 1).is_some_and(|v| v == "mcp") => {
            args.remove(i + 1);
            args[i] = OsString::from("mcp");
            if !has_codec(&args) {
                splice_codec(&mut args, i, "mcp");
            }
        }
        "webui" => {
            args[i] = OsString::from("serve");
            if !has_codec(&args) {
                splice_codec(&mut args, i, "lisp");
            }
        }
        "serve" if !has_codec(&args) => splice_codec(&mut args, i, "lisp"),
        _ => {}
    }
    args
}

fn splice_codec(args: &mut Vec<OsString>, at: usize, codec: &str) {
    args.splice(at..at, [OsString::from("--codec"), OsString::from(codec)]);
}

fn has_codec(args: &[OsString]) -> bool {
    args.iter().any(|arg| {
        let arg = arg.to_string_lossy();
        arg == "--codec" || arg.starts_with("--codec=")
    })
}
