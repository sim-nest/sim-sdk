// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Fixed-point convergence conformance pack.
//!
//! Each funded scope calls the frozen proposal and seal functions and checks
//! that result.

use crate::executor::{require_subject, require_support};
use crate::executor_scope;
use crate::{PackRequest, PackVerdict, harness};

/// Checks one funded convergence scope against the frozen executor.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-converge", request, |request| {
        require_subject(request.subject)?;
        require_support(request)?;
        executor_scope::converge(request.scope, request.subject)
    })
}
