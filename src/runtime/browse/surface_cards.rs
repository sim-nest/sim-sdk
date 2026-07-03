use std::collections::BTreeSet;

use sim_kernel::{Cx, Ref, Result, Symbol, Value};

use super::schema::card_v2_from_card_v1;

const SERVER_SURFACES: &[(&str, &str, &str)] = &[
    ("server", "server", "live evaluation server surface"),
    ("server", "session", "server session surface"),
    ("server", "connection", "server connection surface"),
    ("server", "trigger", "server trigger surface"),
];

const AGENT_SURFACES: &[(&str, &str, &str)] = &[
    ("agent", "tool-surface", "agent callable tool surface"),
    ("agent", "memory", "agent memory surface"),
    ("agent", "model-runner", "agent model runner surface"),
    ("agent", "effects", "agent effect requirement surface"),
];

const STREAM_RANK_STANDARD_SURFACES: &[(&str, &str, &str, &str)] = &[
    ("stream", "stream", "core", "stream"),
    ("stream", "data", "stream", "data"),
    ("stream", "queue-policy", "stream", "queue-policy"),
    ("stream/data", "model-event", "stream", "model-event"),
    ("stream/data", "rank-frontier", "stream", "rank-frontier"),
    ("rank", "space", "rank", "space"),
    ("rank", "coordinate", "rank", "coordinate"),
    ("standard", "profile", "standard", "profile"),
    ("standard", "organ", "standard", "organ"),
];

const FACET_SURFACES: &[(&str, &str, &str)] = &[
    ("server", "metrics", "server metrics facet"),
    ("server", "transport", "server transport facet"),
    ("agent", "tool-surface", "agent tool surface facet"),
    ("agent", "memory", "agent memory facet"),
    ("agent", "model-runner", "agent model runner facet"),
    ("agent", "effects", "agent effect requirements facet"),
    ("codec", "positions", "codec position behavior facet"),
    ("codec", "extensions", "codec extension names facet"),
    (
        "codec",
        "roundtrip-examples",
        "codec round-trip examples facet",
    ),
    ("numbers", "domain", "number domain shape facet"),
    ("numbers", "dispatch", "number dispatch operations facet"),
    ("stream", "events", "stream event surface facet"),
    ("stream", "data", "stream data packet facet"),
    ("stream", "queue-policy", "stream queue policy facet"),
    (
        "stream/data",
        "model-event",
        "model-event data stream compatibility facet",
    ),
    (
        "stream/data",
        "rank-frontier",
        "rank-frontier data stream compatibility facet",
    ),
    ("rank", "space", "rank space facet"),
    ("standard", "fidelity", "standard fidelity facet"),
];

pub(super) fn root_surface_symbols() -> Vec<Symbol> {
    let mut symbols = Vec::new();
    symbols.extend(
        SERVER_SURFACES
            .iter()
            .map(|(namespace, name, _)| Symbol::qualified(*namespace, *name)),
    );
    symbols.extend(
        AGENT_SURFACES
            .iter()
            .map(|(namespace, name, _)| Symbol::qualified(*namespace, *name)),
    );
    symbols.extend(
        STREAM_RANK_STANDARD_SURFACES
            .iter()
            .map(|(namespace, name, _, _)| Symbol::qualified(*namespace, *name)),
    );
    symbols
}

pub(super) fn surface_card_for_symbol(cx: &mut Cx, symbol: &Symbol) -> Result<Option<Value>> {
    let Some(fallback) = surface_fallback(cx, symbol)? else {
        return Ok(None);
    };
    let subject = Ref::Symbol(symbol.clone());
    let card_v1 =
        sim_kernel::card::card_for_ref_with_fallback(cx, subject.clone(), Some(fallback), None)?;
    card_v2_from_card_v1(cx, subject, card_v1).map(Some)
}

fn surface_fallback(cx: &mut Cx, symbol: &Symbol) -> Result<Option<Value>> {
    if cx.registry().codec_by_symbol(symbol).is_some() {
        return codec_fallback(cx, symbol).map(Some);
    }
    if cx.registry().number_domain_by_symbol(symbol).is_some() {
        return number_domain_fallback(cx, symbol).map(Some);
    }
    if let Some((_, name, help)) = server_surface(symbol) {
        return base_fallback(cx, Symbol::qualified("server", *name), help, server_ops()).map(Some);
    }
    if let Some((_, name, help)) = agent_surface(symbol) {
        return base_fallback(
            cx,
            Symbol::qualified("agent", *name),
            help,
            agent_ops(symbol),
        )
        .map(Some);
    }
    if let Some((_, _, kind_ns, kind_name)) = stream_rank_standard_surface(symbol) {
        let help = format!("{symbol} claim, op, shape, and event surface");
        return base_fallback(
            cx,
            Symbol::qualified(*kind_ns, *kind_name),
            &help,
            k6_ops(symbol),
        )
        .map(Some);
    }
    if let Some((_, _, help)) = facet_surface(symbol) {
        return base_fallback(cx, Symbol::qualified("browse", "facet"), help, Vec::new()).map(Some);
    }
    Ok(None)
}

fn codec_fallback(cx: &mut Cx, symbol: &Symbol) -> Result<Value> {
    let mut entries = base_entries(
        cx,
        Symbol::qualified("core", "codec"),
        &format!("codec surface for {symbol}"),
        vec!["core/codec-decode.v1", "core/codec-encode.v1"],
    )?;
    let value = cx.resolve_codec(symbol)?;
    entries.extend(object_table_entries(cx, value)?);
    entries.push((
        field("extensions"),
        string_list(cx, codec_extensions(symbol))?,
    ));
    entries.push((
        field("roundtrip-examples"),
        cx.factory().list(vec![
            cx.factory()
                .symbol(Symbol::qualified("browse-example", "codec-roundtrip"))?,
        ])?,
    ));
    cx.factory().table(entries)
}

fn number_domain_fallback(cx: &mut Cx, symbol: &Symbol) -> Result<Value> {
    let mut entries = base_entries(
        cx,
        Symbol::qualified("core", "number-domain"),
        &format!("number domain surface for {symbol}"),
        vec!["core/number-domain-symbol.v1"],
    )?;
    let value = cx.resolve_number_domain(symbol)?;
    entries.extend(object_table_entries(cx, value)?);
    entries.push((field("dispatch-ops"), dispatch_ops_for_domain(cx, symbol)?));
    cx.factory().table(entries)
}

fn base_fallback(cx: &mut Cx, kind: Symbol, help: &str, ops: Vec<&'static str>) -> Result<Value> {
    let entries = base_entries(cx, kind, help, ops)?;
    cx.factory().table(entries)
}

fn base_entries(
    cx: &mut Cx,
    kind: Symbol,
    help: &str,
    ops: Vec<&'static str>,
) -> Result<Vec<(Symbol, Value)>> {
    Ok(vec![
        (field("kind"), cx.factory().symbol(kind)?),
        (field("help"), cx.factory().string(help.to_owned())?),
        (
            field("args"),
            cx.factory().symbol(Symbol::qualified("core", "Any"))?,
        ),
        (
            field("result"),
            cx.factory().symbol(Symbol::qualified("core", "Any"))?,
        ),
        (field("tests"), cx.factory().list(Vec::new())?),
        (field("ops"), string_list(cx, ops)?),
        (field("requires"), cx.factory().list(Vec::new())?),
        (field("see-also"), cx.factory().list(Vec::new())?),
        (field("shape-known"), cx.factory().bool(false)?),
    ])
}

fn object_table_entries(cx: &mut Cx, value: Value) -> Result<Vec<(Symbol, Value)>> {
    if let Some(table) = value.object().as_table_impl() {
        return table.entries(cx);
    }
    let table = value.object().as_table(cx)?;
    match table.object().as_table_impl() {
        Some(table) => table.entries(cx),
        None => Ok(Vec::new()),
    }
}

fn dispatch_ops_for_domain(cx: &mut Cx, domain: &Symbol) -> Result<Value> {
    let mut symbols = BTreeSet::new();
    for op in cx.registry().number_binary_ops() {
        if &op.left_domain == domain || &op.right_domain == domain {
            symbols.insert(op.operator.clone());
        }
    }
    for op in cx.registry().value_number_binary_ops() {
        if &op.left_domain == domain || &op.right_domain == domain {
            symbols.insert(op.operator.clone());
        }
    }
    for op in cx.registry().number_unary_ops() {
        if &op.operand_domain == domain {
            symbols.insert(op.operator.clone());
        }
    }
    for op in cx.registry().value_number_unary_ops() {
        if &op.operand_domain == domain {
            symbols.insert(op.operator.clone());
        }
    }
    for op in cx.registry().number_reduction_ops() {
        if &op.operand_domain == domain {
            symbols.insert(op.operator.clone());
        }
    }
    for op in cx.registry().value_number_reduction_ops() {
        if &op.operand_domain == domain {
            symbols.insert(op.operator.clone());
        }
    }
    let values = symbols
        .into_iter()
        .map(|symbol| cx.factory().symbol(symbol))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

fn codec_extensions(symbol: &Symbol) -> Vec<&'static str> {
    match (symbol.namespace.as_deref(), symbol.name.as_ref()) {
        (Some("codec"), "lisp") => vec![".siml", ".lisp"],
        (Some("codec"), "json") => vec![".simj", ".json"],
        (Some("codec"), "binary") => vec![".simb"],
        (Some("codec"), "binary-base64") => vec![".simb64"],
        (Some("codec"), "algol") => vec![".sima"],
        (Some("codec"), "chat") => vec![".simchat"],
        _ => Vec::new(),
    }
}

fn server_ops() -> Vec<&'static str> {
    vec![
        "core/realize-start.v1",
        "core/seq-next.v1",
        "core/seq-close.v1",
    ]
}

fn agent_ops(symbol: &Symbol) -> Vec<&'static str> {
    match symbol.name.as_ref() {
        "model-runner" => vec!["ai/infer.v1", "ai/bid.v1"],
        _ => vec!["core/realize-start.v1"],
    }
}

fn k6_ops(symbol: &Symbol) -> Vec<&'static str> {
    match (symbol.namespace.as_deref(), symbol.name.as_ref()) {
        (Some("stream"), "stream") => vec![
            "core/seq-next.v1",
            "core/seq-close.v1",
            "stream/open.v1",
            "stream/clock-convert.v1",
        ],
        (Some("stream"), "data") => vec![
            "stream/filter-kind.v1",
            "stream/filter-shape.v1",
            "stream/map-expr.v1",
            "stream/window.v1",
        ],
        (Some("stream"), "queue-policy") => vec!["stream/open.v1"],
        (Some("stream/data"), "model-event") => vec!["ai/infer.v1", "core/seq-next.v1"],
        (Some("stream/data"), "rank-frontier") => {
            vec!["rank/order-next.v1", "core/seq-next.v1"]
        }
        (Some("rank"), _) => vec![
            "rank/rank.v1",
            "rank/unrank.v1",
            "rank/neighbors.v1",
            "rank/order-next.v1",
        ],
        (Some("standard"), _) => vec!["standard/install.v1", "standard/diff.v1"],
        _ => Vec::new(),
    }
}

fn server_surface(symbol: &Symbol) -> Option<&'static (&'static str, &'static str, &'static str)> {
    SERVER_SURFACES
        .iter()
        .find(|(namespace, name, _)| matches_symbol(symbol, namespace, name))
}

fn agent_surface(symbol: &Symbol) -> Option<&'static (&'static str, &'static str, &'static str)> {
    AGENT_SURFACES
        .iter()
        .find(|(namespace, name, _)| matches_symbol(symbol, namespace, name))
}

fn stream_rank_standard_surface(
    symbol: &Symbol,
) -> Option<&'static (&'static str, &'static str, &'static str, &'static str)> {
    STREAM_RANK_STANDARD_SURFACES
        .iter()
        .find(|(namespace, name, _, _)| matches_symbol(symbol, namespace, name))
}

fn facet_surface(symbol: &Symbol) -> Option<&'static (&'static str, &'static str, &'static str)> {
    FACET_SURFACES
        .iter()
        .find(|(namespace, name, _)| matches_symbol(symbol, namespace, name))
}

fn matches_symbol(symbol: &Symbol, namespace: &str, name: &str) -> bool {
    symbol.namespace.as_deref() == Some(namespace) && symbol.name.as_ref() == name
}

fn string_list(cx: &mut Cx, values: Vec<&str>) -> Result<Value> {
    let values = values
        .into_iter()
        .map(|value| cx.factory().string(value.to_owned()))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

use super::fields::key as field;
