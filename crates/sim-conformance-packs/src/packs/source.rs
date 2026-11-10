// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Source-resolution conformance pack.
//!
//! Each funded scope calls the source provider and checks that result.

use crate::executor::{require_subject, require_support};
use crate::source_invoke;
use crate::{PackRequest, PackVerdict, harness};

/// Checks one funded source scope against the source provider.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-source", request, |request| {
        require_subject(request.subject)?;
        require_support(request)?;
        match request.scope {
            "source/admission" => source_invoke::admission(request.subject),
            "source/performance" => source_invoke::performance(request.subject),
            "source/composed" => source_invoke::composed(request.subject),
            other => Err(crate::PackFailure {
                code: "wrong-scope",
                detail: other.into(),
            }),
        }
    })
}
