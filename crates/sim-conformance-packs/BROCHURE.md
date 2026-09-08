# sim-conformance-packs

In one line: Run named SIM conformance checks over explicit facts and receive exact, reproducible evidence identities.

## What it gives you

`sim-conformance-packs` provides a public catalog of checkers for retirement guards, semantic identity, scoped ownership, architecture boundaries, work packets, durable operation recovery, exact local command effects, artifact facets, qualified semantic projection, disclosure scoping, the read-only world product, measured query latency, and release claims. Every invocation names its checker, scope, subject, input facts, implementation identity, and result. The checkers observe only the facts supplied by the caller, so the same accepted input yields the same passing-result identity on every host.

The catalog also exposes narrow ports for checking packet and facet implementations owned by other crates. Unsupported checker names and scopes fail closed with typed errors. The crate performs no process execution, repository mutation, network access, or receipt storage.

## Why you will be glad

- Evidence stays bound to the exact subject and scope that a checker examined.
- Tools can compare canonical result identities without trusting prose or host-local paths.
- New checker implementations cannot silently widen an existing public contract.
- Tests can inject complete fact sets without arranging ambient machine state, while the projection reference specimen separately checks the released implementation crates from crates.io.

## Where it fits

Within SIM, sim-conformance-packs is the public policy layer above `sim-conformance-core`. Operator tools gather real observations and retain receipts; domain crates own the behavior being judged. This separation keeps conformance deterministic, makes authority visible, and lets independent tools share one checked vocabulary.
