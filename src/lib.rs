//! # sim-nest -- the SIM umbrella crate (imported as `sim`)
//! Published on crates.io as **`sim-nest`** (the bare name `sim` is taken), while
//! its library import identifier is `sim`. Depend on `sim-nest` and write
//! `use sim::...`; `use sim_nest::...` does not resolve. The `#[sim::sim_lib]`
//! and `#[sim::sim_fn]` macros use the same stable import identifier.
//!
//! SIM is an expandable Rust runtime built around a small protocol kernel and
//! loadable libraries. Its data flow is:
//!
//! ```text
//! tokens -> checked forms -> objects -> checked calls -> objects -> encoded forms
//! ```
//!
//! Lisp is one codec, not the system identity. Syntax, codecs, classes,
//! functions, number domains, checkers, evaluators, wasm adapters, loaders, and
//! the standard language surface are libraries loaded above the kernel.
//!
//! ## Umbrella role
//!
//! This crate aggregates the constellation's implementation crates through an
//! optional-dependency feature map and re-exports stable aliases including
//! `sim::kernel`, `sim::shape`, `sim::codec`, and the `codec_*`, `lib_*`,
//! `table_*`, and `list_*` families. The opt-in `expr-tree` feature exposes the
//! canonical expression-tree crates without adding facade policy. Authoring
//! helpers (`functions`, `classes`, `macros`, `shapes`, and `runtime`) are
//! available with `shape`. The canonical feature map is in `Cargo.toml`.
//!
//! ## Kernel boundary
//!
//! The kernel owns identity and transport types, coordination types such as
//! `Cx`, `Registry`, `Lib`, `Linker`, and `ExportRecord`, capabilities, stores,
//! ledgers, control policy, object/callable/class/shape/factory/evaluation
//! contracts, match results, and ABI transport shapes. It does not own concrete
//! language or codec parsing, number domains, arithmetic, user-facing help and
//! browse behavior, guest behavior above ABI transport, or product policy.
//! Extensible metadata remains open `ExportRecord`-style data; concrete
//! behavior is installed through `Lib`, `Linker`, and `ExportRecord`.
//!
//! ## Load-bearing concepts
//!
//! - **`Shape`** is the shared engine for parsing, checking, binding, dispatch,
//!   macro syntax, codec grammar, lambda locals, and overload selection.
//! - **Codecs are first-class runtime objects**, split into decoders and
//!   position-aware encoders. General codecs round-trip every shared `Expr`;
//!   domain codecs fail closed outside the domain they accept.
//! - **`realize` and `EvalFabric`** provide location-transparent distributed
//!   evaluation. Evaluation strategy is an injectable `EvalPolicy`.
//! - **Capabilities make power explicit.** Read-construct is the narrow path
//!   behind Lisp `#(...)`; it is distinct from broad read-eval, which remains
//!   disabled by default for untrusted input.
//! - **Number domains, lists, and tables are pluggable libraries.**
//! - **Wasm is a first-class runtime target and portable plugin ABI.**
//!
//! ## Embedding
//!
//! `runtime::install_core_runtime` is the embedding entry point when `shape` is
//! enabled. Build a `Cx`, install the core runtime, then install codecs and
//! behavior libraries through their helpers or through `Lib` and `Linker`:
//!
//! ```ignore
//! use std::sync::Arc;
//! use sim::kernel::{Cx, DefaultFactory, EagerPolicy};
//! use sim::runtime::install_core_runtime;
//!
//! let mut cx = Cx::new(
//!     Arc::new(EagerPolicy),
//!     Arc::new(DefaultFactory),
//!     sim::kernel::HandleSeed::new(0xeea6_94db_eb56_c485),
//! );
//! install_core_runtime(&mut cx);
//! ```
//!
//! The installer loads the registry-backed core and enabled default number
//! domains; applications then add only the libraries their distribution needs.
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![allow(deprecated)]
extern crate self as sim;

/// Public, data-oriented hot-generation contracts.
///
/// Host provider implementations remain outside the facade: applications
/// compose storage, sandbox, loader, and journal ports and pass only these
/// stable requests, candidates, compatibility reports, and receipts across
/// their public boundary.
#[cfg(feature = "hotload")]
pub mod hotload;

/// Portable estate vocabulary, exposure compiler, organ, projection, and calls.
/// Concrete controller bindings are intentionally absent; providers are opt-in.
#[cfg(feature = "estate")]
pub mod estate;
#[cfg(all(test, feature = "estate"))]
mod estate_facade_tests;

#[cfg(feature = "platform")]
/// Provider-neutral platform records and authoring contracts.
pub mod platform;

/// Direct, provider-replaceable relational assembly surface.
///
/// Raw logical plans are admitted by [`plan`] before reaching [`site`]. The
/// checked plan and migration types remain opaque, and provider preparation
/// artifacts are deliberately not re-exported. Enable `relation-sqlite` only
/// when the SQLite capsule is part of the application.
#[cfg(feature = "relation")]
pub mod relation;

#[rustfmt::skip]
#[cfg(any(feature = "femm-assembly", feature = "femm-codec", feature = "femm-core", feature = "femm-fixtures", feature = "femm-field", feature = "femm-flow", feature = "femm-function", feature = "femm-geometry", feature = "femm-material", feature = "femm-mesh", feature = "femm-ode", feature = "femm-physics", feature = "femm-post", feature = "femm-prelude", feature = "femm-sensitiv", feature = "femm-solve", feature = "femm-space", feature = "femm-tape"))]
pub use femm_exports::*;
#[rustfmt::skip] #[allow(unused_imports)] pub use numbers_exports::*;
#[rustfmt::skip] #[allow(unused_imports)] pub use standard_exports::*;
#[rustfmt::skip]
#[cfg(any(feature = "server-net-http", feature = "agent-net", feature = "glasses", feature = "openai-server-http", feature = "standard", feature = "rank-codec-fallback", feature = "rank-expr", feature = "rank-learn", feature = "rank-music", feature = "rank-scatter", feature = "stream-bridge", feature = "stream-host"))]
const _: bool = true;
#[allow(unused_imports)]
pub use roadmap11_exports::*;
#[rustfmt::skip]
#[cfg(any(feature = "compute-auto", feature = "compute-cli", feature = "compute-cuda", feature = "compute-femm", feature = "compute-model", feature = "compute-rocm", feature = "compute-wgpu"))]
pub use compute_exports::*;
#[rustfmt::skip]
#[cfg(any(feature = "interference-core", feature = "interference-solve", feature = "interference-runtime", feature = "interference-compute", feature = "view-interference"))]
pub use interference_exports::*;
#[cfg(feature = "expr-tree")]
pub use expr_tree_exports::*;
#[cfg(feature = "agent")]
pub use sim_lib_agent::{self as lib_agent, install_agent_lib};
/// Certified topology-backed agent conducts and the shipped conduct catalog.
#[cfg(feature = "agent-conduct")]
pub mod agent_conduct;
/// Pure, codec-stable agent conduct records and durable lifecycle contracts.
#[cfg(feature = "agent-conduct-core")]
pub mod agent_conduct_core;
#[cfg(feature = "topology-core")]
pub use sim_lib_topology as lib_topology;
/// Stable opt-in provider control facade. This is intentionally excluded from
/// the minimal/default SDK feature set.
#[cfg(feature = "provider")]
pub mod provider;
/// Domain-neutral durable study lifecycle, design, decision, and command product.
#[cfg(feature = "study")]
pub mod study;

#[cfg(all(test, feature = "study"))]
mod study_facade_tests;

/// Independently selectable physics contracts and loadable composition.
///
/// Each module is a direct re-export from its owning crate. Enabling
/// `physics-core` does not select proof methods, extended precision, FEMM,
/// interference, or placement providers.
#[cfg(feature = "physics-core")]
pub mod physics;

/// Canonical, pure continuity planning, replay, and journal contracts.
///
/// This module is a direct re-export: policy validation and reduction remain
/// owned by `sim-lib-continuity`; the SDK adds no wrapper model or behavior.
#[cfg(feature = "continuity")]
pub mod continuity;

/// Resilient music routing composed from the owning music and stream-host contracts.
#[cfg(feature = "music-vertical")]
pub mod music_vertical;

#[cfg(feature = "standard-core")]
pub mod characterization;
/// Native class authoring helpers: a `Class` implementation plus the lib
/// wrapper that registers a host-defined class, its constructor, and members.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod classes;
/// Model observatory types and its loadable product command.
#[cfg(feature = "model-test")]
pub mod model_test;
#[cfg(all(test, feature = "model-test"))]
mod model_test_facade_tests;
/// Provider-neutral web research composition.
///
/// This facade intentionally exposes stable plans, receipts, records, and host
/// boundaries. Provider-specific wire DTOs remain in their codec crates.
#[cfg(feature = "web-search")]
pub mod web_search;
#[rustfmt::skip]
#[cfg(all(test, feature = "shape", feature = "codec-lisp", feature = "codec-json", feature = "codec-binary", feature = "codec-binary-base64", feature = "codec-algol", feature = "codec-bridge", feature = "bridge"))]
mod codec_matrix_tests;
/// Stable hashing of lib manifests, shapes, and codecs for compatibility
/// checks across versions of the constellation.
#[cfg(feature = "core")]
pub mod compat;
mod compute_exports;
#[cfg(feature = "control")]
pub mod exceptions;
#[cfg(feature = "expr-tree")]
mod expr_tree_exports;
mod femm_exports;
/// Function authoring helpers built on the shared `Shape` engine: overload
/// cases, native function objects, and member-table construction.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod functions;
/// Managed-object collector selection for standard and minimal/test distributions.
#[cfg(feature = "standard-mutation")]
pub mod gc;
mod interference_exports;
/// Lib loaders for the supported source formats (host, Lisp source, binary
/// pack, native dynamic library, and wasm) plus the standard loader registry.
#[cfg(feature = "core")]
pub mod loaders;
/// Macro authoring and expansion: the `LispMacro` contract, macro objects, the
/// registry-backed expander, and shape constructors for macro syntax.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod macros;
mod music_algorithm_exports;
/// End-to-end music rendering stack that lowers a score to MIDI and renders it
/// to PCM audio through the sound libs.
#[cfg(feature = "sound-music")]
pub mod music_stack;
mod numbers_exports;
#[allow(unused_imports)]
pub use music_algorithm_exports::*;
mod roadmap11_exports;
/// Core runtime installer and the embedding entry point that wires classes,
/// shapes, functions, and the default number domains into a `Cx`.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod runtime;
#[cfg(feature = "serial-music")]
pub mod serial_music;
/// Shape authoring helpers: documented and value-backed shape wrappers plus
/// shape registration and checking utilities.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod shapes;
#[cfg(feature = "core")]
pub mod source_authority;
mod standard_exports;
#[cfg(feature = "proc-macros")]
pub use sim_macros::*;
// The macros' native_export output emits `::sim::codec_binary::{decode_frame,
// encode_frame}`, so the feature that enables the macros must also expose that
// module. `proc-macros` pulls `codec-binary`; this contract asserts it, so an
// edit that drops it fails to compile instead of shipping macros that cannot expand.
#[cfg(all(feature = "proc-macros", not(feature = "codec-binary")))]
compile_error!("feature `proc-macros` requires `codec-binary` (macros emit `::sim::codec_binary`)");
#[cfg(feature = "wasm")]
pub use sim_wasm_abi as wasm_abi;
#[cfg(test)]
mod feature_contract_tests;
#[cfg(all(test, feature = "music-stack"))]
mod music_stack_tests;
#[cfg(all(test, feature = "skill"))]
mod skill_tests;
