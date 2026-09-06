//! Common pure dispatch and result construction.

use std::collections::BTreeMap;

use sim_conformance_core::CheckerResultId;
use sim_kernel::{Datum, Symbol};

use crate::find_pack;

/// An immutable subject projection supplied to a pack.
pub trait PackSubject {
    /// Returns one exact, pre-materialized fact.
    fn fact(&self, key: &str) -> Option<&str>;

    /// Returns the stable fact keys in sorted order.
    fn fact_keys(&self) -> Vec<&str>;
}

impl PackSubject for BTreeMap<String, String> {
    fn fact(&self, key: &str) -> Option<&str> {
        self.get(key).map(String::as_str)
    }

    fn fact_keys(&self) -> Vec<&str> {
        self.keys().map(String::as_str).collect()
    }
}

/// Convenient in-memory subject for callers and conformance fixtures.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemorySubject {
    facts: BTreeMap<String, String>,
}

impl MemorySubject {
    /// Adds or replaces one named fact.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.facts.insert(key.into(), value.into());
        self
    }

    /// Borrows the canonical fact map.
    pub const fn facts(&self) -> &BTreeMap<String, String> {
        &self.facts
    }
}

impl PackSubject for MemorySubject {
    fn fact(&self, key: &str) -> Option<&str> {
        self.facts.get(key).map(String::as_str)
    }

    fn fact_keys(&self) -> Vec<&str> {
        self.facts.keys().map(String::as_str).collect()
    }
}

/// Exact values that identify one pack call.
pub struct PackRequest<'a> {
    /// Checker selected by its static binding.
    pub checker: &'a str,
    /// Rendered immutable checker-binding id.
    pub binding: &'a str,
    /// Rendered checked-subject id.
    pub subject: &'a str,
    /// Exact authorized scope name.
    pub scope: &'a str,
    /// Already materialized, side-effect-free subject facts.
    pub evidence: &'a dyn PackSubject,
}

/// One deterministic observation included in the result identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CheckObservation {
    /// Stable observation key.
    pub key: String,
    /// Canonical observed value.
    pub value: String,
}

/// A named pack refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackFailure {
    /// Stable machine-facing reason.
    pub code: &'static str,
    /// Bounded human-readable context.
    pub detail: String,
}

/// Pure pack outcome. Only `Pass` is eligible for receipt construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackVerdict {
    /// Every scenario for the exact scope passed.
    Pass {
        /// Canonical result identity.
        result: CheckerResultId,
        /// Sorted observations covered by the result.
        observations: Vec<CheckObservation>,
    },
    /// The binding declares the scope but its scenarios are not funded yet.
    UnimplementedPack {
        /// Exact checker id.
        checker: &'static str,
        /// Requested declared scope.
        scope: String,
        /// Phase that owns the checker implementation.
        funded_phase: String,
    },
    /// Input, scope, binding, or scenario evidence was refused.
    Refused(PackFailure),
}

pub(crate) fn check_registered(
    expected_checker: &'static str,
    request: &PackRequest<'_>,
    implementation: impl FnOnce(&PackRequest<'_>) -> Result<Vec<CheckObservation>, PackFailure>,
) -> PackVerdict {
    if request.checker != expected_checker {
        return refused("wrong-checker", request.checker);
    }
    if let Err(failure) = validate_reference("binding", request.binding) {
        return PackVerdict::Refused(failure);
    }
    if let Err(failure) = validate_reference("subject", request.subject) {
        return PackVerdict::Refused(failure);
    }
    let Some(spec) = find_pack(expected_checker) else {
        return refused("unknown-checker", expected_checker);
    };
    let expected_binding = match spec.checker_binding() {
        Ok(binding) => render(binding.id().content_id()),
        Err(error) => return refused("invalid-static-binding", &error.to_string()),
    };
    if request.binding != expected_binding {
        return refused("wrong-binding", request.binding);
    }
    if !spec.allowed_scopes.contains(&request.scope) {
        return refused("wrong-scope", request.scope);
    }
    if !spec.implemented_scopes.contains(&request.scope) {
        return PackVerdict::UnimplementedPack {
            checker: spec.checker,
            scope: request.scope.into(),
            funded_phase: spec.funded_phase(request.scope),
        };
    }
    let mut observations = match implementation(request) {
        Ok(observations) => observations,
        Err(failure) => return PackVerdict::Refused(failure),
    };
    observations.sort();
    observations.dedup();
    let mut preimage = format!(
        "checker={}\nbinding={}\nsubject={}\nscope={}\n",
        request.checker, request.binding, request.subject, request.scope
    );
    for observation in &observations {
        preimage.push_str(&observation.key);
        preimage.push('=');
        preimage.push_str(&observation.value);
        preimage.push('\n');
    }
    let result = match CheckerResultId::from_fields(vec![(
        Symbol::qualified("conformance", "result"),
        Datum::String(preimage),
    )]) {
        Ok(result) => result,
        Err(error) => return refused("noncanonical-result", &error.to_string()),
    };
    PackVerdict::Pass {
        result,
        observations,
    }
}

pub(crate) fn unavailable(
    expected_checker: &'static str,
    request: &PackRequest<'_>,
) -> PackVerdict {
    check_registered(expected_checker, request, |_| Ok(Vec::new()))
}

pub(crate) fn expect_true(
    request: &PackRequest<'_>,
    key: &'static str,
) -> Result<CheckObservation, PackFailure> {
    expect_eq(request, key, "true")
}

pub(crate) fn expect_eq(
    request: &PackRequest<'_>,
    key: &'static str,
    expected: &str,
) -> Result<CheckObservation, PackFailure> {
    let Some(observed) = request.evidence.fact(key) else {
        return Err(PackFailure {
            code: "missing-evidence",
            detail: key.into(),
        });
    };
    if observed != expected {
        return Err(PackFailure {
            code: "evidence-mismatch",
            detail: format!("{key}: expected {expected:?}, observed {observed:?}"),
        });
    }
    Ok(CheckObservation {
        key: key.into(),
        value: observed.into(),
    })
}

pub(crate) fn expect_same_u64(
    request: &PackRequest<'_>,
    left: &'static str,
    right: &'static str,
) -> Result<Vec<CheckObservation>, PackFailure> {
    let parse = |key: &'static str| {
        request
            .evidence
            .fact(key)
            .ok_or(PackFailure {
                code: "missing-evidence",
                detail: key.into(),
            })?
            .parse::<u64>()
            .map_err(|_| PackFailure {
                code: "malformed-evidence",
                detail: key.into(),
            })
    };
    let left_value = parse(left)?;
    let right_value = parse(right)?;
    if left_value != right_value {
        return Err(PackFailure {
            code: "incomplete-inventory",
            detail: format!("{left}={left_value}, {right}={right_value}"),
        });
    }
    Ok(vec![
        CheckObservation {
            key: left.into(),
            value: left_value.to_string(),
        },
        CheckObservation {
            key: right.into(),
            value: right_value.to_string(),
        },
    ])
}

fn validate_reference(label: &'static str, value: &str) -> Result<(), PackFailure> {
    let Some((algorithm, digest)) = value.rsplit_once(':') else {
        return Err(PackFailure {
            code: "malformed-reference",
            detail: label.into(),
        });
    };
    if !algorithm.contains('/')
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(PackFailure {
            code: "malformed-reference",
            detail: label.into(),
        });
    }
    Ok(())
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn refused(code: &'static str, detail: &str) -> PackVerdict {
    PackVerdict::Refused(PackFailure {
        code,
        detail: detail.chars().take(4_096).collect(),
    })
}
