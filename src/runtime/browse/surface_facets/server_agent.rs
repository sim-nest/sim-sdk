use sim_kernel::{Cx, Result, Symbol, Value, browse_internal_capability};

use super::{
    capability_list, empty_public_facet, field, private_facet, public_facet, string_list,
    symbol_list, symbol_payload,
};

pub(super) fn server_facets(cx: &mut Cx) -> Result<Vec<Value>> {
    let metrics = server_metrics_facet(cx)?;
    let transport_payload = server_transport_payload(cx)?;
    let transport = public_facet(
        cx,
        Symbol::qualified("server", "transport"),
        Symbol::new("transport"),
        transport_payload,
        Vec::new(),
    )?;
    Ok(vec![metrics, transport])
}

pub(super) fn agent_surface_facet(cx: &mut Cx, symbol: &Symbol) -> Result<Value> {
    let (name, kind, payload) = match symbol.name.as_ref() {
        "tool-surface" => (
            Symbol::qualified("agent", "tool-surface"),
            Symbol::new("operations"),
            agent_tools_payload(cx)?,
        ),
        "memory" => (
            Symbol::qualified("agent", "memory"),
            Symbol::new("state"),
            agent_memory_payload(cx)?,
        ),
        "model-runner" => (
            Symbol::qualified("agent", "model-runner"),
            Symbol::new("operations"),
            agent_runner_payload(cx)?,
        ),
        "effects" => (
            Symbol::qualified("agent", "effects"),
            Symbol::new("diagnostics"),
            agent_effect_payload(cx)?,
        ),
        _ => return empty_public_facet(cx, Symbol::qualified("agent", "surface")),
    };
    public_facet(cx, name, kind, payload, Vec::new())
}

pub(super) fn is_server_surface(symbol: &Symbol) -> bool {
    matches!(
        (symbol.namespace.as_deref(), symbol.name.as_ref()),
        (
            Some("server"),
            "server" | "session" | "connection" | "trigger"
        )
    )
}

pub(super) fn is_agent_surface(symbol: &Symbol) -> bool {
    matches!(
        (symbol.namespace.as_deref(), symbol.name.as_ref()),
        (
            Some("agent"),
            "tool-surface" | "memory" | "model-runner" | "effects"
        )
    )
}

fn server_metrics_facet(cx: &mut Cx) -> Result<Value> {
    let payload = cx.factory().table(vec![
        (
            field("status"),
            cx.factory().symbol(Symbol::new("unknown"))?,
        ),
        (field("uptime"), cx.factory().string("unknown".to_owned())?),
        (field("sessions"), cx.factory().string("0".to_owned())?),
        (field("connections"), cx.factory().string("0".to_owned())?),
        (field("messages-sent"), cx.factory().string("0".to_owned())?),
        (
            field("messages-received"),
            cx.factory().string("0".to_owned())?,
        ),
        (field("host-addresses"), cx.factory().list(Vec::new())?),
    ])?;
    private_facet(
        cx,
        Symbol::qualified("server", "metrics"),
        Symbol::new("metrics"),
        payload,
        vec![browse_internal_capability()],
        Vec::new(),
    )
}

fn server_transport_payload(cx: &mut Cx) -> Result<Value> {
    let transports = symbol_list(
        cx,
        vec![
            Symbol::qualified("server", "local"),
            Symbol::qualified("server", "tcp"),
            Symbol::qualified("server", "unix"),
            Symbol::qualified("server", "http"),
            Symbol::qualified("server", "sse"),
            Symbol::qualified("server", "websocket"),
            Symbol::qualified("server", "wasm"),
        ],
    )?;
    let frame_kinds = symbol_list(
        cx,
        vec![
            Symbol::qualified("server", "request"),
            Symbol::qualified("server", "reply"),
            Symbol::qualified("server", "notify"),
            Symbol::qualified("server", "stream"),
            Symbol::qualified("server", "close"),
        ],
    )?;
    let stream_frame_kinds = symbol_list(
        cx,
        vec![
            Symbol::qualified("server", "stream-start"),
            Symbol::qualified("server", "stream-chunk"),
            Symbol::qualified("server", "stream-end"),
        ],
    )?;
    let data_stream_kinds = symbol_list(
        cx,
        vec![
            Symbol::qualified("stream/data", "model-event"),
            Symbol::qualified("stream/data", "rank-frontier"),
        ],
    )?;
    cx.factory().table(vec![
        (field("transports"), transports),
        (field("frame-kinds"), frame_kinds),
        (field("stream-frame-kinds"), stream_frame_kinds),
        (field("data-stream-kinds"), data_stream_kinds),
    ])
}

fn agent_tools_payload(cx: &mut Cx) -> Result<Value> {
    let constructors = symbol_list(
        cx,
        vec![
            Symbol::qualified("agent", "defun"),
            Symbol::qualified("agent", "tools"),
            Symbol::qualified("agent", "call-tool"),
        ],
    )?;
    let capabilities = capability_list(cx, vec!["ai-runner", "ai-runner-local"])?;
    cx.factory().table(vec![
        (field("constructors"), constructors),
        (field("capabilities"), capabilities),
    ])
}

fn agent_memory_payload(cx: &mut Cx) -> Result<Value> {
    symbol_payload(
        cx,
        "constructors",
        vec![
            Symbol::qualified("memory", "working"),
            Symbol::qualified("memory", "file"),
            Symbol::qualified("memory", "vector"),
            Symbol::qualified("memory", "blackboard"),
            Symbol::qualified("memory", "persona"),
        ],
    )
}

fn agent_runner_payload(cx: &mut Cx) -> Result<Value> {
    let constructors = symbol_list(
        cx,
        vec![
            Symbol::qualified("runner", "echo"),
            Symbol::qualified("runner", "cassette"),
            Symbol::qualified("runner", "fake"),
            Symbol::qualified("runner", "agent"),
            Symbol::qualified("runner", "debate"),
            Symbol::qualified("runner", "market"),
        ],
    )?;
    let ops = string_list(cx, vec!["ai/infer.v1", "ai/bid.v1"])?;
    let stream_data_kinds = symbol_list(cx, vec![Symbol::qualified("stream/data", "model-event")])?;
    cx.factory().table(vec![
        (field("constructors"), constructors),
        (field("ops"), ops),
        (field("stream-data-kinds"), stream_data_kinds),
    ])
}

fn agent_effect_payload(cx: &mut Cx) -> Result<Value> {
    let requires = capability_list(
        cx,
        vec![
            "ai-runner",
            "ai-runner-network",
            "ai-runner-local",
            "ai-runner-secret",
            "ai-runner-cache",
            "ai-runner-raw-log",
        ],
    )?;
    let effects = symbol_list(
        cx,
        vec![
            Symbol::qualified("effect", "host-process"),
            Symbol::qualified("effect", "network"),
            Symbol::qualified("effect", "filesystem"),
        ],
    )?;
    cx.factory().table(vec![
        (field("requires"), requires),
        (field("effects"), effects),
    ])
}
