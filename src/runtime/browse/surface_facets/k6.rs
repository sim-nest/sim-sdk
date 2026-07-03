use sim_kernel::{Cx, Result, Symbol, Value};

use super::{field, public_facet, string_list, symbol_list};

pub(super) fn stream_facets(cx: &mut Cx, symbol: &Symbol) -> Result<Vec<Value>> {
    let mut facets = vec![stream_events_facet(cx)?];
    if is_stream_root(symbol)
        || is_stream_data_surface(symbol)
        || is_model_event_surface(symbol)
        || is_rank_frontier_surface(symbol)
    {
        facets.push(stream_data_facet(cx)?);
    }
    if is_stream_root(symbol)
        || is_stream_data_surface(symbol)
        || is_stream_queue_policy_surface(symbol)
    {
        facets.push(stream_queue_policy_facet(cx)?);
    }
    if is_stream_root(symbol) || is_model_event_surface(symbol) {
        facets.push(model_event_stream_facet(cx)?);
    }
    if is_stream_root(symbol) || is_rank_frontier_surface(symbol) {
        facets.push(rank_frontier_stream_facet(cx)?);
    }
    Ok(facets)
}

pub(super) fn stream_events_facet(cx: &mut Cx) -> Result<Value> {
    let ops = string_list(
        cx,
        vec![
            "core/seq-next.v1",
            "core/seq-close.v1",
            "stream/open.v1",
            "stream/clock-convert.v1",
        ],
    )?;
    let metadata = symbol_list(
        cx,
        vec![
            Symbol::qualified("stream", "clock"),
            Symbol::qualified("stream", "codec"),
            Symbol::qualified("stream", "transport"),
        ],
    )?;
    let events = symbol_list(cx, vec![Symbol::qualified("core", "Event")])?;
    let media = symbol_list(
        cx,
        vec![
            Symbol::qualified("stream/media", "pcm"),
            Symbol::qualified("stream/media", "midi"),
            Symbol::qualified("stream/media", "diagnostic"),
            Symbol::qualified("stream/media", "data"),
        ],
    )?;
    let payload = cx.factory().table(vec![
        (field("ops"), ops),
        (field("metadata-predicates"), metadata),
        (field("events"), events),
        (field("media"), media),
    ])?;
    public_facet(
        cx,
        Symbol::qualified("stream", "events"),
        Symbol::new("state"),
        payload,
        Vec::new(),
    )
}

fn stream_data_facet(cx: &mut Cx) -> Result<Value> {
    let data_kinds = symbol_list(
        cx,
        vec![
            Symbol::qualified("stream/data", "expr"),
            Symbol::qualified("stream/data", "model-event"),
            Symbol::qualified("stream/data", "rank-frontier"),
            Symbol::qualified("stream/data", "window"),
        ],
    )?;
    let fields = symbol_list(
        cx,
        vec![
            Symbol::new("packet"),
            Symbol::new("kind"),
            Symbol::new("payload"),
        ],
    )?;
    let payload = cx.factory().table(vec![
        (
            field("packet-kind"),
            cx.factory()
                .symbol(Symbol::qualified("stream/packet", "data"))?,
        ),
        (
            field("media"),
            cx.factory()
                .symbol(Symbol::qualified("stream/media", "data"))?,
        ),
        (field("fields"), fields),
        (field("data-kinds"), data_kinds),
    ])?;
    public_facet(
        cx,
        Symbol::qualified("stream", "data"),
        Symbol::new("schema"),
        payload,
        Vec::new(),
    )
}

fn stream_queue_policy_facet(cx: &mut Cx) -> Result<Value> {
    let overflow_policies = symbol_list(
        cx,
        vec![
            Symbol::qualified("stream/overflow", "drop-newest"),
            Symbol::qualified("stream/overflow", "drop-oldest"),
            Symbol::qualified("stream/overflow", "error"),
        ],
    )?;
    let payload = cx.factory().table(vec![
        (field("bounded"), cx.factory().bool(true)?),
        (
            field("capacity-field"),
            cx.factory().symbol(Symbol::new("capacity"))?,
        ),
        (
            field("overflow-field"),
            cx.factory().symbol(Symbol::new("overflow"))?,
        ),
        (field("overflow-policies"), overflow_policies),
    ])?;
    public_facet(
        cx,
        Symbol::qualified("stream", "queue-policy"),
        Symbol::new("state"),
        payload,
        Vec::new(),
    )
}

fn model_event_stream_facet(cx: &mut Cx) -> Result<Value> {
    stream_compatibility_facet(
        cx,
        Symbol::qualified("stream/data", "model-event"),
        Symbol::qualified("agent", "model-runner"),
        vec![
            Symbol::new("model-event"),
            Symbol::new("kind"),
            Symbol::new("runner"),
            Symbol::new("model"),
        ],
    )
}

fn rank_frontier_stream_facet(cx: &mut Cx) -> Result<Value> {
    stream_compatibility_facet(
        cx,
        Symbol::qualified("stream/data", "rank-frontier"),
        Symbol::qualified("rank", "space"),
        vec![
            Symbol::new("rank-frontier"),
            Symbol::new("payload-kind"),
            Symbol::new("frontier"),
        ],
    )
}

fn stream_compatibility_facet(
    cx: &mut Cx,
    data_kind: Symbol,
    compatible_surface: Symbol,
    payload_markers: Vec<Symbol>,
) -> Result<Value> {
    let payload_markers = symbol_list(cx, payload_markers)?;
    let payload = cx.factory().table(vec![
        (
            field("packet-kind"),
            cx.factory()
                .symbol(Symbol::qualified("stream/packet", "data"))?,
        ),
        (field("data-kind"), cx.factory().symbol(data_kind.clone())?),
        (
            field("compatible-surface"),
            cx.factory().symbol(compatible_surface)?,
        ),
        (field("payload-markers"), payload_markers),
    ])?;
    public_facet(
        cx,
        data_kind,
        Symbol::new("compatibility"),
        payload,
        Vec::new(),
    )
}

pub(super) fn rank_space_facet(cx: &mut Cx) -> Result<Value> {
    let ops = string_list(
        cx,
        vec![
            "rank/rank.v1",
            "rank/unrank.v1",
            "rank/neighbors.v1",
            "rank/order-next.v1",
        ],
    )?;
    let predicates = symbol_list(
        cx,
        vec![
            Symbol::qualified("rank", "space"),
            Symbol::qualified("rank", "ordinal"),
        ],
    )?;
    let coordinate_kind = cx
        .factory()
        .symbol(Symbol::qualified("rank", "coordinate"))?;
    let payload = cx.factory().table(vec![
        (field("ops"), ops),
        (field("predicates"), predicates),
        (field("coordinate-kind"), coordinate_kind),
    ])?;
    public_facet(
        cx,
        Symbol::qualified("rank", "space"),
        Symbol::new("schema"),
        payload,
        Vec::new(),
    )
}

pub(super) fn standard_fidelity_facet(cx: &mut Cx) -> Result<Value> {
    let ops = string_list(cx, vec!["standard/install.v1", "standard/diff.v1"])?;
    let predicates = symbol_list(
        cx,
        vec![
            Symbol::qualified("standard", "profile"),
            Symbol::qualified("standard", "organ"),
            Symbol::qualified("standard", "fidelity"),
            Symbol::qualified("standard", "evidence"),
        ],
    )?;
    let payload = cx
        .factory()
        .table(vec![(field("ops"), ops), (field("predicates"), predicates)])?;
    public_facet(
        cx,
        Symbol::qualified("standard", "fidelity"),
        Symbol::new("diagnostics"),
        payload,
        Vec::new(),
    )
}

pub(super) fn is_stream_surface(symbol: &Symbol) -> bool {
    is_stream_root(symbol)
        || is_stream_data_surface(symbol)
        || is_stream_queue_policy_surface(symbol)
        || is_model_event_surface(symbol)
        || is_rank_frontier_surface(symbol)
}

pub(super) fn is_rank_surface(symbol: &Symbol) -> bool {
    symbol.namespace.as_deref() == Some("rank")
        && matches!(symbol.name.as_ref(), "space" | "coordinate")
}

pub(super) fn is_standard_surface(symbol: &Symbol) -> bool {
    symbol.namespace.as_deref() == Some("standard")
        && matches!(symbol.name.as_ref(), "profile" | "organ")
}

fn is_stream_root(symbol: &Symbol) -> bool {
    symbol.namespace.as_deref() == Some("stream") && symbol.name.as_ref() == "stream"
}

fn is_stream_data_surface(symbol: &Symbol) -> bool {
    symbol.namespace.as_deref() == Some("stream") && symbol.name.as_ref() == "data"
}

fn is_stream_queue_policy_surface(symbol: &Symbol) -> bool {
    symbol.namespace.as_deref() == Some("stream") && symbol.name.as_ref() == "queue-policy"
}

fn is_model_event_surface(symbol: &Symbol) -> bool {
    symbol.namespace.as_deref() == Some("stream/data") && symbol.name.as_ref() == "model-event"
}

fn is_rank_frontier_surface(symbol: &Symbol) -> bool {
    symbol.namespace.as_deref() == Some("stream/data") && symbol.name.as_ref() == "rank-frontier"
}
