#[cfg(all(feature = "codec-lisp", feature = "shape"))]
use std::sync::Arc;

#[cfg(any(feature = "codec-binary", feature = "codec-lisp"))]
use sim_kernel::{Lib, Result};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReexportKind {
    Class,
    Function,
    Macro,
    Shape,
    Codec,
    NumberDomain,
    Value,
}

/// One re-export entry mapping an exported symbol to a target already present
/// in the registry, tagged by the kind of item it links.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReexportSpec {
    pub(crate) kind: ReexportKind,
    pub(crate) export: sim_kernel::Symbol,
    pub(crate) target: sim_kernel::Symbol,
}

#[cfg(any(
    feature = "codec-binary",
    all(feature = "codec-lisp", not(feature = "shape"))
))]
pub(crate) struct ReexportLib {
    manifest: sim_kernel::LibManifest,
    exports: Vec<ReexportSpec>,
}

#[cfg(any(
    feature = "codec-binary",
    all(feature = "codec-lisp", not(feature = "shape"))
))]
impl ReexportLib {
    pub(crate) fn new(manifest: sim_kernel::LibManifest, exports: Vec<ReexportSpec>) -> Self {
        Self { manifest, exports }
    }
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceMacroSpec {
    pub(crate) symbol: sim_kernel::Symbol,
    pub(crate) fixed_params: Vec<sim_kernel::Symbol>,
    pub(crate) rest_param: Option<sim_kernel::Symbol>,
    pub(crate) template: sim_kernel::Expr,
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
pub(crate) struct SourceLib {
    manifest: sim_kernel::LibManifest,
    exports: Vec<ReexportSpec>,
    macros: Vec<SourceMacroSpec>,
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
impl SourceLib {
    pub(crate) fn new(
        manifest: sim_kernel::LibManifest,
        exports: Vec<ReexportSpec>,
        macros: Vec<SourceMacroSpec>,
    ) -> Self {
        Self {
            manifest,
            exports,
            macros,
        }
    }
}

#[cfg(any(feature = "codec-binary", feature = "codec-lisp"))]
fn link_reexport(linker: &mut sim_kernel::Linker<'_>, export: &ReexportSpec) -> Result<()> {
    match export.kind {
        ReexportKind::Class => {
            let value = linker
                .registry()
                .class_by_symbol(&export.target)
                .cloned()
                .ok_or(sim_kernel::Error::UnknownClass {
                    class: export.target.clone(),
                })?;
            linker.class_value(export.export.clone(), value)?;
        }
        ReexportKind::Function => {
            let value = linker
                .registry()
                .function_by_symbol(&export.target)
                .cloned()
                .ok_or(sim_kernel::Error::UnknownFunction {
                    function: export.target.clone(),
                })?;
            linker.function_value(export.export.clone(), value)?;
        }
        ReexportKind::Macro => {
            let value = linker
                .registry()
                .macro_by_symbol(&export.target)
                .cloned()
                .ok_or(sim_kernel::Error::UnknownSymbol {
                    symbol: export.target.clone(),
                })?;
            linker.macro_value(export.export.clone(), value)?;
        }
        ReexportKind::Shape => {
            let value = linker
                .registry()
                .shape_by_symbol(&export.target)
                .cloned()
                .ok_or(sim_kernel::Error::UnknownSymbol {
                    symbol: export.target.clone(),
                })?;
            linker.shape_value(export.export.clone(), value)?;
        }
        ReexportKind::Codec => {
            let value = linker
                .registry()
                .codec_by_symbol(&export.target)
                .cloned()
                .ok_or(sim_kernel::Error::UnknownSymbol {
                    symbol: export.target.clone(),
                })?;
            linker.codec_value(export.export.clone(), value)?;
        }
        ReexportKind::NumberDomain => {
            let value = linker
                .registry()
                .number_domain_by_symbol(&export.target)
                .cloned()
                .ok_or(sim_kernel::Error::UnknownSymbol {
                    symbol: export.target.clone(),
                })?;
            linker.number_domain_value(export.export.clone(), value)?;
        }
        ReexportKind::Value => {
            let value = linker
                .registry()
                .value_by_symbol(&export.target)
                .cloned()
                .ok_or(sim_kernel::Error::UnknownSymbol {
                    symbol: export.target.clone(),
                })?;
            linker.value(export.export.clone(), value)?;
        }
    }
    Ok(())
}

#[cfg(all(feature = "codec-lisp", feature = "shape"))]
impl Lib for SourceLib {
    fn manifest(&self) -> sim_kernel::LibManifest {
        self.manifest.clone()
    }

    fn load(
        &self,
        _cx: &mut sim_kernel::LoadCx,
        linker: &mut sim_kernel::Linker<'_>,
    ) -> Result<()> {
        for export in &self.exports {
            if matches!(export.kind, ReexportKind::Macro)
                && export.export == export.target
                && self.macros.iter().any(|mac| mac.symbol == export.export)
            {
                continue;
            }
            link_reexport(linker, export)?;
        }

        for mac in &self.macros {
            let syntax_shape = crate::macros::positional_macro_shape(
                mac.symbol.clone(),
                &mac.fixed_params,
                mac.rest_param.as_ref(),
            );
            let value = crate::macros::macro_value_with_parser_trust(
                Arc::new(crate::macros::SourceTemplateMacro::new(
                    mac.symbol.clone(),
                    syntax_shape,
                    mac.template.clone(),
                )),
                false,
            );
            linker.macro_value(mac.symbol.clone(), value)?;
        }

        Ok(())
    }
}

#[cfg(any(
    feature = "codec-binary",
    all(feature = "codec-lisp", not(feature = "shape"))
))]
impl Lib for ReexportLib {
    fn manifest(&self) -> sim_kernel::LibManifest {
        self.manifest.clone()
    }

    fn load(
        &self,
        _cx: &mut sim_kernel::LoadCx,
        linker: &mut sim_kernel::Linker<'_>,
    ) -> Result<()> {
        for export in &self.exports {
            link_reexport(linker, export)?;
        }
        Ok(())
    }
}
