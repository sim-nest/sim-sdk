//! Cold-succession conformance pack.

use crate::{PackRequest, PackVerdict, harness};

/// Returns `UnimplementedPack` until the funded succession phase ships.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::unavailable("checker/c-succeed", request)
}
