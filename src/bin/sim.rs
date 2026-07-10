//! The batteries-included `sim` binary.
//!
//! `cargo install sim-nest --features serve-cli` installs a `sim` that serves both
//! MCP (`sim mcp --stdio`) and the web shell (`sim serve --addr HOST:PORT`) AND runs an
//! interactive or piped `sim repl`, all through the same `sim_run_core::Bootloader` the
//! wrapper binaries use -- no separate `sim-mcp-server` / `sim-web-shell` install and no
//! on-disk native dylib bundle required. Server and repl behavior stay in the loaded
//! libraries (`sim-lib-mcp`, `sim-web-shell`, `sim-lib-repl`); this binary only composes
//! their host factories, installs the repl's static eval stack, and normalizes
//! user-facing verb aliases.
//!
//! This composition lives in the downstream facade on purpose: `sim-run` is the
//! upstream bootloader, and the serve libs depend on `sim-run-core`, so composing them
//! here (a forward edge) keeps the constellation free of a repo-boundary dependency
//! cycle (R7).

use std::ffi::OsString;
use std::process::ExitCode;
use std::sync::Arc;

use sim_run_core::Bootloader;

fn main() -> ExitCode {
    let args = normalize_aliases(std::env::args_os().collect());
    // `sim repl` needs an eval stack; the serve verbs must not pay for it (and its
    // eager lisp codec would collide with the web boot codec). Compose one or the
    // other by the requested verb, exactly as the native `sim-run` bootloader branches.
    let bootloader = if verb_is(&args, "repl") {
        configure_repl_bootloader(Bootloader::standard())
    } else {
        sim_web_shell::configure_web_bootloader(sim_lib_mcp::configure_mcp_bootloader(
            Bootloader::standard(),
        ))
    };
    match bootloader.run(args) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(code as u8),
        Err(err) => {
            eprintln!("sim: {err}");
            ExitCode::from(2)
        }
    }
}

/// Wires `sim repl` with a statically-linked eval stack so it works from a plain
/// `cargo install` -- no `SIM_REPL_BUNDLE_DIR` native dylibs. Installs an eager eval
/// policy, the core runtime, the numbers prelude, and the Lisp codec into the boot
/// `Cx`, then dispatches the `repl` verb to `sim-lib-repl`'s `cli/main/repl`
/// entrypoint. Mirrors `sim-sdk/examples/repl.rs`, the proven in-process recipe.
fn configure_repl_bootloader(loader: Bootloader) -> Bootloader {
    loader
        // The eval stack goes into the boot `Cx` directly: an eager policy (the
        // bootloader's default is noop), the core runtime, and the numbers prelude.
        .with_context(|cx| {
            cx.set_eval_policy(Arc::new(sim::kernel::EagerPolicy));
            sim::runtime::install_core_runtime(cx);
            sim::numbers_prelude::NumbersPreludeLib::new()
                .install_all(cx)
                .expect("repl numbers prelude installs");
        })
        // The Lisp codec is registered as a host factory so the boot-codec resolver
        // finds `codec/lisp` (it falls back to a host source when the crates.io
        // resolver has nothing), mirroring `configure_web_bootloader`.
        .host_lib("codec/lisp", || {
            Box::new(
                sim::codec_lisp::LispCodecLib::new(sim::kernel::CodecId(1))
                    .expect("repl lisp boot codec builds"),
            )
        })
        .host_verb(
            "repl",
            "lib/repl",
            || Box::new(sim_lib_repl::ReplLib::new()),
        )
}

/// True when the first non-flag payload token equals `want` (the requested verb).
fn verb_is(args: &[OsString], want: &str) -> bool {
    args.iter()
        .skip(1)
        .find(|arg| !arg.to_string_lossy().starts_with('-'))
        .is_some_and(|arg| arg == want)
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
