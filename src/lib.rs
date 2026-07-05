//! # sim-nest -- the SIM umbrella crate (imported as `sim`)
//!
//! Published on crates.io as **`sim-nest`** (the bare name `sim` is taken), but the
//! library import identifier is `sim`. Add it as `sim-nest = "0.1"` (or, to make the
//! rename explicit, `sim = { package = "sim-nest", version = "0.1" }`) and write
//! `use sim::...` throughout; the `#[sim::sim_lib]` / `#[sim::sim_fn]` proc-macros
//! resolve against it unchanged. Note: `use sim_nest::...` will NOT resolve -- the
//! crate's library name is `sim`, so import `sim`, not `sim_nest`.
//!
//! SIM is an expandable Rust runtime built around a small protocol kernel plus
//! a large set of loadable libraries. The kernel defines contracts; libraries
//! provide behavior. The data flow is:
//!
//! ```text
//! tokens -> checked forms -> objects -> checked calls -> objects -> encoded forms
//! ```
//!
//! SIM is a Rust runtime with multiple codec surfaces. Lisp is one codec, not
//! the system identity. Everything above the kernel is a lib: syntax, codecs,
//! classes, functions, number domains, checkers, evaluators, wasm adapters,
//! loaders, and even the standard language surface. The standard distribution
//! is just a set of libs loaded by default.
//!
//! ## Umbrella role
//!
//! This crate (`sim`) is the umbrella and entry point of the SIM constellation.
//! The implementation crates live in sibling repositories; this crate
//! aggregates them through optional dependencies and a feature map, re-exports
//! them under stable module aliases (`sim::kernel`, `sim::shape`,
//! `sim::codec`, the `sim::codec_*`, `sim::lib_*`, `sim::table_*`, and
//! `sim::list_*` families), and ships the core runtime installer plus the
//! authoring helpers (`functions`, `classes`, `macros`, `shapes`, and
//! `runtime`, available with the `shape` feature). The default feature set is
//! `core`, `codec-lisp`, and
//! `numbers-f64`; the canonical, current feature map is this crate's
//! `Cargo.toml`.
//!
//! ## Kernel boundary
//!
//! The central discipline is keeping the kernel small. The kernel may define
//! identity and transport types (`Symbol`, `Expr`, `Value`, `Origin`, `Ref`,
//! `Datum`, errors, stable ids), coordination types (`Cx`, `Registry`, `Lib`,
//! `Linker`, `ExportRecord`, capabilities, claim/fact and handle stores, Card
//! records, operation specs, event/effect ledgers, control policy, rank
//! metadata), the object/callable/class/shape/factory/eval-policy/
//! macro-expander behavior contracts, shape match and binding result types, and
//! the ABI frame and manifest transport shapes. The kernel must not define
//! concrete Lisp/JSON/Algol parsing, concrete number domains or arithmetic,
//! concrete help/test/browse implementations, wasm guest behavior above the ABI
//! transport, or remote transport and agent-product policy. New metadata is
//! modeled as open `ExportRecord`-style data rather than new closed kernel
//! enums. Concrete behavior is added as a lib through `Lib`, `Linker`, and
//! `ExportRecord`.
//!
//! ## Load-bearing concepts
//!
//! - **`Shape`** is one shared engine for parsing, checking, binding, dispatch,
//!   macro syntax, codec grammar, lambda locals, and overload selection. It is
//!   a first-class kernel protocol (object-accessible via `as_shape`, callable
//!   as a matcher); concrete shape behavior lives in `sim-shape` and other libs.
//! - **Codecs are first-class runtime objects**, split into independent
//!   decoders and encoders; encoders know their output position. General-purpose
//!   expression codecs are total over the shared `Expr` graph and round-trip
//!   every expression semantically; domain codecs round-trip only their domain
//!   and fail closed outside it.
//! - **`realize` and `EvalFabric`** are the location-transparent distributed
//!   evaluation surface. Server and agent code targets these, never a
//!   transport-specific API. Evaluation strategy itself is an injectable
//!   `EvalPolicy` (eager, lazy, need, hybrid, no-op).
//! - **Capability gating** makes power explicit: read-eval, native dynamic
//!   loading, and host effects (file, network, clock, random, process) are
//!   capabilities a host grants. **Read-construct** is the narrower
//!   capability-gated path that backs Lisp `#(...)` literals; it is distinct
//!   from broad **read-eval**, which evaluates during decode and is disabled by
//!   default for untrusted input.
//! - **Number domains, lists, and tables are pluggable libs**, not kernel
//!   behavior; codecs delegate numeric literals to the active domains by parse
//!   priority.
//! - **Wasm** is a first-class runtime target and the portable plugin ABI.
//!
//! ## Embedding
//!
//! `runtime::install_core_runtime` (with the `shape` feature) is the entry
//! point for embedding SIM.
//! Build a `Cx` with an eval policy and a factory, install the core runtime,
//! then install codecs and behavior libs through their `install_*` helpers or
//! directly through `Lib` and `Linker`:
//!
//! ```ignore
//! use std::sync::Arc;
//! use sim::kernel::{Cx, DefaultFactory, EagerPolicy};
//! use sim::runtime::install_core_runtime;
//!
//! let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
//! install_core_runtime(&mut cx);
//! // install codecs and libs, then cx.eval_expr(...).
//! ```
//!
//! `install_core_runtime` loads the core runtime through the lib registry and
//! installs the default number domain(s) for the enabled `numbers-*` features.
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![allow(deprecated)]
extern crate self as sim;

#[rustfmt::skip]
#[cfg(any(feature = "femm-assembly", feature = "femm-codec", feature = "femm-core", feature = "femm-fixtures", feature = "femm-field", feature = "femm-flow", feature = "femm-function", feature = "femm-geometry", feature = "femm-material", feature = "femm-mesh", feature = "femm-ode", feature = "femm-physics", feature = "femm-post", feature = "femm-prelude", feature = "femm-sensitiv", feature = "femm-solve", feature = "femm-space", feature = "femm-tape"))]
pub use femm_exports::*;
#[rustfmt::skip] #[allow(unused_imports)] pub use numbers_exports::*;
#[rustfmt::skip]
#[cfg(any(feature = "codec-lisp", feature = "codec-json", feature = "codec-binary", feature = "codec-binary-base64", feature = "codec-bitwise", feature = "codec-bitwise-base64", feature = "codec-chat", feature = "codec-mcp", feature = "codec-algol"))]
pub use sim_codec as codec;
#[cfg(feature = "codec-algol")]
pub use sim_codec_algol as codec_algol;
#[cfg(feature = "codec-binary")]
pub use sim_codec_binary as codec_binary;
#[cfg(feature = "codec-binary-base64")]
pub use sim_codec_binary_base64 as codec_binary_base64;
#[cfg(feature = "codec-bitwise")]
pub use sim_codec_bitwise as codec_bitwise;
#[cfg(feature = "codec-bitwise-base64")]
pub use sim_codec_bitwise_base64 as codec_bitwise_base64;
#[cfg(feature = "codec-chat")]
pub use sim_codec_chat as codec_chat;
#[cfg(feature = "codec-json")]
pub use sim_codec_json as codec_json;
#[cfg(feature = "codec-lisp")]
pub use sim_codec_lisp as codec_lisp;
#[cfg(feature = "codec-mcp")]
pub use sim_codec_mcp as codec_mcp;
#[cfg(feature = "core")]
pub use sim_kernel as kernel;
#[cfg(feature = "standard-binding")]
pub use sim_lib_binding as lib_binding;
#[cfg(feature = "control")]
pub use sim_lib_control as lib_control;
#[cfg(feature = "core")]
pub use sim_lib_core as lib_core;
#[cfg(feature = "standard-dispatch")]
pub use sim_lib_dispatch as lib_dispatch;
#[cfg(feature = "standard-cl")]
pub use sim_lib_lang_cl as lib_lang_cl;
#[cfg(feature = "standard-clojure")]
pub use sim_lib_lang_clojure as lib_lang_clojure;
#[cfg(feature = "standard-islisp")]
pub use sim_lib_lang_islisp as lib_lang_islisp;
#[cfg(feature = "standard-julia")]
pub use sim_lib_lang_julia as lib_lang_julia;
#[cfg(feature = "standard-lua")]
pub use sim_lib_lang_lua as lib_lang_lua;
#[cfg(feature = "standard-ruby")]
pub use sim_lib_lang_ruby as lib_lang_ruby;
#[cfg(feature = "standard-scheme")]
pub use sim_lib_lang_scheme as lib_lang_scheme;
#[cfg(feature = "standard-typed-lazy")]
pub use sim_lib_lang_typed_lazy as lib_lang_typed_lazy;
#[cfg(feature = "logic-core")]
pub use sim_lib_logic as lib_logic;
#[cfg(feature = "mcp")]
pub use sim_lib_mcp::{self as lib_mcp, install_mcp_lib};
#[cfg(feature = "standard-mutation")]
pub use sim_lib_mutation as lib_mutation;
#[cfg(feature = "standard-namespace")]
pub use sim_lib_namespace as lib_namespace;
#[cfg(feature = "openai-server")]
pub use sim_lib_openai_server as lib_openai_server;
#[cfg(feature = "standard-pattern")]
pub use sim_lib_pattern as lib_pattern;
#[cfg(feature = "rank")]
pub use sim_lib_rank as lib_rank;
#[cfg(feature = "standard-sequence")]
pub use sim_lib_sequence as lib_sequence;
#[cfg(feature = "server")]
pub use sim_lib_server::{self as lib_server, install_server_lib};
#[cfg(feature = "skill")]
pub use sim_lib_skill::{self as lib_skill, install_skill_lib};
#[cfg(feature = "standard-core")]
pub use sim_lib_standard_core as lib_standard_core;
#[cfg(feature = "stream-audio")]
pub use sim_lib_stream_audio as lib_stream_audio;
#[cfg(feature = "stream-bridge")]
pub use sim_lib_stream_bridge as lib_stream_bridge;
#[cfg(feature = "stream-clock")]
pub use sim_lib_stream_clock as lib_stream_clock;
#[cfg(feature = "stream-combinators")]
pub use sim_lib_stream_combinators as lib_stream_combinators;
#[cfg(feature = "stream-core")]
pub use sim_lib_stream_core as lib_stream_core;
#[cfg(feature = "stream-fabric")]
pub use sim_lib_stream_fabric as lib_stream_fabric;
#[cfg(feature = "stream-file")]
pub use sim_lib_stream_file as lib_stream_file;
#[cfg(feature = "stream-host")]
pub use sim_lib_stream_host as lib_stream_host;
#[cfg(feature = "stream-midi")]
pub use sim_lib_stream_midi as lib_stream_midi;
#[cfg(feature = "stream-prelude")]
pub use sim_lib_stream_prelude as lib_stream_prelude;
#[cfg(feature = "web-bridge")]
pub use sim_lib_web_bridge as lib_web_bridge;
#[cfg(feature = "list-cell")]
pub use sim_list_cell as list_cell;
#[cfg(feature = "list-lazy")]
pub use sim_list_lazy as list_lazy;
#[cfg(feature = "shape")]
pub use sim_shape as shape;
#[cfg(feature = "table-db")]
pub use sim_table_db as table_db;
#[cfg(feature = "table-fs")]
pub use sim_table_fs as table_fs;
#[cfg(feature = "table-hash")]
pub use sim_table_hash as table_hash;
#[cfg(feature = "table-lazy")]
pub use sim_table_lazy as table_lazy;
#[cfg(feature = "table-override")]
pub use sim_table_override as table_override;
#[cfg(feature = "table-remote")]
pub use sim_table_remote as table_remote;
#[rustfmt::skip]
#[cfg(any(feature = "server-net-http", feature = "agent-net", feature = "openai-server-http", feature = "standard", feature = "rank-codec-fallback", feature = "rank-expr", feature = "rank-learn", feature = "rank-music", feature = "rank-scatter", feature = "stream-bridge", feature = "stream-host"))]
const _: bool = true;
#[allow(unused_imports)]
pub use roadmap11_exports::*;
#[cfg(feature = "agent")]
pub use sim_lib_agent::{self as lib_agent, install_agent_lib};
/// Native class authoring helpers: a `Class` implementation plus the lib
/// wrapper that registers a host-defined class, its constructor, and members.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod classes;
#[rustfmt::skip]
#[cfg(all(test, feature = "shape", feature = "codec-lisp", feature = "codec-json", feature = "codec-binary", feature = "codec-binary-base64", feature = "codec-algol"))]
mod codec_matrix_tests;
/// Stable hashing of lib manifests, shapes, and codecs for compatibility
/// checks across versions of the constellation.
#[cfg(feature = "core")]
pub mod compat;
mod femm_exports;
/// Function authoring helpers built on the shared `Shape` engine: overload
/// cases, native function objects, and member-table construction.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod functions;
/// Lib loaders for the supported source formats (host, Lisp source, binary
/// pack, native dynamic library, and wasm) plus the standard loader registry.
#[cfg(feature = "core")]
pub mod loaders;
/// Macro authoring and expansion: the `LispMacro` contract, macro objects, the
/// registry-backed expander, and shape constructors for macro syntax.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod macros;
/// End-to-end music rendering stack that lowers a score to MIDI and renders it
/// to PCM audio through the sound libs.
#[cfg(feature = "sound-music")]
pub mod music_stack;
mod numbers_exports;
mod roadmap11_exports;
/// Core runtime installer and the embedding entry point that wires classes,
/// shapes, functions, and the default number domains into a `Cx`.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod runtime;
/// Shape authoring helpers: documented and value-backed shape wrappers plus
/// shape registration and checking utilities.
#[cfg(all(feature = "core", feature = "shape"))]
pub mod shapes;
#[cfg(feature = "proc-macros")]
pub use sim_macros::*;
// The macros' native_export output emits `::sim::codec_binary::{decode_frame,
// encode_frame}`, so the feature that enables the macros must also expose that
// module. `proc-macros` pulls `codec-binary`; this contract asserts it, so a future
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
