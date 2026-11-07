# Developing SIM

This is the current build picture for someone who wants to work on SIM. The
public repositories build from their own checkouts against crates.io
dependencies; maintainers also use the private control-plane workspace for
full-constellation checks.

## The shape

SIM is a constellation of repositories, not a single tree:

- public **code** repositories, each publishing its own crates (the kernel,
  the shape engine, the codecs, the number-domain libraries, the list/table
  backends, the behavior libs, the server/agent substrate, and the music, audio,
  FEMM, and web stacks). `sim-sdk` (this repo) owns the umbrella `sim` crate that
  aggregates them.
- `sim-say`, the generated public front page (start there for the overview).

The umbrella `sim` crate is the single dependency surface: depend on `sim`, pick a
feature set, and you pull exactly the slice of the constellation you need.

## Public checkout builds

Each public repository builds and tests from its own checkout. The root crate and
the conformance suite in this repository use the published crate graph unless a
local maintainer workspace overrides it deliberately.

```bash
git clone https://github.com/sim-nest/sim-sdk
cd sim-sdk
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
lock_sha="$(sha256sum "$PWD/Cargo.lock" | cut -d' ' -f1)" && printf '%s\n' "$lock_sha" | grep -Eq '^[0-9a-f]{64}$' && SIM_CONFORMANCE_PACKS_LOCK_FILE="$PWD/Cargo.lock" SIM_CONFORMANCE_PACKS_LOCK_SHA256="$lock_sha" cargo clippy --locked --workspace --all-features --all-targets -- -D warnings
lock_sha="$(sha256sum "$PWD/Cargo.lock" | cut -d' ' -f1)" && printf '%s\n' "$lock_sha" | grep -Eq '^[0-9a-f]{64}$' && SIM_CONFORMANCE_PACKS_LOCK_FILE="$PWD/Cargo.lock" SIM_CONFORMANCE_PACKS_LOCK_SHA256="$lock_sha" cargo test --locked --workspace --all-features
cargo run -p xtask -- simdoc --check
```

The two all-features chains stop before cargo when `Cargo.lock` is missing
or its digest is not 64 lowercase hex characters. `cargo test --workspace`
above writes that lock on a local clone. The all-features CI job writes the
gitignored lock itself, refuses the same bad digest, then runs the two
`--locked` commands.

The `sim-kernel` repository remains the dependency root and has no
constellation-internal dependencies. Other repositories declare versioned
dependencies on the published crates they need.

## Maintainer constellation checks

The private `sim-private` control plane assembles a generated Cargo workspace for
cross-repository checks. That workspace is a maintainer tool, not source:

```bash
cd ../sim-private
sh bin/simctl meta-build
sh bin/simctl test-all
```

Use the control-plane workspace when a change crosses repository or ABI
boundaries. Keep source edits in the owning public repository.

## Architecture and conformance

- The architecture contract is [`SIM.md`](SIM.md) -- the machine-checked half of
  the design, verified by the conformance suite. The narrative overview lives on
  the front page (`sim-say`).
- The conformance suite is the `sim-conformance` crate; it exercises the public
  facade and protects the runtime's architecture claims. Run it in the workspace
  with `cargo test -p sim-conformance`.
- Per-crate API contracts live in each repository's `rustdoc`.

## License and contributing

SIM is licensed under **MPL-2.0** (see `LICENSE` in each repository).

Issues, questions, and patches go through the project's GitHub repositories at
`github.com/sim-nest`. Because the constellation spans many repositories, a
change to an implementation crate is made in that crate's owning repository. If
you are unsure where something lives, the front page (`sim-say`) carries the
full repository catalog.
