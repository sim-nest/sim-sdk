# Configured SearXNG diagnostic

This separately invoked operator diagnostic accepts `SIM_SEARXNG_ENDPOINT` and
`SIM_SEARXNG_SITE_CONFIG`. It skips when either is absent. It is not part of the
default conformance suite, does not discover a service from `test-hosts.toml`,
and cannot replace the recorded, network-free fixture proof.
