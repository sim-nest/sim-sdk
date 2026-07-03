use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use sim_kernel::{
    AbiVersion, Args, CORE_FUNCTION_CLASS_ID, Callable, ClassRef, Cx, DefaultFactory, Demand,
    EagerPolicy, Error, ExportKind, Expr, Lib, LibManifest, LibTarget, Linker, Object,
    PreparedArgs, Result, Symbol, Value, Version, eval_fabric_capability,
};
use sim_shape::{AnyShape, Bindings, FunctionCase, FunctionObject, ListShape};

use crate::runtime::install_core_runtime;

pub(super) fn table_value<'a>(expr: &'a Expr, key: &Symbol) -> Option<&'a Expr> {
    let Expr::Map(entries) = expr else {
        return None;
    };
    entries.iter().find_map(|(entry_key, entry_value)| {
        let Expr::Symbol(entry_key) = entry_key else {
            return None;
        };
        (entry_key == key).then_some(entry_value)
    })
}

pub(super) fn eval_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx.grant(eval_fabric_capability());
    cx
}

pub(super) struct UnsupportedExportLib;

#[derive(Clone)]
pub(super) struct TickCallable {
    pub(super) counter: Arc<AtomicUsize>,
}

impl Lib for UnsupportedExportLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: Symbol::qualified("test", "unsupported"),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![sim_kernel::Export::Codec {
                symbol: Symbol::qualified("codec", "future"),
                codec_id: None,
            }],
        }
    }

    fn load(&self, _cx: &mut sim_kernel::LoadCx, linker: &mut Linker) -> sim_kernel::Result<()> {
        linker.unsupported_export(
            ExportKind::named(ExportKind::CODEC),
            Symbol::qualified("codec", "future"),
            "host runtime does not implement codec exports for this lib target",
        )?;
        Ok(())
    }
}

impl Object for TickCallable {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("#<function tick>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for TickCallable {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&Symbol::qualified("core", "Function"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            CORE_FUNCTION_CLASS_ID,
            Symbol::qualified("core", "Function"),
        )
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for TickCallable {
    fn call(&self, cx: &mut Cx, _args: Args) -> Result<Value> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        cx.factory().bool(true)
    }
}

pub(super) fn one_arg_function(
    cx: &mut Cx,
    symbol: Symbol,
    demand: Demand,
    implementation: fn(&mut Cx, &PreparedArgs, Bindings) -> Result<Value>,
) {
    let function = FunctionObject::new(
        cx.registry_mut().fresh_function_id(),
        symbol.clone(),
        vec![FunctionCase {
            id: cx.registry_mut().fresh_case_id(),
            name: Symbol::qualified(symbol.to_string(), "one"),
            args: Arc::new(ListShape::new(vec![Arc::new(AnyShape)])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![demand],
            priority: 10,
            implementation,
        }],
    );
    let value = cx.factory().opaque(Arc::new(function)).unwrap();
    cx.env_mut().define(symbol, value);
}

pub(super) fn two_arg_function(
    cx: &mut Cx,
    symbol: Symbol,
    demands: [Demand; 2],
    implementation: fn(&mut Cx, &PreparedArgs, Bindings) -> Result<Value>,
) {
    let function = FunctionObject::new(
        cx.registry_mut().fresh_function_id(),
        symbol.clone(),
        vec![FunctionCase {
            id: cx.registry_mut().fresh_case_id(),
            name: Symbol::qualified(symbol.to_string(), "two"),
            args: Arc::new(ListShape::new(vec![Arc::new(AnyShape), Arc::new(AnyShape)])),
            result: Some(Arc::new(AnyShape)),
            demand: demands.to_vec(),
            priority: 10,
            implementation,
        }],
    );
    let value = cx.factory().opaque(Arc::new(function)).unwrap();
    cx.env_mut().define(symbol, value);
}

pub(super) fn ignore_arg_impl(
    cx: &mut Cx,
    _prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    cx.factory().bool(true)
}

pub(super) fn return_first_arg_impl(
    _cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    prepared
        .get(0)
        .cloned()
        .ok_or_else(|| Error::Eval("missing prepared arg 0".to_owned()))
}

pub(super) fn force_first_arg_twice_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: Bindings,
) -> Result<Value> {
    let value = prepared
        .get(0)
        .cloned()
        .ok_or_else(|| Error::Eval("missing prepared arg 0".to_owned()))?;
    let _ = cx.force(value.clone(), Demand::Value)?;
    cx.force(value, Demand::Value)
}

pub(super) fn call_expr(operator: Symbol, args: Vec<Expr>) -> Expr {
    Expr::Call {
        operator: Box::new(Expr::Symbol(operator)),
        args,
    }
}
