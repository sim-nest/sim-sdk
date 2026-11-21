// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Planning observation for `release/nv12-08`.
//!
//! The scope runs `prove plan` twice and `world why` on the program named by
//! `product.command`. The two plans must match. The plan record keeps a zero
//! effect count and lists the native leaves in its excluded set. The
//! explanation names the disclosure owner. The recorded values are that
//! program output.

use crate::{CheckObservation, PackFailure, PackRequest};

const NATIVE_LEAVES: [&str; 2] = ["leaf/native-worker-adapter", "leaf/native-assay"];

pub(super) fn observations(
    request: &PackRequest<'_>,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let program = crate::packs::product::program(request)?;
    let first = crate::packs::product::run(&program, &["prove", "plan"])?;
    let second = crate::packs::product::run(&program, &["prove", "plan"])?;
    if first != second {
        return Err(fail("plan-unstable", "prove plan"));
    }
    let record = crate::packs::product::prove_record(&first)?;
    if !record.starts_with("#(prove/plan-v1 ") {
        return Err(fail("marker-absent", "prove/plan-v1"));
    }
    crate::packs::product::require_field(record, "denominator", "29")?;
    crate::packs::product::require_field(record, "effects", "0")?;
    crate::packs::product::require_field(record, "subprocesses", "0")?;
    crate::packs::product::require_field(record, "owner-commands", "0")?;
    crate::packs::product::require_field(record, "resident", "true")?;
    for leaf in NATIVE_LEAVES {
        if !record.contains(leaf) {
            return Err(fail("marker-absent", leaf));
        }
    }
    let why = crate::packs::product::run(
        &program,
        &[
            "world",
            "why",
            "conclusion/public-release",
            "policy/no-v3-disclosure",
        ],
    )?;
    if !why.contains("world/explanation-v1") {
        return Err(fail("marker-absent", "world/explanation-v1"));
    }
    if !why.contains("owner/sim-private/disclosure") {
        return Err(fail("marker-absent", "owner/sim-private/disclosure"));
    }
    Ok(vec![
        observation("release.plan", &first),
        observation("release.why", &why),
    ])
}

fn observation(key: &str, stdout: &str) -> CheckObservation {
    CheckObservation {
        key: key.to_owned(),
        value: stdout.to_owned(),
    }
}

fn fail(code: &'static str, detail: &str) -> PackFailure {
    PackFailure {
        code,
        detail: detail.chars().take(256).collect(),
    }
}
