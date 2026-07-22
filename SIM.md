# SIM.md -- architecture and conformance contract

This is the authored architecture contract for the SIM runtime and the document
the `sim-conformance` suite checks itself against. It is not generated; it states
the data flow, the kernel-contract claims the executable suite verifies, and the
surfaces covered by this suite versus surfaces outside this suite.

The companion narrative lives in `README.md`. This file is the machine-checked
half: the conformance harness loads SIM.md through the public facade only and
fails closed when a named claim is absent from this file, so the document and
the executable checks cannot silently drift apart.

## What SIM is

SIM is an expandable Rust runtime built around a small protocol kernel plus a
large set of loadable libraries. The kernel defines contracts; libraries provide
behavior. The data flow is fixed:

```text
tokens -> checked forms -> objects -> checked calls -> objects -> encoded forms
```

SIM is a Rust runtime with multiple codec surfaces. Lisp is one codec, not the
system identity. Everything above the kernel is a lib: syntax, codecs, classes,
functions, number domains, optimizers, checkers, evaluators, wasm adapters,
dynamic loaders, and the standard language surface.

## The conformance suite

`sim-conformance` is a test-only crate that depends on `sim` through the public
facade only (`default-features = false` plus an explicit feature set). It turns
the runtime's architecture claims into executable checks. It exercises behavior,
not source text: every assertion drives the facade or the runtime and observes a
result. The only document it reads is this file, to keep the claim list honest.

## Kernel-contract claims the suite verifies

The suite protects the checkable architecture claims below. A regression in any
of them fails the suite, and every new current architecture claim gets a matching
conformance assertion.

- **codec totality** over the shared `Expr` graph: every codec in the
  conformance general-purpose set (`lisp`, `json`, `binary`, `binary-base64`,
  `bitwise`, `bitwise-base64`, and `algol`) round-trips every `Expr` variant
  and quote mode semantically.
- **class semantics**: every registered class exposes the callable class
  protocol and constructs instances through the facade.
- **number-domain replaceability**: the number domains named by the runtime
  parse their literals and promote through the published lattice.
- **capability gating**: read-eval and read-construct are gated separately by
  capability and trust level.
- **eval policy**: the named eval policies (eager, lazy, lazy-by-need,
  strict-by-shape, hybrid) are present and selectable.
- **loader behavior**: the loader backends named by the runtime accept their
  source kinds (binary, lisp source, native dylib, and wasm).
- **reversible library lifecycle**: load/unload/reload produce observable,
  equal registry snapshots, and dependents refuse or cascade in order.
- **boot receipt replay**: boot receipts encode, decode, and replay to an equal
  state.
- the **wasm ABI scope**: wasm ABI v1 executes functions and marks richer
  exports (class, codec, shape, number-domain) explicitly unsupported.
- **stream transport conformance**: streams record, replay, and bridge across
  the transport layers with explicit skip diagnostics for unsupported pairs.

## Conformance matrices

The library lifecycle conformance matrix lives at
`crates/sim-conformance/tests/spec/lib_lifecycle.rs`.

The stream conformance matrix lives at
`crates/sim-conformance/tests/spec/stream_matrix.rs`. It covers L0 through L7
with PCM, MIDI, diagnostics, data, cancel, done, overflow, timeout, reconnect,
and refused-profile fixtures.

The placement conformance matrix lives at
`crates/sim-conformance/tests/spec/placement.rs`. It covers single-site,
multi-thread, and multi-process deterministic placement with golden report and
audio hashes.

## Stream cassette publishability

Stream cassettes carry a `to_expr`/`from_expr` serialization that round-trips
through the conformance general-purpose codec set. The suite guards that
round-trip in memory and validates the structural invariants a cassette must
satisfy before it could be published as a golden fixture (finite trace,
sequenced envelopes, replay- or preview-only transport, and no unredacted payload
or host-device name). These are in-memory invariant checks, not comparisons
against a committed `.simcassette` corpus on disk.

## Surfaces covered versus not covered

Covered by executable conformance in this suite, with the listed feature set on:

- the kernel codec, class, number, capability, eval-policy, loader, lifecycle,
  and wasm-ABI contracts;
- the CORE host primitives for table-backed filesystem read/write/edit/search,
  bounded process execution, direct HTTP table reads, and compatibility fs/net
  capability aliases;
- the stream-core, stream-combinators, stream-fabric, stream-file, stream-host,
  and web-bridge transport surfaces;
- the topology placement surface and the CLI boot surface.

Explicit ignored checks lint offline, deterministic sibling recipe corpora (not
full runtime replays): the `30-agents` and `40-atelier` recipe sets, whose
`(quote ...)` setup forms are decoded and evaluated through the lisp codec and
compared to their expected forms.

Outside this executable conformance suite (their features are off by default in
this suite): the agents, MCP, music, audio-FEMM, logic, and discrete surfaces.
Their absence is intentional and named so the suite does not over-claim coverage
it does not have.
