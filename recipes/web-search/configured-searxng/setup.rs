/// Returns the explicit operator configuration, or `None` so the diagnostic
/// runner can report a skip. The ordinary cookbook/conformance path never
/// calls a connector and never consults ambient service discovery.
pub fn configured_searxng_diagnostic() -> Option<(String, String)> {
    let endpoint = std::env::var("SIM_SEARXNG_ENDPOINT").ok()?;
    let site_config = std::env::var("SIM_SEARXNG_SITE_CONFIG").ok()?;
    Some((endpoint, site_config))
}
