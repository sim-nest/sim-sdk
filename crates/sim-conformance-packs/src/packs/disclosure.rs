//! Disclosure-policy conformance pack.

use crate::{PackRequest, PackVerdict, harness};

/// Returns `UnimplementedPack` until the funded disclosure phase ships.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::unavailable("checker/c-disclose", request)
}
