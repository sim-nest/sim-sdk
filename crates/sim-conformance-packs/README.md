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

The implemented scopes cover retirement, identity register/vector and canonical
journal identities, activation ownership, boundary inventory, pure work-packet,
pure artifact-facet, native journal compatibility and replay performance, causal
journal sharing, and the NV12.01 release closure. The facet and work packs expose
public implementation traits so independently authored foreign implementations
can be judged with the same scenario suite. Journal packs consume explicit
synthetic-model, measurement, and reducer evidence; they never perform host work.
