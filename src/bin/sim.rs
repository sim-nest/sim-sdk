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

/// The batteries commands, listed ahead of the bootloader's option help so a bare
/// `sim --help` shows what a newcomer can actually run. The core `--help` (below this)
/// documents the `--codec`/`--load`/`--eval` bootloader options.
const COMMANDS_HELP: &str = "\
The SIM runtime command line.

Commands:
  sim repl              Start a read-eval-print loop (Lisp).
  sim webui             Serve the browser Web UI.
  sim mcp               Serve an MCP server over stdio.
  sim <expression>      Evaluate a payload through the boot codec.

";

fn main() -> ExitCode {
    let args = normalize_aliases(std::env::args_os().collect());
    // A bare `sim --help` / `sim -h` (no verb) lists the batteries commands before the
    // bootloader prints its option help; a `sim <verb> --help` is left to the verb.
    if is_help_request(&args) {
        print!("{COMMANDS_HELP}");
    }
    // Report THIS binary's version. The bootloader core's own `--version` prints the
    // sim-run-core crate version, not the installed `sim-nest` facade's, so intercept
    // and short-circuit with the facade version the user actually installed.
    if is_version_request(&args) {
        println!("sim {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let bootloader = if verb_is(&args, "repl") {
        configure_repl_bootloader(Bootloader::standard())
    } else {
        configure_webui_bootloader(sim_lib_mcp::configure_mcp_bootloader(
            Bootloader::standard().with_context(install_web_minimum_loaded),
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

/// Installs the product minimum-loaded set for `sim webui`: an eager eval policy and
/// the core runtime. The Lisp codec is registered as the boot codec by
/// `configure_web_bootloader_with_cookbook`; every other cookbook lib stays visible
/// as a load recipe.
fn install_web_minimum_loaded(cx: &mut sim::kernel::Cx) {
    cx.set_eval_policy(Arc::new(sim::kernel::EagerPolicy));
    sim::runtime::install_core_runtime(cx);
}

/// Installs the statically-linked eval stack into the REPL boot `Cx`: an eager eval
/// policy, the core runtime, and the numbers prelude. The Lisp codec is not
/// installed here; `configure_repl_bootloader` registers it as a host factory.
fn install_eval_stack(cx: &mut sim::kernel::Cx) {
    cx.set_eval_policy(Arc::new(sim::kernel::EagerPolicy));
    sim::runtime::install_core_runtime(cx);
    sim::numbers_prelude::NumbersPreludeLib::new()
        .install_all(cx)
        .expect("numbers prelude installs");
}

/// Wires `sim repl` with a statically-linked eval stack so it works from a plain
/// `cargo install` -- no `SIM_REPL_BUNDLE_DIR` native dylibs. Installs the eval stack
/// and the Lisp codec into the boot `Cx`, then dispatches the `repl` verb to
/// `sim-lib-repl`'s `cli/main/repl` entrypoint. Mirrors `sim-sdk/examples/repl.rs`.
fn configure_repl_bootloader(loader: Bootloader) -> Bootloader {
    loader
        .with_context(install_eval_stack)
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

/// Wires `sim webui` with the product cookbook directory and effective
/// `sim/cookbook` overrides.
fn configure_webui_bootloader(loader: Bootloader) -> Bootloader {
    sim_web_shell::configure_web_bootloader_with_cookbook(
        loader,
        vec![sim_lib_cookbook::cookbook_lib_symbol()],
        Arc::new(cookbook_web_state),
    )
}

fn cookbook_web_state(
    config: &sim_run_core::RuntimeConfigState,
) -> sim_lib_server::CookbookWebState {
    let provider = sim_lib_cookbook::ConfigCookbookProvider::new_with_base(
        config.effective(),
        sim::runtime::cookbook_directory::default_cookbook_config(),
        &sim::runtime::cookbook_directory::SimNestCookbookResolver,
    );
    let (directory, mut diagnostics) = provider.loadable_libs();
    diagnostics.extend(config.diagnostics().iter().cloned());
    sim_lib_server::CookbookWebState::from_loadable_libs(directory, diagnostics)
}

/// True when the first non-flag payload token equals `want` (the requested verb).
fn verb_is(args: &[OsString], want: &str) -> bool {
    args.iter()
        .skip(1)
        .find(|arg| !arg.to_string_lossy().starts_with('-'))
        .is_some_and(|arg| arg == want)
}

/// True for a bare help request with no subcommand (`sim --help` / `sim -h`). A help
/// flag that follows a verb belongs to that verb, so it is not treated as bare here.
fn is_help_request(args: &[OsString]) -> bool {
    let has_verb = args
        .iter()
        .skip(1)
        .any(|arg| !arg.to_string_lossy().starts_with('-'));
    if has_verb {
        return false;
    }
    args.iter().skip(1).any(|arg| {
        let arg = arg.to_string_lossy();
        arg == "--help" || arg == "-h"
    })
}

/// True for a version request (`sim version` / `sim --version` / `sim -V`). A bare
/// `version` verb counts; a version flag trailing another verb belongs to that verb.
fn is_version_request(args: &[OsString]) -> bool {
    match args
        .iter()
        .skip(1)
        .find(|arg| !arg.to_string_lossy().starts_with('-'))
    {
        Some(verb) => verb == "version",
        None => args.iter().skip(1).any(|arg| {
            let arg = arg.to_string_lossy();
            arg == "--version" || arg == "-V"
        }),
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
