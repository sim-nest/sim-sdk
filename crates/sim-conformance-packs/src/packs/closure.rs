//! Closure-planning conformance pack.

use crate::{PackRequest, PackVerdict, harness};

/// Returns `UnimplementedPack` until the funded closure phase ships.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::unavailable("checker/c-closure", request)
}
