# sim-conformance

In one line: the runnable checklist that proves SIM actually behaves the way its architecture promises.

## What it gives you

This is the executable test suite that holds the whole runtime to its stated contract. Every big claim SIM makes about itself becomes a check you can run: that codecs faithfully round-trip every expression, that classes act as callable functions, that number parsing and promotion can be swapped out, that reading input never quietly runs code unless a host allows it, that named evaluation strategies work, that libraries can be installed and cleanly removed, that a boot receipt replays the same way, and that placement across machines behaves. It exercises the public entry crate exactly as an outside developer would, so the promises are tested through the same door you use.

## Why you will be glad

- You get proof, not prose, that the runtime keeps its word.
- A single command tells you whether the architecture still holds together.
- Changes that quietly break a core guarantee are caught before they spread.
- The checks read as a plain description of what the system commits to.

## Where it fits

This suite is the guardian of SIM's architecture claims. The written contract states how the runtime must behave; this crate turns that contract into tests that either pass or fail. It sits beside the entry crate and drives the same public surface an application would, so it measures the system as users meet it. When a contributor changes something deep in the constellation, this suite is the honest referee that confirms the whole thing still adds up.
