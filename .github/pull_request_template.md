## What this changes

<!-- One or two sentences on the change and why. -->

## Checklist

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo test -p sim-conformance` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo doc --workspace --no-deps` passes
- [ ] `SIM_CONFORMANCE_PACKS_LOCK_FILE="$PWD/Cargo.lock" SIM_CONFORMANCE_PACKS_LOCK_SHA256="$(sha256sum "$PWD/Cargo.lock" | cut -d' ' -f1)" cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [ ] `SIM_CONFORMANCE_PACKS_LOCK_FILE="$PWD/Cargo.lock" SIM_CONFORMANCE_PACKS_LOCK_SHA256="$(sha256sum "$PWD/Cargo.lock" | cut -d' ' -f1)" cargo test --workspace --all-features` passes
- [ ] `cargo run -p xtask -- simdoc --check` passes
- [ ] Tests added/updated for the behavior changed
- [ ] Source and Markdown are ASCII-only
- [ ] Commits are signed off (DCO: `git commit -s`)
