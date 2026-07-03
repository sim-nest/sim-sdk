use sim_kernel::{
    ContentId, Coordinate, Cx, Error, Expr, HandleId, PreparedArgs, Ref, RefResolver, Result,
    Symbol, TemporaryRefResolver,
};

pub(super) fn prepared_ref(
    prepared: &PreparedArgs,
    cx: &mut Cx,
    index: usize,
    message: &str,
) -> Result<Ref> {
    let Some(value) = prepared.get(index).cloned() else {
        return Err(Error::Eval(message.to_owned()));
    };
    let expr = value.object().as_expr(cx)?;
    if let Some(reference) = ref_from_expr(&expr)? {
        return Ok(reference);
    }
    match expr {
        Expr::Symbol(symbol) => Ok(Ref::Symbol(symbol)),
        Expr::String(text) => Ok(Ref::Symbol(parse_symbol_text(&text))),
        _ => {
            let mut resolver = TemporaryRefResolver::new();
            resolver.ref_for_value(cx, &value)
        }
    }
}

pub(super) fn ref_from_expr(expr: &Expr) -> Result<Option<Ref>> {
    let Expr::Extension { tag, payload } = expr else {
        return Ok(None);
    };
    if tag != &Symbol::qualified("core", "ref") {
        return Ok(None);
    }
    let Expr::Map(entries) = payload.as_ref() else {
        return Err(Error::Eval("core/ref payload must be a map".to_owned()));
    };
    let kind = required_symbol(entries, "kind")?;
    if kind == Symbol::qualified("core", "content") {
        return content_id_from_entries(entries).map(|id| Some(Ref::Content(id)));
    }
    if kind == Symbol::qualified("core", "handle") {
        return handle_id_from_entries(entries).map(|handle| Some(Ref::Handle(handle)));
    }
    if kind == Symbol::qualified("core", "coord") {
        return coordinate_from_entries(entries).map(|coord| Some(Ref::Coord(coord)));
    }
    Err(Error::Eval(format!("unknown core/ref kind {kind}")))
}

pub(super) fn parse_symbol_text(value: &str) -> Symbol {
    match value.split_once('/') {
        Some((namespace, name)) if !namespace.is_empty() && !name.is_empty() => {
            Symbol::qualified(namespace.to_owned(), name.to_owned())
        }
        _ => Symbol::new(value.to_owned()),
    }
}

fn content_id_from_entries(entries: &[(Expr, Expr)]) -> Result<ContentId> {
    let algorithm = required_symbol(entries, "algorithm")?;
    let bytes = required_bytes(entries, "bytes", 32)?;
    let mut digest = [0_u8; 32];
    digest.copy_from_slice(bytes);
    Ok(ContentId::from_bytes(algorithm, digest))
}

fn handle_id_from_entries(entries: &[(Expr, Expr)]) -> Result<HandleId> {
    let bytes = required_bytes(entries, "id", 16)?;
    let mut id = [0_u8; 16];
    id.copy_from_slice(bytes);
    Ok(HandleId(u128::from_be_bytes(id)))
}

fn coordinate_from_entries(entries: &[(Expr, Expr)]) -> Result<Coordinate> {
    let space = required_symbol(entries, "space")?;
    let ordinal = required_field(entries, "ordinal")?;
    let Some(Ref::Content(ordinal)) = ref_from_expr(ordinal)? else {
        return Err(Error::Eval(
            "core/ref coord ordinal must be a content ref".to_owned(),
        ));
    };
    Ok(Coordinate { space, ordinal })
}

fn required_symbol(entries: &[(Expr, Expr)], name: &str) -> Result<Symbol> {
    match required_field(entries, name)? {
        Expr::Symbol(symbol) => Ok(symbol.clone()),
        _ => Err(Error::TypeMismatch {
            expected: "symbol",
            found: "non-symbol",
        }),
    }
}

fn required_bytes<'a>(
    entries: &'a [(Expr, Expr)],
    name: &str,
    expected: usize,
) -> Result<&'a [u8]> {
    let bytes = match required_field(entries, name)? {
        Expr::Bytes(bytes) => bytes.as_slice(),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "bytes",
                found: "non-bytes",
            });
        }
    };
    if bytes.len() != expected {
        return Err(Error::Eval(format!(
            "core/ref {name} must contain {expected} bytes"
        )));
    }
    Ok(bytes)
}

fn required_field<'a>(entries: &'a [(Expr, Expr)], name: &str) -> Result<&'a Expr> {
    expr_field(entries, name).ok_or_else(|| Error::Eval(format!("missing core/ref field {name}")))
}

use super::fields::expr_entry_field as expr_field;
