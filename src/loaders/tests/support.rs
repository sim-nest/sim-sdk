#[cfg(any(
    all(feature = "codec-lisp", feature = "shape"),
    feature = "codec-binary"
))]
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(all(feature = "codec-lisp", feature = "shape"))]
use std::sync::atomic::{AtomicUsize, Ordering};

use sim_kernel::{
    AbiVersion, Args, CORE_FUNCTION_CLASS_ID, Callable, ClassRef, DefaultFactory, EagerPolicy,
    Export, Lib, LibManifest, LibTarget, Object, Symbol, Value, Version,
};

pub(super) struct StubLib {
    pub(super) symbol: Symbol,
}

pub(super) struct FailingLib;
pub(super) struct CapabilityLib;
pub(super) struct BrokenResolvedExportLib;
pub(super) struct DeclaredOnlyLib;
pub(super) struct ResolvingLib;

pub(super) struct VersionedStubLib {
    pub(super) symbol: Symbol,
    pub(super) version: &'static str,
    pub(super) requires: Vec<sim_kernel::Dependency>,
}

#[derive(Clone)]
pub(super) struct StubCallable;

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
#[derive(Clone)]
pub(super) struct TickCallable {
    pub(super) counter: Arc<AtomicUsize>,
}

impl Object for StubCallable {
    fn display(&self, _cx: &mut sim_kernel::Cx) -> sim_kernel::Result<String> {
        Ok("#<function stub>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for StubCallable {
    fn class(&self, cx: &mut sim_kernel::Cx) -> sim_kernel::Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&sim_kernel::Symbol::qualified("core", "Function"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            CORE_FUNCTION_CLASS_ID,
            sim_kernel::Symbol::qualified("core", "Function"),
        )
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for StubCallable {
    fn call(&self, cx: &mut sim_kernel::Cx, _args: Args) -> sim_kernel::Result<Value> {
        cx.factory().nil()
    }
}

impl Lib for StubLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: self.symbol.clone(),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![Export::Function {
                symbol: Symbol::new("stub"),
                function_id: None,
            }],
        }
    }

    fn load(
        &self,
        cx: &mut sim_kernel::LoadCx,
        linker: &mut sim_kernel::Linker<'_>,
    ) -> sim_kernel::Result<()> {
        let value = cx.factory().opaque(Arc::new(StubCallable))?;
        linker.function_value(Symbol::new("stub"), value)?;
        Ok(())
    }
}

impl Lib for FailingLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: Symbol::new("failing-lib"),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![Export::Function {
                symbol: Symbol::new("half-registered"),
                function_id: None,
            }],
        }
    }

    fn load(
        &self,
        cx: &mut sim_kernel::LoadCx,
        linker: &mut sim_kernel::Linker<'_>,
    ) -> sim_kernel::Result<()> {
        let value = cx.factory().opaque(Arc::new(StubCallable))?;
        linker.function_value(Symbol::new("half-registered"), value)?;
        Err(sim_kernel::Error::HostError("boom".to_owned()))
    }
}

impl Lib for CapabilityLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: Symbol::new("cap-lib"),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: vec![sim_kernel::read_eval_capability()],
            exports: vec![Export::Function {
                symbol: Symbol::new("cap-stub"),
                function_id: None,
            }],
        }
    }

    fn load(
        &self,
        cx: &mut sim_kernel::LoadCx,
        linker: &mut sim_kernel::Linker<'_>,
    ) -> sim_kernel::Result<()> {
        let value = cx.factory().opaque(Arc::new(StubCallable))?;
        linker.function_value(Symbol::new("cap-stub"), value)?;
        Ok(())
    }
}

impl Lib for BrokenResolvedExportLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: Symbol::new("broken-lib"),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![Export::Function {
                symbol: Symbol::new("broken"),
                function_id: None,
            }],
        }
    }

    fn load(
        &self,
        _cx: &mut sim_kernel::LoadCx,
        linker: &mut sim_kernel::Linker<'_>,
    ) -> sim_kernel::Result<()> {
        linker.function(Symbol::new("broken"))?;
        Ok(())
    }
}

impl Lib for DeclaredOnlyLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: Symbol::new("declared-lib"),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![Export::Function {
                symbol: Symbol::new("declared-only"),
                function_id: None,
            }],
        }
    }

    fn load(
        &self,
        _cx: &mut sim_kernel::LoadCx,
        _linker: &mut sim_kernel::Linker<'_>,
    ) -> sim_kernel::Result<()> {
        Ok(())
    }
}

impl Lib for ResolvingLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: Symbol::new("resolving-lib"),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: vec![Export::Function {
                symbol: Symbol::new("resolved-during-load"),
                function_id: None,
            }],
        }
    }

    fn load(
        &self,
        cx: &mut sim_kernel::LoadCx,
        linker: &mut sim_kernel::Linker<'_>,
    ) -> sim_kernel::Result<()> {
        cx.resolve_class(&Symbol::new("already-there"))?;
        let value = cx.factory().opaque(Arc::new(StubCallable))?;
        linker.function_value(Symbol::new("resolved-during-load"), value)?;
        Ok(())
    }
}

impl Lib for VersionedStubLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: self.symbol.clone(),
            version: Version(self.version.to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: self.requires.clone(),
            capabilities: Vec::new(),
            exports: vec![Export::Function {
                symbol: Symbol::qualified(self.symbol.to_string(), "stub"),
                function_id: None,
            }],
        }
    }

    fn load(
        &self,
        cx: &mut sim_kernel::LoadCx,
        linker: &mut sim_kernel::Linker<'_>,
    ) -> sim_kernel::Result<()> {
        let value = cx.factory().opaque(Arc::new(StubCallable))?;
        linker.function_value(Symbol::qualified(self.symbol.to_string(), "stub"), value)?;
        Ok(())
    }
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
impl Object for TickCallable {
    fn display(&self, _cx: &mut sim_kernel::Cx) -> sim_kernel::Result<String> {
        Ok("#<function tick>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
impl sim_kernel::ObjectCompat for TickCallable {
    fn class(&self, cx: &mut sim_kernel::Cx) -> sim_kernel::Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&sim_kernel::Symbol::qualified("core", "Function"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            CORE_FUNCTION_CLASS_ID,
            sim_kernel::Symbol::qualified("core", "Function"),
        )
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
impl Callable for TickCallable {
    fn call(&self, cx: &mut sim_kernel::Cx, _args: Args) -> sim_kernel::Result<Value> {
        let value = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        cx.factory().number_literal(
            sim_kernel::Symbol::qualified("numbers", "f64"),
            value.to_string(),
        )
    }
}

pub(super) fn cx() -> sim_kernel::Cx {
    sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory))
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
pub(super) fn cx_with_lisp_codec() -> sim_kernel::Cx {
    let mut cx = cx();
    crate::runtime::install_core_runtime(&mut cx);
    cx.load_lib(&crate::codec_lisp::LispCodecLib::new(sim_kernel::CodecId(1)).unwrap())
        .unwrap();
    cx
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
pub(super) fn write_source_file(name: &str, source: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("sim-loader-{}-{}.lisp", std::process::id(), name));
    std::fs::write(&path, source).unwrap();
    path
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
pub(super) fn source_fixture() -> &'static str {
    "(sim_lib
        (id \"loader/source-demo\")
        (version \"0.2.0\")
        (export function \"loader/tick\" tick)
        (export shape \"loader/Expr\" \"core/Expr\")
        (export codec \"loader/lisp\" \"codec/lisp\")
        (export \"number-domain\" \"loader/f64\" \"numbers/f64\"))"
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
pub(super) fn source_macro_fixture() -> &'static str {
    "(sim_lib
        (id \"loader/source-macro-demo\")
        (version \"0.2.0\")
        (export macro \"loader/truthy\" truthy))"
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
pub(super) fn source_defmacro_fixture() -> &'static str {
    "(sim_lib
        (id \"loader/source-defmacro-demo\")
        (version \"0.2.0\")
        (defmacro \"loader/when\" (condition &rest body)
          (quasiquote
            (if
              (unquote condition)
              (do (splice body))
              nil))))"
}

#[cfg(feature = "shape")]
pub(super) fn register_truthy_macro(cx: &mut sim_kernel::Cx) {
    let mac = crate::macros::NativeExprMacro::new(
        Symbol::new("truthy"),
        crate::macros::list_macro_shape(Symbol::new("truthy"), Vec::new()),
        |_cx, _input, _bindings| Ok(sim_kernel::Expr::Bool(true)),
    );
    crate::macros::register_macro(cx, Arc::new(mac)).unwrap();
}

#[cfg(feature = "codec-binary")]
pub(super) fn write_binary_pack_file(name: &str, bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("sim-loader-{}-{}.l8b", std::process::id(), name));
    std::fs::write(&path, bytes).unwrap();
    path
}

#[cfg(all(feature = "codec-lisp", feature = "codec-binary"))]
pub(super) fn pack_output_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "sim-loader-{}-{}.out.l8b",
        std::process::id(),
        name
    ))
}
