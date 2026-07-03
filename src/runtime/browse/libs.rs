use std::sync::Arc;

use sim_kernel::{
    Cx, Demand, Error, ExportRecord, ExportState, Expr, FunctionId, LibManifest, Result, RuntimeId,
    Symbol, Value,
};
use sim_shape::{AnyShape, Bindings, CaptureShape, ListShape};

use crate::functions::{FunctionCase, FunctionObject};

fn one_symbol_arg_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
    implementation: fn(&mut Cx, &sim_kernel::PreparedArgs, Bindings) -> Result<Value>,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "one"),
            args: Arc::new(ListShape::new(vec![Arc::new(CaptureShape::new(
                Symbol::new("subject"),
                Arc::new(AnyShape),
            ))])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![Demand::Value],
            priority: 10,
            implementation,
        }],
    )
}

pub(crate) fn libs_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "all"),
            args: Arc::new(ListShape::new(Vec::new())),
            result: Some(Arc::new(AnyShape)),
            demand: Vec::new(),
            priority: 10,
            implementation: libs_impl,
        }],
    )
}

pub(crate) fn lib_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "one"),
            args: Arc::new(ListShape::new(vec![Arc::new(CaptureShape::new(
                Symbol::new("lib"),
                Arc::new(AnyShape),
            ))])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![Demand::Value],
            priority: 10,
            implementation: lib_impl,
        }],
    )
}

pub(crate) fn exports_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    one_symbol_arg_function(case_id, function_id, symbol, exports_impl)
}

pub(crate) fn export_function(
    case_id: sim_kernel::CaseId,
    function_id: FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    one_symbol_arg_function(case_id, function_id, symbol, export_impl)
}

fn libs_impl(
    cx: &mut Cx,
    _prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let loaded = cx.registry().libs().to_vec();
    let values = loaded
        .iter()
        .map(|loaded| loaded_lib_value(cx, loaded))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

fn lib_impl(
    cx: &mut Cx,
    prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let Some(value) = prepared.get(0) else {
        return Err(Error::Eval("core/lib expects one lib symbol".to_owned()));
    };
    let expr = value.object().as_expr(cx)?;
    let symbol = match expr {
        Expr::Symbol(symbol) => symbol,
        Expr::String(text) => parse_symbol_text(&text),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "symbol",
                found: "non-symbol",
            });
        }
    };
    let manifest = cx.registry().lib(&symbol).cloned();
    match manifest {
        Some(loaded) => loaded_lib_value(cx, &loaded),
        None => cx.factory().nil(),
    }
}

fn exports_impl(
    cx: &mut Cx,
    prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let lib = prepared_symbol(prepared, cx, 0, "core/exports expects one lib symbol")?;
    let Some(loaded) = cx.registry().lib(&lib).cloned() else {
        return cx.factory().nil();
    };
    let values = loaded
        .exports
        .iter()
        .map(|export| export_record_value(cx, &loaded.manifest.id, export))
        .collect::<Result<Vec<_>>>()?;
    cx.factory().list(values)
}

fn export_impl(
    cx: &mut Cx,
    prepared: &sim_kernel::PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let symbol = prepared_symbol(prepared, cx, 0, "core/export expects one export symbol")?;
    let mut matched = None;
    for loaded in cx.registry().libs() {
        for export in &loaded.exports {
            if export.symbol == symbol {
                if matched.is_some() {
                    return Err(Error::Lib(format!(
                        "multiple exports match symbol {symbol}"
                    )));
                }
                matched = Some((loaded.manifest.id.clone(), export.clone()));
            }
        }
    }
    let Some((lib, export)) = matched else {
        return cx.factory().nil();
    };
    export_record_value(cx, &lib, &export)
}

fn prepared_symbol(
    prepared: &sim_kernel::PreparedArgs,
    cx: &mut Cx,
    index: usize,
    message: &str,
) -> Result<Symbol> {
    let Some(value) = prepared.get(index) else {
        return Err(Error::Eval(message.to_owned()));
    };
    let expr = value.object().as_expr(cx)?;
    Ok(match expr {
        Expr::Symbol(symbol) => symbol,
        Expr::String(text) => parse_symbol_text(&text),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "symbol",
                found: "non-symbol",
            });
        }
    })
}

pub(crate) fn loaded_lib_value(cx: &mut Cx, loaded: &sim_kernel::LoadedLib) -> Result<Value> {
    let table = manifest_value(cx, &loaded.manifest, &loaded.exports)?;
    let expr = table.object().as_expr(cx)?;
    let Expr::Map(mut entries) = expr else {
        return Err(Error::HostError(
            "manifest browse object must encode as a table".to_owned(),
        ));
    };
    entries.push((
        Expr::Symbol(Symbol::new("trusted")),
        Expr::Bool(loaded.trusted),
    ));
    entries.push((
        Expr::Symbol(Symbol::new("tests")),
        Expr::List(
            cx.registry()
                .tests_for_lib(&loaded.manifest.id)
                .unwrap_or(&[])
                .iter()
                .cloned()
                .map(Expr::Symbol)
                .collect(),
        ),
    ));
    cx.factory().expr(Expr::Map(entries))
}

fn manifest_value(cx: &mut Cx, manifest: &LibManifest, exports: &[ExportRecord]) -> Result<Value> {
    let export_values = exports
        .iter()
        .map(|export| export_record_value(cx, &manifest.id, export))
        .collect::<Result<Vec<_>>>()?;
    let exports = cx.factory().list(export_values)?;
    cx.factory().table(vec![
        (Symbol::new("id"), cx.factory().symbol(manifest.id.clone())?),
        (
            Symbol::new("version"),
            cx.factory().string(manifest.version.0.clone())?,
        ),
        (
            Symbol::new("abi"),
            cx.factory().table(vec![
                (
                    Symbol::new("major"),
                    cx.factory().number_literal(
                        Symbol::qualified("numbers", "f64"),
                        manifest.abi.major.to_string(),
                    )?,
                ),
                (
                    Symbol::new("minor"),
                    cx.factory().number_literal(
                        Symbol::qualified("numbers", "f64"),
                        manifest.abi.minor.to_string(),
                    )?,
                ),
            ])?,
        ),
        (
            Symbol::new("target"),
            cx.factory().string(lib_target_name(manifest))?,
        ),
        (
            Symbol::new("requires"),
            cx.factory().list(
                manifest
                    .requires
                    .iter()
                    .map(|dependency| {
                        cx.factory().table(vec![
                            (
                                Symbol::new("id"),
                                cx.factory().symbol(dependency.id.clone())?,
                            ),
                            (
                                Symbol::new("minimum-version"),
                                match &dependency.minimum_version {
                                    Some(sim_kernel::Version(version)) => {
                                        cx.factory().string(version.clone())?
                                    }
                                    None => cx.factory().nil()?,
                                },
                            ),
                        ])
                    })
                    .collect::<Result<Vec<_>>>()?,
            )?,
        ),
        (
            Symbol::new("capabilities"),
            cx.factory().list(
                manifest
                    .capabilities
                    .iter()
                    .map(|capability| cx.factory().string(capability.as_str().to_owned()))
                    .collect::<Result<Vec<_>>>()?,
            )?,
        ),
        (Symbol::new("exports"), exports),
    ])
}

fn export_record_value(cx: &mut Cx, lib: &Symbol, export: &ExportRecord) -> Result<Value> {
    let mut entries = vec![
        (Symbol::new("lib"), cx.factory().symbol(lib.clone())?),
        (
            Symbol::new("kind"),
            cx.factory().symbol(export.kind.symbol().clone())?,
        ),
        (
            Symbol::new("symbol"),
            cx.factory().symbol(export.symbol.clone())?,
        ),
    ];
    match &export.state {
        ExportState::Resolved { id } => {
            entries.push((
                Symbol::new("state"),
                cx.factory().symbol(Symbol::new("resolved"))?,
            ));
            entries.push((Symbol::new("runtime-id"), runtime_id_value(cx, *id)?));
        }
        ExportState::Declared => {
            entries.push((
                Symbol::new("state"),
                cx.factory().symbol(Symbol::new("declared"))?,
            ));
        }
        ExportState::Unsupported { reason } => {
            entries.push((
                Symbol::new("state"),
                cx.factory().symbol(Symbol::new("unsupported"))?,
            ));
            entries.push((Symbol::new("reason"), cx.factory().string(reason.clone())?));
        }
        ExportState::Invalid { error } => {
            entries.push((
                Symbol::new("state"),
                cx.factory().symbol(Symbol::new("invalid"))?,
            ));
            entries.push((Symbol::new("error"), cx.factory().string(error.clone())?));
        }
    }
    cx.factory().table(entries)
}

fn runtime_id_value(cx: &mut Cx, id: RuntimeId) -> Result<Value> {
    let (kind, raw_id) = match id {
        RuntimeId::Class(id) => ("class", Some(id.0)),
        RuntimeId::Function(id) => ("function", Some(id.0)),
        RuntimeId::Macro(id) => ("macro", Some(id.0)),
        RuntimeId::Shape(id) => ("shape", Some(id.0)),
        RuntimeId::Codec(id) => ("codec", Some(id.0)),
        RuntimeId::NumberDomain(id) => ("number-domain", Some(id.0)),
        RuntimeId::Value => ("value", None),
        RuntimeId::Site(id) => ("site", Some(id.0)),
    };
    let mut entries = vec![(Symbol::new("kind"), cx.factory().symbol(Symbol::new(kind))?)];
    if let Some(raw_id) = raw_id {
        entries.push((
            Symbol::new("id"),
            cx.factory()
                .number_literal(Symbol::qualified("numbers", "f64"), raw_id.to_string())?,
        ));
    }
    cx.factory().table(entries)
}

fn lib_target_name(manifest: &LibManifest) -> String {
    manifest.target.to_symbol().as_qualified_str()
}

fn parse_symbol_text(value: &str) -> Symbol {
    match value.split_once('/') {
        Some((namespace, name)) if !namespace.is_empty() && !name.is_empty() => {
            Symbol::qualified(namespace.to_owned(), name.to_owned())
        }
        _ => Symbol::new(value.to_owned()),
    }
}
