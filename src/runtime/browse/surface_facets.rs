mod codec_domain;
mod k6;
mod server_agent;

use sim_kernel::{
    CapabilityName, ContentId, Cx, Datum, DatumStore, Expr, Ref, Result, Symbol, Value,
    value_from_datum,
};

use super::schema::{FacetBuilder, RedactionBuilder};

pub(super) fn surface_facets(
    cx: &mut Cx,
    subject: &Ref,
    _spine: &[(Symbol, Value)],
) -> Result<Vec<Value>> {
    if let Ref::Content(id) = subject {
        return content_facets(cx, id);
    }
    let Ref::Symbol(symbol) = subject else {
        return Ok(Vec::new());
    };
    let mut facets = Vec::new();
    if server_agent::is_server_surface(symbol) {
        facets.extend(server_agent::server_facets(cx)?);
    }
    if server_agent::is_agent_surface(symbol) {
        facets.push(server_agent::agent_surface_facet(cx, symbol)?);
    }
    if cx.registry().codec_by_symbol(symbol).is_some() {
        facets.extend(codec_domain::codec_facets(cx, symbol)?);
    }
    if cx.registry().number_domain_by_symbol(symbol).is_some() {
        facets.extend(codec_domain::number_domain_facets(cx, symbol)?);
    }
    if k6::is_stream_surface(symbol) {
        facets.extend(k6::stream_facets(cx, symbol)?);
    }
    if k6::is_rank_surface(symbol) {
        facets.push(k6::rank_space_facet(cx)?);
    }
    if k6::is_standard_surface(symbol) {
        facets.push(k6::standard_fidelity_facet(cx)?);
    }
    Ok(facets)
}

fn content_facets(cx: &mut Cx, id: &ContentId) -> Result<Vec<Value>> {
    let facet = match cx.datum_store().get(id)?.cloned() {
        Some(datum) => content_facet(cx, id, datum)?,
        None => missing_content_facet(cx, id)?,
    };
    Ok(vec![facet])
}

fn content_facet(cx: &mut Cx, id: &ContentId, datum: Datum) -> Result<Value> {
    let datum_kind = datum_kind(&datum);
    let value = value_from_datum(cx, datum)?;
    let payload = cx.factory().table(vec![
        (
            field("algorithm"),
            cx.factory().symbol(id.algorithm.clone())?,
        ),
        (field("bytes"), cx.factory().bytes(id.bytes.to_vec())?),
        (field("datum-kind"), cx.factory().symbol(datum_kind)?),
        (field("value"), value),
    ])?;
    public_facet(
        cx,
        Symbol::qualified("browse", "content"),
        Symbol::new("content"),
        payload,
        Vec::new(),
    )
}

fn missing_content_facet(cx: &mut Cx, id: &ContentId) -> Result<Value> {
    let missing_ref = sim_kernel::card::ref_value(cx, &Ref::Content(id.clone()))?;
    let payload = cx.factory().table(vec![
        (
            field("algorithm"),
            cx.factory().symbol(id.algorithm.clone())?,
        ),
        (field("bytes"), cx.factory().bytes(id.bytes.to_vec())?),
        (field("missing-ref"), missing_ref),
    ])?;
    public_facet(
        cx,
        Symbol::qualified("browse", "missing-ref"),
        Symbol::new("missing-ref"),
        payload,
        Vec::new(),
    )
}

fn datum_kind(datum: &Datum) -> Symbol {
    let name = match datum {
        Datum::Nil => "nil",
        Datum::Bool(_) => "bool",
        Datum::Number(_) => "number",
        Datum::Symbol(_) => "symbol",
        Datum::String(_) => "string",
        Datum::Bytes(_) => "bytes",
        Datum::List(_) => "list",
        Datum::Vector(_) => "vector",
        Datum::Map(_) => "map",
        Datum::Set(_) => "set",
        Datum::Node { .. } => "node",
    };
    Symbol::qualified("datum", name)
}

fn public_facet(
    cx: &mut Cx,
    name: Symbol,
    kind: Symbol,
    value: Value,
    evidence: Vec<Value>,
) -> Result<Value> {
    facet(cx, name, kind, value, Vec::new(), "public", evidence)
}

fn private_facet(
    cx: &mut Cx,
    name: Symbol,
    kind: Symbol,
    value: Value,
    requires: Vec<CapabilityName>,
    evidence: Vec<Value>,
) -> Result<Value> {
    if requires
        .iter()
        .all(|capability| cx.capabilities().contains(capability))
    {
        return facet(cx, name, kind, value, requires, "private", evidence);
    }
    let redacted = redaction(cx, &requires)?;
    facet(cx, name, kind, redacted, requires, "private", evidence)
}

fn empty_public_facet(cx: &mut Cx, name: Symbol) -> Result<Value> {
    let value = cx.factory().list(Vec::new())?;
    public_facet(cx, name, Symbol::new("custom"), value, Vec::new())
}

fn facet(
    cx: &mut Cx,
    name: Symbol,
    kind: Symbol,
    value: Value,
    requires: Vec<CapabilityName>,
    visibility: &'static str,
    evidence: Vec<Value>,
) -> Result<Value> {
    let mut builder = FacetBuilder::new(name);
    builder.kind = kind;
    builder.shape = Symbol::qualified("core", "Any");
    builder.value = Some(value);
    builder.requires = capability_values(cx, &requires)?;
    builder.visibility = Symbol::new(visibility);
    builder.evidence = evidence;
    builder.build(cx)
}

fn redaction(cx: &mut Cx, requires: &[CapabilityName]) -> Result<Value> {
    let mut builder = RedactionBuilder::unavailable();
    builder.reason = Symbol::new("capability-required");
    builder.requires = capability_values(cx, requires)?;
    builder.summary = format!(
        "requires {}",
        requires
            .iter()
            .map(CapabilityName::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
    builder.build(cx)
}

fn object_table(cx: &mut Cx, value: Value) -> Result<Vec<(Symbol, Value)>> {
    if let Some(table) = value.object().as_table_impl() {
        return table.entries(cx);
    }
    let table = value.object().as_table(cx)?;
    match table.object().as_table_impl() {
        Some(table) => table.entries(cx),
        None => Ok(Vec::new()),
    }
}

fn string_field(cx: &mut Cx, entries: &[(Symbol, Value)], name: &str) -> Result<Option<String>> {
    let Some(value) = field_value(entries, name) else {
        return Ok(None);
    };
    Ok(match value.object().as_expr(cx)? {
        Expr::String(text) => Some(text),
        _ => None,
    })
}

use super::fields::{key as field, value_field as field_value};

fn symbol_payload(cx: &mut Cx, key: &'static str, symbols: Vec<Symbol>) -> Result<Value> {
    let values = symbol_list(cx, symbols)?;
    cx.factory().table(vec![(field(key), values)])
}

fn symbol_list(cx: &mut Cx, symbols: Vec<Symbol>) -> Result<Value> {
    let values = symbols
        .into_iter()
        .map(|symbol| cx.factory().symbol(symbol))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

fn string_list(cx: &mut Cx, values: Vec<&str>) -> Result<Value> {
    let values = values
        .into_iter()
        .map(|value| cx.factory().string(value.to_owned()))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

fn capability_list(cx: &mut Cx, capabilities: Vec<&'static str>) -> Result<Value> {
    let values = capabilities
        .into_iter()
        .map(|capability| {
            cx.factory()
                .symbol(Symbol::qualified("capability", capability))
        })
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

fn capability_values(cx: &mut Cx, capabilities: &[CapabilityName]) -> Result<Vec<Value>> {
    capabilities
        .iter()
        .map(|capability| cx.factory().symbol(capability.as_symbol()))
        .collect()
}
