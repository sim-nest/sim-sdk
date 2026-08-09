# Capability-scoped embedded Python

This recipe uses the SDK's ordinary codec and profile exports. It proves that
dynamic `eval` and `exec` fail without `read-eval`, succeed only after the host
grants and then diminishes that authority, directly evaluates lowered Python,
and inspects the declared compiler/CPython gap. It creates no Python executable
or alternate runtime bootstrap.
