//! Outward-delivery conformance pack.

use crate::{PackRequest, PackVerdict, harness};

/// Returns `UnimplementedPack` until the funded delivery phase ships.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::unavailable("checker/c-deliver", request)
}
