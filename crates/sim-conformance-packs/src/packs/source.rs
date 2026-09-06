//! Source-resolution conformance pack.

use crate::{PackRequest, PackVerdict, harness};

/// Returns `UnimplementedPack` until the funded source phase ships.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::unavailable("checker/c-source", request)
}
