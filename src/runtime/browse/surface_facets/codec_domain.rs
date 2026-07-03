use sim_kernel::{Cx, Result, Symbol, Value};

use super::{
    field, field_value, object_table, public_facet, string_field, string_list, symbol_list,
};

pub(super) fn codec_facets(cx: &mut Cx, symbol: &Symbol) -> Result<Vec<Value>> {
    let value = cx.resolve_codec(symbol)?;
    let table = object_table(cx, value)?;
    let default_decode =
        string_field(cx, &table, "default-decode")?.unwrap_or_else(|| "datum".to_owned());
    let positions_payload = codec_positions_payload(cx, &default_decode)?;
    let extensions_payload = codec_extensions_payload(cx, symbol)?;
    let roundtrip_payload = codec_roundtrip_payload(cx, symbol)?;
    let positions = public_facet(
        cx,
        Symbol::qualified("codec", "positions"),
        Symbol::new("schema"),
        positions_payload,
        Vec::new(),
    )?;
    let extensions = public_facet(
        cx,
        Symbol::qualified("codec", "extensions"),
        Symbol::new("schema"),
        extensions_payload,
        Vec::new(),
    )?;
    let roundtrip = public_facet(
        cx,
        Symbol::qualified("codec", "roundtrip-examples"),
        Symbol::new("examples"),
        roundtrip_payload,
        Vec::new(),
    )?;
    Ok(vec![positions, extensions, roundtrip])
}

pub(super) fn number_domain_facets(cx: &mut Cx, symbol: &Symbol) -> Result<Vec<Value>> {
    let value = cx.resolve_number_domain(symbol)?;
    let table = object_table(cx, value)?;
    let domain_payload = number_domain_payload(cx, &table)?;
    let dispatch_payload = number_dispatch_payload(cx, symbol)?;
    let domain = public_facet(
        cx,
        Symbol::qualified("numbers", "domain"),
        Symbol::new("schema"),
        domain_payload,
        Vec::new(),
    )?;
    let dispatch = public_facet(
        cx,
        Symbol::qualified("numbers", "dispatch"),
        Symbol::new("operations"),
        dispatch_payload,
        Vec::new(),
    )?;
    Ok(vec![domain, dispatch])
}

fn codec_positions_payload(cx: &mut Cx, default_decode: &str) -> Result<Value> {
    let positions = ["eval", "quote", "data", "pattern"]
        .into_iter()
        .map(|position| {
            let target = if default_decode == "term-in-eval-datum-otherwise" && position == "eval" {
                Symbol::qualified("core", "Term")
            } else {
                Symbol::qualified("core", "Datum")
            };
            cx.factory().table(vec![
                (
                    field("position"),
                    cx.factory().symbol(Symbol::new(position))?,
                ),
                (field("default-target"), cx.factory().symbol(target)?),
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    cx.factory().table(vec![
        (
            field("default-decode"),
            cx.factory().string(default_decode.to_owned())?,
        ),
        (field("positions"), cx.factory().list(positions)?),
    ])
}

fn codec_extensions_payload(cx: &mut Cx, symbol: &Symbol) -> Result<Value> {
    let extensions = string_list(cx, codec_extensions(symbol))?;
    cx.factory().table(vec![(field("extensions"), extensions)])
}

fn codec_roundtrip_payload(cx: &mut Cx, symbol: &Symbol) -> Result<Value> {
    let positions = symbol_list(
        cx,
        vec![
            Symbol::new("eval"),
            Symbol::new("quote"),
            Symbol::new("data"),
            Symbol::new("pattern"),
        ],
    )?;
    let example = cx
        .factory()
        .symbol(Symbol::qualified("browse-example", "codec-roundtrip"))?;
    let entry = cx.factory().table(vec![
        (field("codec"), cx.factory().symbol(symbol.clone())?),
        (field("example"), example),
        (field("positions"), positions),
    ])?;
    cx.factory().list(vec![entry])
}

fn number_domain_payload(cx: &mut Cx, table: &[(Symbol, Value)]) -> Result<Value> {
    let mut entries = Vec::new();
    for name in [
        "numeric-family",
        "canonical-form",
        "parse-priority",
        "literal-class",
        "instance-shape",
        "value-shape",
    ] {
        if let Some(value) = field_value(table, name) {
            entries.push((field(name), value.clone()));
        }
    }
    cx.factory().table(entries)
}

fn number_dispatch_payload(cx: &mut Cx, domain: &Symbol) -> Result<Value> {
    let mut entries = Vec::new();
    let registry = cx.registry().clone();
    for op in registry.number_binary_ops() {
        if &op.left_domain == domain || &op.right_domain == domain {
            entries.push(dispatch_row(
                cx,
                "binary",
                op.operator.clone(),
                vec![
                    ("left-domain", op.left_domain.clone()),
                    ("right-domain", op.right_domain.clone()),
                ],
                op.cost,
            )?);
        }
    }
    for op in registry.value_number_binary_ops() {
        if &op.left_domain == domain || &op.right_domain == domain {
            entries.push(dispatch_row(
                cx,
                "value-binary",
                op.operator.clone(),
                vec![
                    ("left-domain", op.left_domain.clone()),
                    ("right-domain", op.right_domain.clone()),
                ],
                op.cost,
            )?);
        }
    }
    for op in registry.number_unary_ops() {
        if &op.operand_domain == domain {
            entries.push(dispatch_row(
                cx,
                "unary",
                op.operator.clone(),
                vec![("operand-domain", op.operand_domain.clone())],
                op.cost,
            )?);
        }
    }
    for op in registry.value_number_unary_ops() {
        if &op.operand_domain == domain {
            entries.push(dispatch_row(
                cx,
                "value-unary",
                op.operator.clone(),
                vec![("operand-domain", op.operand_domain.clone())],
                op.cost,
            )?);
        }
    }
    for op in registry.number_reduction_ops() {
        if &op.operand_domain == domain {
            entries.push(dispatch_row(
                cx,
                "reduction",
                op.operator.clone(),
                vec![("operand-domain", op.operand_domain.clone())],
                op.cost,
            )?);
        }
    }
    for op in registry.value_number_reduction_ops() {
        if &op.operand_domain == domain {
            entries.push(dispatch_row(
                cx,
                "value-reduction",
                op.operator.clone(),
                vec![("operand-domain", op.operand_domain.clone())],
                op.cost,
            )?);
        }
    }
    cx.factory().list(entries)
}

fn dispatch_row(
    cx: &mut Cx,
    kind: &'static str,
    operator: Symbol,
    domains: Vec<(&'static str, Symbol)>,
    cost: u16,
) -> Result<Value> {
    let mut entries = vec![
        (field("kind"), cx.factory().symbol(Symbol::new(kind))?),
        (field("operator"), cx.factory().symbol(operator)?),
        (
            field("cost"),
            cx.factory()
                .number_literal(Symbol::qualified("numbers", "i64"), cost.to_string())?,
        ),
    ];
    for (name, domain) in domains {
        entries.push((field(name), cx.factory().symbol(domain)?));
    }
    cx.factory().table(entries)
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
