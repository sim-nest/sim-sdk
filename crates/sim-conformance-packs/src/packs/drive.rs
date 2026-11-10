// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Drive conformance pack.
//!
//! Funded scopes call the one-effect conduct path. The native scope stays
//! unimplemented.

use crate::executor::{require_subject, require_support};
use crate::executor_scope;
use crate::{PackRequest, PackVerdict, harness};

/// Checks one funded drive scope against the frozen executor.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-drive", request, |request| {
        require_subject(request.subject)?;
        require_support(request)?;
        executor_scope::drive(request.scope, request.subject)
    })
}
