//! One typed truth source for bidirectional MCP conformance runners.
//!
//! Transport adapters consume these vectors; they do not own parallel golden
//! protocols. The model is deliberately data-only so every runner receives the
//! same authority, timing, cancellation, effects, cache, and redaction facts.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Protocol surface exercised by a vector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransportProjection {
    /// Canonical codec without I/O.
    Codec,
    /// Immutable service dispatch.
    Direct,
    /// Initialize-era compatibility adapter.
    Legacy,
    /// Line-framed process transport.
    Stdio,
    /// Streamable HTTP JSON response.
    HttpJson,
    /// Streamable HTTP SSE response.
    HttpSse,
    /// Complete product process.
    Product,
    /// SDK/runtime projection.
    Runtime,
}

/// One explicit cancellation or fake-clock event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeEvent {
    /// Logical tick; never wall-clock time.
    pub tick: u64,
    /// Event name such as `cancel`, `disconnect`, or `timeout`.
    pub event: String,
}

/// Complete input and expected output for all MCP runners.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceVector {
    /// Stable vector identifier.
    pub id: String,
    /// Visibility profile.
    pub profile: String,
    /// Applicable transport projections.
    pub transports: Vec<TransportProjection>,
    /// Authenticated authority/principal description.
    pub authority: String,
    /// Ordered canonical request messages.
    pub requests: Vec<String>,
    /// Deterministic cancellation and fake-time events.
    pub time_events: Vec<TimeEvent>,
    /// Ordered canonical response messages.
    pub expected_messages: Vec<String>,
    /// Ordered externally visible effects.
    pub expected_effects: Vec<String>,
    /// Ordered cache observations.
    pub expected_cache_activity: Vec<String>,
    /// Secret fragments that must not occur in diagnostics or output.
    pub redaction_assertions: Vec<String>,
    /// Explicit hostile-input/resource limit for this case.
    pub budget: usize,
}

/// Explicit dimensions used to derive metamorphic vectors without ambient state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetamorphicInput {
    /// Principal subject.
    pub principal: String,
    /// Protocol version.
    pub version: String,
    /// Capability set in canonical order.
    pub capabilities: Vec<String>,
    /// Negotiated extension set.
    pub extensions: Vec<String>,
    /// Trace identifier.
    pub trace: String,
    /// Initial cache state.
    pub cache_state: String,
    /// Cancellation tick.
    pub cancellation_tick: u64,
    /// Deterministic transport chunk sizes.
    pub chunks: Vec<usize>,
}

/// Hostile cases that every applicable adapter must reject within `budget`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostileCase {
    /// Recursive or oversized JSON Schema references.
    JsonSchemaRefs,
    /// Header/body projection mismatch.
    HeaderBodyMismatch,
    /// Partial input/output.
    PartialIo,
    /// Peer disconnect.
    Disconnect,
    /// Child process death.
    ChildDeath,
    /// OAuth issuer/resource mix-up.
    OAuthMixUp,
    /// MRTR tamper or replay.
    MrtrTamperReplay,
    /// Extension result validation failure.
    ExtensionResult,
    /// Subscription teardown race.
    SubscriptionTeardown,
    /// Explicit legacy fallback refusal or success.
    LegacyFallback,
}

/// Returns a bounded pairwise set: baseline plus one mutation per dimension.
pub fn metamorphic_inputs(base: &MetamorphicInput) -> Vec<MetamorphicInput> {
    let mut values = vec![base.clone()];
    let mut changed = base.clone();
    changed.principal.push_str("-other");
    values.push(changed);
    let mut changed = base.clone();
    changed.version.push_str("-other");
    values.push(changed);
    let mut changed = base.clone();
    changed.capabilities.reverse();
    values.push(changed);
    let mut changed = base.clone();
    changed.extensions.reverse();
    values.push(changed);
    let mut changed = base.clone();
    changed.trace.push_str("-other");
    values.push(changed);
    let mut changed = base.clone();
    changed.cache_state.push_str("-warm");
    values.push(changed);
    let mut changed = base.clone();
    changed.cancellation_tick = changed.cancellation_tick.saturating_add(1);
    values.push(changed);
    let mut changed = base.clone();
    changed.chunks.reverse();
    values.push(changed);
    values
}

/// Canonical observation returned by every adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorOutcome {
    /// Ordered canonical response messages.
    pub messages: Vec<String>,
    /// Ordered externally visible effects.
    pub effects: Vec<String>,
    /// Ordered cache observations.
    pub cache_activity: Vec<String>,
    /// Redaction-safe diagnostic text.
    pub diagnostics: String,
}

/// Adapter from the one vector authority to a concrete direct or real transport.
pub trait VectorRunner {
    /// Runner identity included in failures.
    fn name(&self) -> &'static str;
    /// Executes one applicable vector.
    fn run(
        &self,
        vector: &ConformanceVector,
        transport: &TransportProjection,
    ) -> Result<VectorOutcome, String>;
}

/// Checks one runner outcome against the vector, including negative redaction assertions.
pub fn check_outcome(vector: &ConformanceVector, outcome: &VectorOutcome) -> Result<(), String> {
    if outcome.messages != vector.expected_messages {
        return Err("message mismatch".into());
    }
    if outcome.effects != vector.expected_effects {
        return Err("effect mismatch".into());
    }
    if outcome.cache_activity != vector.expected_cache_activity {
        return Err("cache mismatch".into());
    }
    if vector
        .redaction_assertions
        .iter()
        .any(|secret| outcome.diagnostics.contains(secret))
    {
        return Err("redaction assertion failed".into());
    }
    Ok(())
}

/// Runs every applicable vector/transport pair through every supplied adapter.
pub fn run_matrix(
    vectors: &[ConformanceVector],
    runners: &[&dyn VectorRunner],
) -> Result<usize, String> {
    let mut count = 0;
    for vector in vectors {
        for transport in &vector.transports {
            for runner in runners {
                let outcome = runner
                    .run(vector, transport)
                    .map_err(|e| format!("{}:{}:{e}", vector.id, runner.name()))?;
                check_outcome(vector, &outcome)
                    .map_err(|e| format!("{}:{}:{e}", vector.id, runner.name()))?;
                count += 1;
            }
        }
    }
    Ok(count)
}
