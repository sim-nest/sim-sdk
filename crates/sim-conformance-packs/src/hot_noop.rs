// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Hot no-op samples through the public roadmap command.
//!
//! An untimed run admits the sealed plan and stops at need-direction. One
//! untimed status call warms that command. The next twenty status calls are
//! complete samples through the serialized receipt.

use std::time::{Duration, Instant};

use sim_lib_roadmap_runner::{
    GenerationHandle, LocalExecutionIdentity, LocalRoadmapPort, LocalRoadmapReceipt,
    LocalRoadmapRequest, PublicLocalRoadmapPort,
};

const SAMPLES: usize = 20;
const SAMPLE_CEILING: Duration = Duration::from_secs(5 * 60);

const PLAN: &str = "\
sim-selected-plan-v1
provider harbor
operation conduct
execution execution
owner owner/authored
generator owner/generator
seal seal-bytes
authored fact/src authored-bytes
generated fact/out generated-bytes
claim claim/src fact/src
proof leaf same same
mutate model/file before after
land model/land base next
direction begin-work
";

/// Twenty hot status samples and the effect delta they produced.
pub(crate) struct HotNoop {
    /// Effect records added by the timed status calls.
    pub(crate) effect_delta: u64,
    /// Wall time of each timed call, in microseconds, in call order.
    pub(crate) samples_us: Vec<u64>,
}

/// Opens one sealed plan, warms one status, then times twenty status calls.
///
/// # Errors
/// Returns an error when the command refuses, a sample performs an effect, or
/// a sample exceeds five minutes.
pub(crate) fn measure() -> Result<HotNoop, String> {
    let port = PublicLocalRoadmapPort::default();
    let opened = invoke(&port, "run", None, None)?;
    if field(&opened.detail, "outcome") != "need-direction"
        || parse_count(&opened.detail, "effects")? != 0
    {
        return Err(format!(
            "plan admission was not quiescent: {}",
            opened.detail
        ));
    }
    let warm = invoke(&port, "status", None, None)?;
    let baseline = parse_count(&warm.detail, "effects")?;
    if baseline != 0 || warm.outcome != "ok" {
        return Err(format!("warmup status performed work: {}", warm.detail));
    }
    let mut samples_us = Vec::with_capacity(SAMPLES);
    let mut observed = baseline;
    for _ in 0..SAMPLES {
        let started = Instant::now();
        let receipt = invoke(&port, "status", None, None)?;
        let elapsed = started.elapsed();
        if elapsed > SAMPLE_CEILING {
            return Err(format!("hot status sample took {elapsed:?}"));
        }
        observed = parse_count(&receipt.detail, "effects")?;
        if receipt.outcome != "ok" || observed != baseline {
            return Err(format!("hot status performed work: {}", receipt.detail));
        }
        samples_us.push(u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX));
    }
    Ok(HotNoop {
        effect_delta: observed.saturating_sub(baseline),
        samples_us,
    })
}

fn invoke(
    port: &PublicLocalRoadmapPort,
    verb: &str,
    product: Option<&str>,
    answer: Option<&str>,
) -> Result<LocalRoadmapReceipt, String> {
    let request = LocalRoadmapRequest {
        verb: verb.to_owned(),
        observe: false,
        disposable_checkout: None,
        local_authority_token: None,
        identity: LocalExecutionIdentity {
            execution: "local/selected".to_owned(),
            conduct: "conduct".to_owned(),
            model_pick: "harbor".to_owned(),
            proof_catalog: "catalog".to_owned(),
            runner_generation: "generation-1".to_owned(),
        },
        plan: Some(PLAN.to_owned()),
        product: product.map(str::to_owned),
        answer: answer.map(str::to_owned),
        answer_kind: None,
        named_revision: None,
    };
    let generation = GenerationHandle::acquire(request.identity.runner_generation.clone());
    port.invoke(&request, generation)
}

fn parse_count(detail: &str, name: &str) -> Result<u64, String> {
    field(detail, name)
        .parse()
        .map_err(|_| format!("{name} is not a count in {detail}"))
}

fn field<'a>(detail: &'a str, name: &str) -> &'a str {
    detail
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&format!("{name}=")))
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_status_samples_add_no_effect() {
        let measured = measure().expect("hot");
        assert_eq!(measured.effect_delta, 0);
        assert_eq!(measured.samples_us.len(), SAMPLES);
    }
}
