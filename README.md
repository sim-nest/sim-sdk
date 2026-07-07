# sim-sdk

`sim-sdk` is the **developer entry point** for SIM and the home of the umbrella
`sim` crate: it gives you one dependency to build applications and libraries
against the SIM runtime. New to SIM? Read the overview first on the front page
(`sim-say`); this README is for writing code against the runtime.

Just want to run SIM? Install the `sim` command from `sim-run`
(`cargo install sim-run`) and see `sim-say` -- you do not need this crate to use
SIM, only to build against it.

This repo owns two crates plus the architecture contract:

- **`sim`** -- the umbrella crate: one dependency surface that aggregates the
  constellation's kernel, codecs, number domains, list/table backends, and
  behavior libs, with the core runtime installer (`install_core_runtime`) and
  authoring helpers (`functions`, `classes`, `macros`, `shapes`, `runtime`).
- **`sim-conformance`** -- the executable spec suite that exercises the public
  facade and protects the runtime's architecture claims (codec totality over
  `Expr`, class-as-function behavior, replaceable number-domain parsing and
  promotion, read-eval / read-construct security, named eval policies, loader
  backends, reversible library lifecycle, boot receipt replay, the wasm ABI
  export scope, stream transport, and placement conformance).
- **`SIM.md`** -- the machine-checked architecture contract the conformance suite
  verifies. Per-crate API contracts live in each repo's published `rustdoc`.

## Quickstart

Add the umbrella crate (the single dependency surface for the whole
constellation):

```toml
[dependencies]
# Published as `sim-nest` (the name `sim` was taken); imported as `sim`.
sim = { package = "sim-nest", version = "0.1" }   # default features: core, codec-lisp, numbers-f64
```

Boot a runtime in a few lines:

```rust
use std::sync::Arc;
use sim::kernel::{Cx, DefaultFactory, EagerPolicy};
use sim::runtime::install_core_runtime;

let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
install_core_runtime(&mut cx);
// install codecs and behavior libs via their install_* / Lib + Linker paths,
// then cx.eval_expr(...).
```

Widen the feature set as you reach for more of the constellation (more codecs,
number domains, the music/audio/FEMM/web stacks) -- see "Default features" and
"Optional feature families" below. Confirm your build resolves the umbrella with:

```bash
cargo build      # or `cargo add sim-nest --features standard,server,wasm` first
```

For a complete, runnable version of this boot-and-eval loop, see
[`examples/repl.rs`](examples/repl.rs) -- a tiny REPL that installs the number
domains and the Lisp codec, then reads, evaluates, and prints each line
(`cargo run --example repl --features shape,numbers-prelude`).

(The full architecture conformance suite `sim-conformance` is a maintainer gate run
across the whole constellation, not a published crate; contributors run it from the
repo checkout.)

Naming note: the umbrella crate is published on crates.io as `sim-nest` (the
bare name `sim` was already taken), but it keeps the library import identifier
`sim` -- so you depend on it as `sim = { package = "sim-nest", version = "0.1" }`
(or simply `sim-nest = "0.1"`) and write `use sim::...` throughout, and the
`#[sim::sim_lib]` proc-macros resolve against it unchanged. Do NOT write
`use sim_nest::...` -- the crate's library name is `sim`, so `sim_nest` will not
resolve (cargo's error even suggests `cargo add sim_nest`, which is wrong). The
whole constellation is live on crates.io. See [`DEVELOPING.md`](DEVELOPING.md) for
the contributor build. The architecture narrative and the data-flow overview live on
the front page (`sim-say`); the sections below are the developer's architecture
reference.

## Architecture

SIM is organized around a protocol kernel with behavior layered above it:

```text
+-----------------------------------------------------------------------+
|                              Codec Surfaces                           |
|  classes, functions, libs, codecs, shapes, parsers, eval policies     |
+--------------------------+--------------------------------------------+
                           |
+--------------------------v--------------------------------------------+
|                              Lib Runtime                              |
|  registry, capabilities, versioning, loading, linking, reflection     |
+------+-------------------+------------------+-------------------------+
       |                   |                  |
+------v------+     +------v------+    +------v-------------------------+
| Object Model|     | Shape Engine|    | Codec Engine                  |
| class/call  |     | parse/check |    | decoder/encoder/read-eval     |
+------^------+     +------^------+    +------^-------------------------+
       |                   |                  |
+------v-------------------v------------------v-------------------------+
|                              Kernel                                   |
|  Value handles, Cx, Factory, EvalPolicy, ClassId, LibId, errors       |
+-----------------------------------------------------------------------+
```

The kernel has no opinion about Lisp syntax, JSON, number towers, lazy values,
dynamic libraries, or user-facing help. It defines only the contracts that let
libs provide those things.

### Kernel boundary

The kernel is a small protocol surface. If a feature can live as data plus a lib
contract, it should not become another closed kernel subsystem. The kernel
**may** define identity and transport types (`Symbol`, `Expr`, `Value`,
`Origin`, `Ref`, `Datum`, diagnostics, errors, stable ids); coordination types
(`Cx`, `Registry`, `Lib`, `Linker`, `ExportRecord`, capabilities, claim/fact and
handle stores, Card records, operation specs, event/effect ledgers, control
policy, rank metadata); behavior contracts (object, callable, class, shape,
factory, eval-policy, macro-expander); shape match/binding result types; and the
ABI frame and manifest transport shapes. The kernel **must not** define concrete
Lisp/JSON/Algol parsing, concrete number domains or arithmetic, concrete
help/test/browse implementations, wasm guest behavior above the ABI transport,
or remote transport and agent-product policy. New metadata is modeled as open
data (`ExportRecord`-style) before any new closed kernel enum.

### Shape: one shared engine

`Shape` is the bold center of the design and a first-class kernel protocol
(object-accessible via `as_shape`, callable as a matcher, subclassable through
open metadata). Parsing, checking, binding, destructuring, dispatch, macro
syntax, codec grammar, lambda local environments, and overload selection are all
specializations of this one engine: a parser recognizes structure and produces
values, a checker recognizes structure and accepts or rejects, a binder names
parts, an overload selector chooses behavior, and a codec translates between
external syntax and internal forms. The kernel defines the `Shape` protocol;
concrete shape behavior lives in `sim-shape` and other libs. When a subsystem
needs validation, reuse a `Shape` rather than adding a closed checker to the
kernel.

### Universal expression graph and codecs

Every codec targets one universal `Expr` graph, wider than ordinary Lisp lists,
with nil, bools, numbers, symbols, locals, strings, bytes, lists, vectors, maps,
sets, calls, infix/prefix/postfix forms, blocks, quotes, annotations, and tagged
extensions. Codecs are first-class runtime objects, split into independent
**decoders and encoders** (real systems often need one without the other);
encoders know their output position (eval, quote, data, pattern). General-purpose
expression codecs are **total over `Expr`** -- they round-trip every expression
semantically, using a standard escape form rather than failing -- while domain
codecs (such as chat or MCP) round-trip only their domain and fail closed
outside it. `Expr` may carry optional `Origin` metadata so encoders can offer a
canonical mode (stable, minimal trivia) and a lossless mode (origin preserved).
`proc-macro2` is the portable token-tree substrate for textual codecs, and Pratt
operator metadata is shared protocol data while concrete parsers stay
codec-owned.

### Classes are functions

Every object has a class, every class is itself an object, and every class is
callable -- calling a class constructs an instance, so `(Point 1 2)` is the same
operation as `(call Point 1 2)`. Because constructors are ordinary callables,
they participate in overload, shape checking, help, reflection, and codec
round-trips for free.

### Evaluation policy and realize / EvalFabric

Evaluation strategy is injectable, not hardwired: eager, lazy thunks,
lazy-by-need, hybrid per-argument demand, and a no-op policy are available, and a
named policy can be selected at runtime. Distributed evaluation is
location-transparent through the `realize` path and the `EvalFabric` contract --
server and agent code targets `realize` and the eval fabric, never a
transport-specific API.

### Pluggable backends

Number representation, lists, and tables are library concerns. Number domains
range from a tiny `f64` system to bigint, rational, complex, fixed/float,
symbolic CAS, and arbitrary-dimensional tensors; codecs delegate numeric
literals to the active domains by parse priority, and arithmetic is just
overload. List and table backends are likewise swappable libs with kernel
defaults.

### Wasm and dynamic loading

Wasm is a first-class runtime target and the portable plugin ABI: the binary
codec is the default ABI payload, and ABI v1 executes binary-frame function
exports over a minimal host callback set. Loader backends cover host-registered
libs, binary packs, Lisp-source libs, wasm modules, and (off wasm32, behind the
`dynamic-native` feature) native dynamic libraries. Loading is capability-gated;
native dynamic loading is never implicit.

### Security model

Power is explicit. Read-eval is a capability, separate from the narrower
capability-gated **read-construct** path that backs Lisp `#(...)` literals;
file, network, clock, random, process, and host calls are capabilities; codecs
run with decoder capabilities rather than ambient power; and shapes used for
validation must be pure unless explicitly marked effectful. Libs declare the
capabilities they request, and hosts grant them.

### Reflection

Every framework exposes ordinary runtime objects through stable `core/*`
surfaces, so an agent can ask what classes, functions, shapes, codecs, number
domains, eval policies, libs, and exports are loaded, and read help as data
rather than prose buried in source.

## The umbrella `sim` crate

The implementation libraries SIM loads do not live in this repository. They are
sibling repositories in the SIM constellation, each publishing its own crates
(the kernel, the shape engine, each codec, each number-domain library, the
list/table backends, and the behavior libs). The `sim` crate's `Cargo.toml` is
the canonical feature map: every optional library is an optional dependency,
and a feature turns it on and pulls it into the aggregate. Consult that file for
the authoritative, current list; the families below are a map, not a copy.

### Default features

```text
default = ["core", "codec-lisp", "numbers-f64"]
```

- **`core`** brings in `sim-kernel` (the protocol kernel) and `sim-lib-core`
  (the core runtime library).
- **`codec-lisp`** brings in the Lisp reader/printer codec surface.
- **`numbers-f64`** brings in the default `f64` number domain.

This is the minimal useful runtime: kernel contracts, one general-purpose codec,
and one number domain.

### Optional feature families

The large optional surface is organized into families. Each feature is gated in
`Cargo.toml` and may imply other features it depends on:

- **`shape`** -- the `sim-shape` engine (one shared engine for parsing,
  checking, binding, dispatch, macro syntax, codec grammar, and overload
  selection).
- **`codec-*`** -- additional codecs: `codec-json`, `codec-binary`,
  `codec-binary-base64`, `codec-chat`, `codec-mcp`, `codec-algol`.
- **`numbers-*`** -- pluggable number domains: `numbers-arith`, `numbers-i64`,
  `numbers-rational`, `numbers-complex`, `numbers-bigint`, `numbers-tensor-*`,
  the `numbers-cas-*` symbolic stack, and the `numbers-prelude` aggregate, among
  many more.
- **`list-*` / `table-*`** -- pluggable list and table backends: `list-cell`,
  `list-lazy`, `table-hash`, `table-override`, `table-lazy`, `table-fs`,
  `table-db`, `table-remote`.
- **`control`** -- control-flow and policy library.
- **`standard-*`** -- the standard distribution and language surface libs
  (`standard-core`, binding, sequence, pattern, dispatch, namespace, mutation,
  and the `lang-*` front ends such as Scheme, Clojure, Common Lisp, Julia, Lua,
  and Ruby).
- **`skill-*` / `mcp-*`** -- agent skills and Model Context Protocol surfaces.
- **`stream-*`** -- the event-stream core, combinators, fabric, and host audio
  and MIDI stream backends.
- **`pitch-*` / `midi-*` / `music-*` / `sound-*`** -- the music, pitch, MIDI,
  and sound stack, with `music-stack` as the convenience aggregate.
- **`audio-graph-*` / `audio-dsp` / `audio-synth` / `plugin-*` / `daw-session`**
  -- the audio graph, DSP, synthesis, plugin hosting (CLAP, LV2, VST3), and DAW
  session libs.
- **`femm-*`** -- the finite-element / numeric-physics (FEMM) stack.
- **`topology-*`**, **`view-*` / `web-*`**, **`rank-*`**, **`logic-*`**,
  **`intent`**, **`scene`**, **`discrete-*`** -- additional behavior families.
- **`server` / `server-net-http`**, **`agent` / `agent-net` /
  `agent-runner-*`**, **`openai-server*`** -- server, agent, and agent-runner
  surfaces.
- **`wasm`** -- the wasm ABI transport (a first-class plugin ABI and runtime
  target). **`dynamic-native`** -- native dynamic library loading.
- **`proc-macros`** -- the `sim-macros` procedural macros.

Because feature edges encode real dependencies, enabling a high-level feature
(for example `music`, `daw-session`, or `numbers-prelude`) transitively enables
the libs it needs. Use `default-features = false` plus an explicit feature list
to build a tailored runtime, as `sim-conformance` does.

### Booting a runtime

The umbrella crate's runtime installer is the entry point for embedding SIM:

```rust
use std::sync::Arc;
use sim::kernel::{Cx, DefaultFactory, EagerPolicy};
use sim::runtime::install_core_runtime;

let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
install_core_runtime(&mut cx);
// install codecs and behavior libs via their install_* / Lib + Linker paths,
// then cx.eval_expr(...).
```

`install_core_runtime` loads the core runtime through the lib registry and
installs the default number domain(s) for the enabled `numbers-*` features.
Codecs and additional behavior libraries are installed the same way every other
lib is: through their own `install_*` helper or directly through `Lib` and
`Linker`.

---

## Extending SIM

The governing rule is the SIM rule: **the kernel defines contracts; libraries
provide behavior.** New behavior should enter through values, libs, shapes,
codecs, macros, and loaders -- not by hardwiring a new subsystem into the
kernel. Only extend kernel types when the new contract is genuinely
protocol-level. When metadata exposure grows, prefer `ExportRecord`-style data
over new closed kernel enums plus parallel maps.

This section is the extension-surface guide for the runtime as it exists today.
The exact trait shapes and module paths are authoritative in each crate's
`rustdoc`; the trait sketches here are conceptual.

### 1. Runtime values and objects

Every runtime value crosses the public API as `Value`, which wraps an
`Arc<dyn RuntimeObject>`. The extension contract is split between two traits in
the kernel:

- `Object` -- the small root protocol: headers, operations (`op`), claims,
  snapshots, display, and downcasting (`as_any`).
- `ObjectCompat` -- the compatibility protocol with optional `as_*` adapters for
  class, callable, shape, object-encoder, read-constructor, number domain,
  number value, eval fabric, stream, sequence, thunk, list, table, and dir.

A minimum practical object implements `display`, `as_any`, and (unless
deliberately anonymous) `class`. Add the optional surfaces only as the value
needs to participate in a contract:

- `header`, `op`, `claims`, `snapshot` -- to take part in refs, operations,
  facts, Cards, content addressing, or effect records.
- `as_expr` -- to round-trip as structured data.
- `as_table` -- to be browseable through help and registry surfaces.
- `as_object_encoder` -- for codec-aware object encoding.
- the remaining `as_*` adapters -- when the object is a class, callable, shape,
  read-constructor, number domain/value, eval fabric, stream, sequence, thunk,
  list, table, or dir.

Use `Object::op` plus an op spec when the new behavior is an operation on an
existing value, rather than minting a new value type.

Construct extension values through the active factory:

```rust
let value = cx.factory().opaque(Arc::new(my_object))?;
```

Public SIM-facing value types must satisfy the citizen policy: derive or
hand-write a `Citizen`, or place exactly one inline exemption with a concrete
reason and kind at the type definition. Reconstructable values should encode
through class-backed constructors and the capability-gated read-construct path;
live host resources should expose inert descriptor citizens instead of
reconstructing handles. Exporting behavior libraries also need crate-local
cookbook recipes; a strict recipe gate keeps the cookbook a runtime projection
over crate-shipped recipe cards.

### 2. Functions and callable values

Any object becomes callable by returning `Some(self)` from `as_callable()` and
implementing `Callable`. For ordinary native functions, use the standard
machinery instead of a bespoke callable type:

1. build one or more function cases,
2. wrap them in a function object,
3. export the function through a `Lib` or `Linker`.

Dispatch is shape-driven: raw call syntax is mediated by the active eval policy,
argument forcing produces prepared args, a `Shape` selects a case and captures
bindings, and an optional result-shape check validates the return value.

### 3. Classes and instances

A class implements the `Class` contract (which extends `Callable`): it reports
its id and symbol, its parents and subclass relationships, its constructor and
instance shapes, an optional read-constructor, and a members table. Constructor
calls forward through `Callable`; constructor and instance shapes and member
functions are exported as browseable, callable values; and `as_table()` on the
class provides a stable browse surface. The standard instance object returns a
structural object expression from `as_expr()`, field tables from `as_table()`,
and constructor encoding from `as_object_encoder()` -- which is what lets
quote-position Lisp encoding emit `#(Class ...)` when the encode policy allows
it.

### 4. Shapes

Shapes are not hardwired kernel behavior; the kernel defines the `Shape`
protocol and the shape engine lives in `sim-shape`. A shape implements
`Callable` and can be invoked with a value or expression, returning a match
object with binding captures, a score, and diagnostics. Shapes power overload
selection, lambda parameter binding, macro syntax checking, class documentation,
codec metadata, help/browse output, and shape inheritance.

When a new subsystem needs validation, reuse a `Shape` rather than adding a
closed enum or an ad hoc checker into the kernel.

### 5. Macros

Macros lower syntax. A macro reports its symbol and a syntax shape, and expands
an input expression with captures. Expansion is phase-aware and bounded by depth
and step limits, hygienic symbol generation is available, syntax checking is
shape-driven, and source-defined template macros are supported through the
loader/runtime path. Use macro expansion when a surface needs syntactic
lowering -- do not hide new evaluation semantics inside a codec when the real
feature is macro-like.

### 6. Read constructors and object encoding

Read constructors back the Lisp `#(...)` surface. A read constructor reports its
symbol, an args shape, and a `construct_read` that returns a runtime object. The
read path requires the `read-construct` capability at both the codec and `Cx`
levels, resolves the target class from the registry, and calls through the
class's read-constructor value.

The object-to-codec bridge is the object-encoder contract, whose encoding cases
are constructor (`class` + `args`), tagged data (`tag` + `fields`), and opaque
(`class` + stable id). Consequently quote-position Lisp encoding can emit
`#(Class ...)`, eval-position emits `(Class ...)`, and other positions fall back
to an `(object ...)` form. Broad read-eval surfaces are not part of normal
object encoding.

### 7. Codecs

Codecs are first-class runtime objects, integrated through the codec runtime and
split into decoders and encoders. The extension surface is a set of optional
helpers (plain decoder/encoder, located, and tree variants). Plain decode/encode
needs only the plain helpers; located decode falls back to plain decode with no
origin; tree decode falls back to recursive reconstruction from the decoded
`Expr`; and located/tree encode only engage specialized encoders when lossless
origin is requested. A new codec can therefore start with plain `Expr` support
and add origin-aware surfaces later without changing the public helper API.
General-purpose expression codecs round-trip every expression semantically
through the shared `Expr` graph; domain codecs round-trip only their domain and
fail closed outside it.

### 8. Number domains

Number representation is provided by libraries, not by the kernel or codecs. A
number domain plugs in through the `as_number_domain` / `as_number_value`
compatibility hooks and registry registration. On decode, the Lisp and Algol
readers call `cx.parse_number_literal(text)`; installed domains are tried in
parse-priority order, and the first accepting domain wins. Add a new numeric
family as a number-domain library and register it, rather than teaching each
codec a new concrete number type.

### 9. Eval fabrics, libs, and loaders

The location-transparent distributed evaluation surface is the `EvalFabric`
contract (a single `realize` entry point over an eval request/reply). The
in-tree runtime installs a local fabric and a Lisp-visible `realize` path.
Evaluation strategy is injected through an eval policy (eager, hybrid, need, and
no-op policies exist). Server and agent code should target `realize` and the
eval fabric, never transport-specific APIs.

For packaging and export, use the library path: implement `Lib`, provide an
honest manifest, and register values through `Linker`. Loaders and browse
surfaces expect exports to flow through the export / export-record metadata
path; prefer that over inventing new side registries.

### 10. Browse, help, and test surfaces

The runtime exposes agent-facing reflection through stable `core/*` surfaces:
help, tests, lib-tests, run-tests, functions, classes, macros, shapes, codecs,
number-domains, eval-policies, and browseable lib manifests and export records.
Extensions should preserve this property: exported values should have stable
symbols and publish honest claims, snapshots, Cards, or `as_table()` summaries;
shapes, classes, codecs, and tests should describe what is actually loaded. If a
new framework cannot explain itself through the existing browse/help/test
surfaces, treat that as an extension bug.

Domains that expose streaming should keep domain typing at the boundary and then
adapt into the shared event stream (`domain payload -> Expr or data packet ->
chunk event -> stream frame`), documenting the surfaces through browse Cards and
facets instead of adding a new kernel hook.

### Practical checklist

When adding behavior in the current tree:

1. start with a value-level contract (`Object`/`ObjectCompat`, `Op`, `Callable`,
   `Class`, `Shape`, read constructor, object encoder, eval fabric, codec
   traits, sequence, stream, list, table, or dir);
2. export it through a `Lib` and `Linker`;
3. make it browseable through claims, Cards, facets, snapshots, `as_expr()`, or
   `as_table()`;
4. add round-trip or runtime tests in the crate that owns the behavior;
5. only extend kernel types when the new contract is truly protocol-level.

## Conformance

`sim-conformance` is a test-only crate that depends on `sim` through the public
facade only (`default-features = false` plus an explicit feature set). It turns
the runtime's architecture claims into executable checks, so a regression in
codec totality, class semantics, number-domain replaceability, capability
gating, eval policy, loader behavior, reversible library lifecycle, boot
receipt replay, the wasm ABI scope, stream transport conformance, or placement
conformance fails the suite. New current architecture claims get matching
conformance assertions.

The library lifecycle conformance matrix lives at
`crates/sim-conformance/tests/spec/lib_lifecycle.rs`. It covers observable
registry snapshot equality for load/unload, reload equality, absent unload,
dependent refusal and cascade order, live-reference behavior, boot receipt
encode/decode and replay, and standard profile/control receipt retraction.

The stream conformance matrix lives at
`crates/sim-conformance/tests/spec/stream_matrix.rs`. It covers L0 through L7
with PCM, MIDI, diagnostics, data, cancel, done, overflow, timeout, reconnect,
and refused-profile fixtures. Unsupported fixture/profile pairs emit explicit
skip diagnostics.

The placement conformance matrix lives at
`crates/sim-conformance/tests/spec/placement.rs`. It covers single-site,
multi-thread, and multi-process deterministic placement with golden report and
audio hashes; server and LAN placements match their declared latency classes;
and clock crossings that cannot be sample-exact carry bridge diagnostics.

| Layer | Profile | Supported fixtures | Explicit skips |
| --- | --- | --- | --- |
| `L0-memory` | `memory-local` | PCM, MIDI, diagnostics, data, cancel, done, overflow, timeout | reconnect, refused profile |
| `L1-coroutine` | `memory-event-projection` | PCM, MIDI, diagnostics, data, done | cancel, overflow, timeout, reconnect, refused profile |
| `L2-thread` | `bounded-push-queue` | PCM, diagnostics, cancel, overflow, timeout | MIDI, data, done, reconnect, refused profile |
| `L3-host` | `fake-host-callback` | PCM, MIDI, data, cancel, overflow | diagnostics, done, timeout, reconnect, refused profile |
| `L4-process` | `remote-stream-fabric` | PCM, MIDI, diagnostics, data, done, refused profile | cancel, overflow, timeout, reconnect |
| `L5-lan` | `lan-midi-control` | PCM, MIDI, diagnostics, data, done | cancel, overflow, timeout, reconnect, refused profile |
| `L6-browser` | `fixture-browser-bridge` | PCM, diagnostics, data, cancel, overflow, reconnect | MIDI, done, timeout, refused profile |
| `L7-wan` | `remote-stream-fabric` | diagnostics, data, done, refused profile | PCM, MIDI, cancel, overflow, timeout, reconnect |

## Server Runtime Surface

Server behavior is a library surface, not a workspace CLI in this repository.
Enable the `server` or `server-net-http` feature on the umbrella crate to embed
the server contracts and `install_server_lib` re-export. Command-line hosts load
the server entrypoint through the SIM bootloader, for example
`sim --load symbol:server server ...`, so server startup uses the same loader
and registry receipt path as other SIM behavior.

## Building and validating

The whole constellation is published on crates.io at `0.1.0`, so `cargo add
sim-nest` (imported as `sim`) resolves the umbrella from a public registry, and
every library resolves standalone. Each public repo also builds and tests from a
lone clone against crates.io. See [`DEVELOPING.md`](DEVELOPING.md) for how to work
on SIM across the constellation.

Build a tailored runtime by selecting features on the dependency (the crate is
`sim-nest`, imported as `sim`):

```toml
[dependencies]
sim = { package = "sim-nest", version = "0.1", default-features = false, features = ["core", "codec-lisp", "numbers-f64", "server"] }
```

Contributors, from a repo checkout, run the repository validation gate:

```bash
cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo doc --workspace --no-deps
cargo run -p xtask -- simdoc --check    # `xtask` is a repo-local dev tool, not a published crate
```

`cargo run -p xtask -- simdoc` builds the public documentation lanes (API docs,
agent cards, human docs, and diagrams) and the split contract files under
`docs/`. Everything under `docs/` is generated; do not hand-edit it.

## License

MPL-2.0.
