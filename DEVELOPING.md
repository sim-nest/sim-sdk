# Developing SIM

This is the honest, current picture for someone who wants to work on SIM. It is
deliberately frank about what is and is not buildable today, because SIM is
pre-publish.

## The shape

SIM is a constellation of repositories, not a single tree:

- 19 public **code** repositories, each publishing its own crates (the kernel,
  the shape engine, the codecs, the number-domain libraries, the list/table
  backends, the behavior libs, the server/agent substrate, and the music, audio,
  FEMM, and web stacks). `sim-sdk` (this repo) owns the umbrella `sim` crate that
  aggregates them.
- `sim-say`, the generated public front page (start there for the overview).

The umbrella `sim` crate is the single dependency surface: depend on `sim`, pick a
feature set, and you pull exactly the slice of the constellation you need.

## What builds today

- **`sim-kernel` builds and tests from a lone clone.** It is the dependency root
  with no constellation-internal dependencies:

  ```bash
  git clone https://github.com/sim-nest/sim-kernel && cd sim-kernel
  cargo test
  ```

- **Every other repo builds together, not alone.** A lone clone of, say,
  `sim-codecs` does not compile yet: its cross-repo dependencies are version
  requirements that resolve from crates.io, and nothing is published there yet
  (you will see `error: no matching package named ...`). The full constellation
  builds as one Cargo workspace assembled by the project's build tooling; that
  assembly is not yet a public, one-command step.

## How the full build will open up

The public build story completes at the **first crates.io publish**. After that:

- users add one line -- `sim = "0.1"` -- and the umbrella crate pulls the
  constellation from the registry;
- any single repo builds from a lone clone, because its cross-repo dependencies
  resolve from crates.io.

Publishing is a deliberate, human-gated step (it is the only path to GitHub and
crates.io for this project). Until then, treat anything beyond `sim-kernel` as
"builds in the workspace".

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
`github.com/sim-nest`. Because
the constellation spans many repositories, a change to a split crate is made in
that crate's own repository. If you are unsure where something lives, the front
page (`sim-say`) carries the full repository catalog.
