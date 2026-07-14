use sim_kernel::{
    CapabilityName, Cx, Expr, Ref, Result, Symbol, Value, catalog::registry_catalog_view,
    force_list_to_vec, registry_catalog_read_capability,
};

use super::super::browse_internal_capability;
use super::registry::registry_catalog_subject;
use super::schema::{FacetBuilder, RedactionBuilder};

const SCHEMA_FACET: (&str, &str) = ("browse", "schema");
const EXAMPLES_FACET: (&str, &str) = ("browse", "examples");
const OPERATIONS_FACET: (&str, &str) = ("browse", "operations");

pub(super) fn facets_from_card_spine(
    cx: &mut Cx,
    subject: &Ref,
    spine: &[(Symbol, Value)],
    provenance: Option<Value>,
) -> Result<Vec<Value>> {
    let schema = schema_facet(cx, spine, provenance)?;
    let examples = examples_facet(cx, spine)?;
    let operations = operations_facet(cx, spine)?;
    let mut facets = vec![schema, examples, operations];
    if has_registry_catalog_facet(subject) {
        facets.push(registry_catalog_facet(cx)?);
    }
    Ok(facets)
}

fn schema_facet(
    cx: &mut Cx,
    spine: &[(Symbol, Value)],
    provenance: Option<Value>,
) -> Result<Value> {
    let evidence = optional_list_items(cx, provenance.clone())?;
    let payload = cx.factory().table(vec![
        (field_symbol("args"), required_field(spine, "args")?.clone()),
        (
            field_symbol("result"),
            required_field(spine, "result")?.clone(),
        ),
        (
            field_symbol("shape-known"),
            required_field(spine, "shape-known")?.clone(),
        ),
        (
            field_symbol("evidence"),
            provenance.unwrap_or(cx.factory().list(Vec::new())?),
        ),
    ])?;
    public_facet(
        cx,
        Symbol::qualified(SCHEMA_FACET.0, SCHEMA_FACET.1),
        Symbol::new("schema"),
        payload,
        evidence,
    )
}

fn examples_facet(cx: &mut Cx, spine: &[(Symbol, Value)]) -> Result<Value> {
    let examples = example_refs(cx, required_field(spine, "tests")?.clone())?;
    let payload = cx.factory().list(examples)?;
    public_facet(
        cx,
        Symbol::qualified(EXAMPLES_FACET.0, EXAMPLES_FACET.1),
        Symbol::new("examples"),
        payload,
        Vec::new(),
    )
}

fn operations_facet(cx: &mut Cx, spine: &[(Symbol, Value)]) -> Result<Value> {
    let payload = operation_payload(cx, spine)?;
    private_facet(
        cx,
        Symbol::qualified(OPERATIONS_FACET.0, OPERATIONS_FACET.1),
        Symbol::new("operations"),
        payload,
        vec![browse_internal_capability()],
        Vec::new(),
    )
}

fn registry_catalog_facet(cx: &mut Cx) -> Result<Value> {
    let requires = vec![registry_catalog_read_capability()];
    let value = if cx.capabilities().contains(&requires[0]) {
        registry_catalog_view(cx)?
    } else {
        cx.factory().nil()?
    };
    private_facet(
        cx,
        registry_catalog_subject(),
        Symbol::new("catalog"),
        value,
        requires,
        Vec::new(),
    )
}

fn has_registry_catalog_facet(subject: &Ref) -> bool {
    matches!(
        subject,
        Ref::Symbol(symbol)
            if symbol == &registry_catalog_subject()
                || symbol == &Symbol::qualified("browse", "catalog")
    )
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
    gated_facet(cx, name, kind, value, requires, "private", evidence)
}

fn gated_facet(
    cx: &mut Cx,
    name: Symbol,
    kind: Symbol,
    value: Value,
    requires: Vec<CapabilityName>,
    visibility: &'static str,
    evidence: Vec<Value>,
) -> Result<Value> {
    if requires
        .iter()
        .all(|capability| cx.capabilities().contains(capability))
    {
        return facet(cx, name, kind, value, requires, visibility, evidence);
    }
    let redacted = redaction(cx, &requires)?;
    facet(cx, name, kind, redacted, requires, visibility, evidence)
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

fn operation_payload(cx: &mut Cx, spine: &[(Symbol, Value)]) -> Result<Value> {
    let args = required_field(spine, "args")?.clone();
    let result = required_field(spine, "result")?.clone();
    let requires = required_field(spine, "requires")?.clone();
    let ops = list_items(cx, required_field(spine, "ops")?.clone())?;
    let entries = ops
        .into_iter()
        .map(|op| operation_value(cx, op, args.clone(), result.clone(), requires.clone()))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(entries)
}

fn operation_value(
    cx: &mut Cx,
    key: Value,
    args: Value,
    result: Value,
    requires: Value,
) -> Result<Value> {
    cx.factory().table(vec![
        (field_symbol("key"), key),
        (field_symbol("args"), args),
        (field_symbol("result"), result),
        (field_symbol("effects"), cx.factory().list(Vec::new())?),
        (field_symbol("requires"), requires),
    ])
}

fn example_refs(cx: &mut Cx, tests: Value) -> Result<Vec<Value>> {
    list_items(cx, tests)?
        .into_iter()
        .filter_map(|test| match test.object().as_expr(cx) {
            Ok(Expr::Map(entries)) if bool_field(&entries, "example") == Some(true) => {
                Some(name_value(cx, &entries))
            }
            Ok(_) => None,
            Err(err) => Some(Err(err)),
        })
        .collect()
}

fn name_value(cx: &mut Cx, entries: &[(Expr, Expr)]) -> Result<Value> {
    match expr_field(entries, "name") {
        Some(Expr::Symbol(symbol)) => cx.factory().symbol(symbol.clone()),
        _ => cx.factory().nil(),
    }
}

fn optional_list_items(cx: &mut Cx, value: Option<Value>) -> Result<Vec<Value>> {
    match value {
        Some(value) => list_items(cx, value),
        None => Ok(Vec::new()),
    }
}

fn list_items(cx: &mut Cx, value: Value) -> Result<Vec<Value>> {
    let Some(list) = value.object().as_list() else {
        return Ok(Vec::new());
    };
    force_list_to_vec(cx, list, "browse facet list")
}

fn capability_values(cx: &mut Cx, capabilities: &[CapabilityName]) -> Result<Vec<Value>> {
    capabilities
        .iter()
        .map(|capability| cx.factory().symbol(capability.as_symbol()))
        .collect()
}

fn required_field<'a>(entries: &'a [(Symbol, Value)], name: &str) -> Result<&'a Value> {
    find_field(entries, name)
        .ok_or_else(|| sim_kernel::Error::HostError(format!("missing Card fixed field {name}")))
}

use super::fields::{
    expr_entry_field as expr_field, key as field_symbol, value_field as find_field,
};

fn bool_field(entries: &[(Expr, Expr)], name: &str) -> Option<bool> {
    match expr_field(entries, name) {
        Some(Expr::Bool(value)) => Some(*value),
        _ => None,
    }
}
