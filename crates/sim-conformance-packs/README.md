# sim-conformance-packs

`sim-conformance-packs` owns SIM's public, project-neutral checker entrypoints.
It exposes all 21 statically bound packs from the first release. Each declared
scope remains `UnimplementedPack` until the phase that funds its scenarios.

The library is pure. Callers materialize evidence through their own authorized
ports, then pass an immutable `PackSubject`. Only `PackVerdict::Pass` carries a
canonical checker-result identity that the separate `sim-conformance-core`
harness may bind into a receipt. `PackSpec::checker_binding` derives the typed
runtime binding from the immutable activation record, so an authorized adapter
can instantiate one exact subject and scope before it issues and verifies the
receipt. A pack neither stores evidence nor performs
filesystem, process, Git, network, release, or approval effects.

The implemented scopes cover retirement, canonical identity, scoped bootstrap
ownership, architecture inventory and the local adapter boundary, pure work
packets and their local effect handoff, pure artifact facets, native journal
compatibility and performance, causal journal sharing, durable operation logging
and reconciliation, the exact local command path, semantic projection admission,
projection evidence and disclosure, the read-only world product, measured query
latency, produced ownership, and release closure through NV12.06. The projection
reference specimen exercises released crates rather than workspace paths. The
facet and work packs expose public implementation traits so
independently authored foreign implementations can be judged with the same
scenario suite. Every pack consumes supplied evidence and never performs host
work.
