use sim_kernel::{Cx, Expr, Ref, Result, Symbol, Value, card::ref_value, force_list_to_vec};

use super::schema::{HELP_FIELDS, HelpBuilder};

pub(super) fn normalize_help_field(
    cx: &mut Cx,
    subject: &Ref,
    spine: &mut [(Symbol, Value)],
) -> Result<()> {
    let Some(help) = find_field(spine, "help").cloned() else {
        return Ok(());
    };
    if has_exact_fields(cx, &help, HELP_FIELDS)? {
        return Ok(());
    }

    let mut builder = HelpBuilder::new(ref_value(cx, subject)?);
    builder.kind = symbol_field(cx, spine, "kind")?.unwrap_or(Symbol::qualified("core", "unknown"));
    builder.summary = help_summary(cx, &help)?;
    builder.detail = String::new();
    builder.capabilities = list_field(cx, spine, "requires")?;
    builder.see_also = list_field(cx, spine, "see-also")?;
    let normalized = builder.build(cx)?;
    replace_field(spine, "help", normalized);
    Ok(())
}

fn has_exact_fields(cx: &mut Cx, value: &Value, fields: &[&str]) -> Result<bool> {
    let expr = value.object().as_expr(cx)?;
    let Expr::Map(entries) = expr else {
        return Ok(false);
    };
    let keys = entries
        .iter()
        .map(|(key, _)| match key {
            Expr::Symbol(symbol) => Some(symbol.clone()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>();
    Ok(keys == Some(fields.iter().map(|field| field_symbol(field)).collect()))
}

fn symbol_field(cx: &mut Cx, entries: &[(Symbol, Value)], name: &str) -> Result<Option<Symbol>> {
    let Some(value) = find_field(entries, name) else {
        return Ok(None);
    };
    Ok(match value.object().as_expr(cx)? {
        Expr::Symbol(symbol) => Some(symbol),
        _ => None,
    })
}

fn list_field(cx: &mut Cx, entries: &[(Symbol, Value)], name: &str) -> Result<Vec<Value>> {
    let Some(value) = find_field(entries, name).cloned() else {
        return Ok(Vec::new());
    };
    let Some(list) = value.object().as_list() else {
        return Ok(Vec::new());
    };
    force_list_to_vec(cx, list, name)
}

fn help_summary(cx: &mut Cx, help: &Value) -> Result<String> {
    Ok(match help.object().as_expr(cx)? {
        Expr::String(text) => text,
        Expr::Map(entries) => string_expr_field(&entries, "summary")
            .or_else(|| string_expr_field(&entries, "purpose"))
            .unwrap_or_default(),
        _ => String::new(),
    })
}

fn string_expr_field(entries: &[(Expr, Expr)], name: &str) -> Option<String> {
    let key = field_symbol(name);
    entries.iter().find_map(|(candidate, value)| {
        let Expr::Symbol(candidate) = candidate else {
            return None;
        };
        if candidate != &key {
            return None;
        }
        match value {
            Expr::String(text) => Some(text.clone()),
            _ => None,
        }
    })
}

use super::fields::{key as field_symbol, value_field as find_field};

fn replace_field(entries: &mut [(Symbol, Value)], name: &str, value: Value) {
    let key = field_symbol(name);
    if let Some((_, slot)) = entries.iter_mut().find(|(field, _)| field == &key) {
        *slot = value;
    }
}
