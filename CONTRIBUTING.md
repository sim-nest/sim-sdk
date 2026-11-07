# Contributing

Thanks for your interest in SIM. This repo is one crate group in a constellation
of repos that build together; contributions of all sizes are welcome.

## Building and testing

This repo is self-contained and builds against the published SIM crates on
crates.io -- no extra tooling or sibling checkouts are required:

- Clone this repo and run `cargo build` and `cargo test --workspace`.
- Cross-repo dependencies resolve from crates.io; dependencies within this repo
  resolve locally.

## What a pull request must pass

Every PR runs these gates in CI, and they must be green before merge:

- `cargo fmt --all --check`
- `cargo test -p sim-conformance`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo doc --workspace --no-deps`
- `lock_sha="$(sha256sum "$PWD/Cargo.lock" | cut -d' ' -f1)" && printf '%s\n' "$lock_sha" | grep -Eq '^[0-9a-f]{64}$' && SIM_CONFORMANCE_PACKS_LOCK_FILE="$PWD/Cargo.lock" SIM_CONFORMANCE_PACKS_LOCK_SHA256="$lock_sha" cargo clippy --locked --workspace --all-features --all-targets -- -D warnings`
- `lock_sha="$(sha256sum "$PWD/Cargo.lock" | cut -d' ' -f1)" && printf '%s\n' "$lock_sha" | grep -Eq '^[0-9a-f]{64}$' && SIM_CONFORMANCE_PACKS_LOCK_FILE="$PWD/Cargo.lock" SIM_CONFORMANCE_PACKS_LOCK_SHA256="$lock_sha" cargo test --locked --workspace --all-features`
- `cargo run -p xtask -- simdoc --check`

The all-features job writes the gitignored root `Cargo.lock` before it
hashes that file. It refuses a digest that is not 64 lowercase hex
characters, then runs the two `--locked` commands above. The contributor
commands use the same refusal: a missing lock or a bad digest stops the
chain before cargo starts.

CI runs the standalone public-facade conformance suite with
`cargo test -p sim-conformance`. Conformance tests whose ignored-test output
names sibling recipe corpora or the generated constellation meta-workspace are
maintainer checks, not GitHub CI gates.

Please keep source and Markdown ASCII-only, and add or update tests for behavior
you change. Public APIs carry `#![deny(missing_docs)]`; document new public items.

## Sign your work (DCO)

We use the Developer Certificate of Origin, not a CLA. Add a `Signed-off-by` line
to each commit certifying you wrote the change or have the right to submit it:

```
git commit -s -m "your message"
```

This adds `Signed-off-by: Your Name <you@example.com>`. That is all we need; there
is no copyright-assignment agreement to sign.

## License

By contributing you agree that your contributions are licensed under the
repository's MPL-2.0 license (see `LICENSE`).

## Filing issues

Use the issue templates. A small reproducible example beats a long description.
Security-sensitive reports go through `SECURITY.md`, not public issues.
