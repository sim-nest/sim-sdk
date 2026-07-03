mod compile;

#[cfg(all(feature = "codec-lisp", feature = "codec-binary"))]
use sim_kernel::Symbol;
use sim_kernel::{Cx, Lib, LibLoader, LibSource, Result};

#[cfg(not(feature = "shape"))]
use super::reexport::ReexportLib;
#[cfg(feature = "shape")]
use super::reexport::SourceLib;

/// Loader that compiles `.lisp` source files into libs using a Lisp codec.
#[cfg(feature = "codec-lisp")]
pub struct LispSourceLoader {
    codec: sim_kernel::Symbol,
}

#[cfg(feature = "codec-lisp")]
impl Default for LispSourceLoader {
    fn default() -> Self {
        Self::new(sim_kernel::Symbol::qualified("codec", "lisp"))
    }
}

#[cfg(feature = "codec-lisp")]
impl LispSourceLoader {
    /// Creates a loader that decodes source with the codec named `codec`.
    pub fn new(codec: sim_kernel::Symbol) -> Self {
        Self { codec }
    }
}

#[cfg(feature = "codec-lisp")]
impl LibLoader for LispSourceLoader {
    fn can_load(&self, source: &LibSource) -> bool {
        matches!(source, LibSource::Path(path) if path.extension().is_some_and(|ext| ext == "lisp"))
    }

    fn load(&self, cx: &mut Cx, source: LibSource) -> Result<Box<dyn Lib>> {
        let path = match source {
            LibSource::Path(path) => path,
            _ => {
                return Err(sim_kernel::Error::HostError(
                    "lisp source loader received unsupported source".to_owned(),
                ));
            }
        };
        let text = std::fs::read_to_string(&path).map_err(|err| {
            sim_kernel::Error::HostError(format!(
                "failed to read lisp source {}: {err}",
                path.display()
            ))
        })?;
        let expr = sim_codec::decode_with_codec(
            cx,
            &self.codec,
            sim_codec::Input::Text(text),
            sim_kernel::ReadPolicy::default(),
        )?;
        compile_lisp_source_lib(path, expr)
    }

    fn inspect_manifest(
        &self,
        cx: &mut Cx,
        source: &LibSource,
    ) -> Result<Option<sim_kernel::LibManifest>> {
        let path = match source {
            LibSource::Path(path) => path.clone(),
            _ => return Ok(None),
        };
        let text = std::fs::read_to_string(&path).map_err(|err| {
            sim_kernel::Error::HostError(format!(
                "failed to read lisp source {}: {err}",
                path.display()
            ))
        })?;
        let expr = sim_codec::decode_with_codec(
            cx,
            &self.codec,
            sim_codec::Input::Text(text),
            sim_kernel::ReadPolicy::default(),
        )?;
        Ok(Some(
            compile::compile_lisp_source_parts(path, expr)?.manifest,
        ))
    }
}

#[cfg(all(feature = "codec-lisp", feature = "codec-binary"))]
/// Compiles Lisp source text into an in-memory binary lib pack.
pub fn compile_lisp_source_text_to_pack(
    cx: &mut Cx,
    codec: &sim_kernel::Symbol,
    source_path: impl Into<std::path::PathBuf>,
    text: impl Into<String>,
) -> Result<super::binary_pack::BinaryLibPack> {
    let path = source_path.into();
    let expr = sim_codec::decode_with_codec(
        cx,
        codec,
        sim_codec::Input::Text(text.into()),
        sim_kernel::ReadPolicy::default(),
    )?;
    compile_lisp_source_pack(path, expr)
}

#[cfg(all(feature = "codec-lisp", feature = "codec-binary"))]
/// Compiles Lisp source text and encodes it as binary lib pack bytes.
pub fn encode_lisp_source_text_to_binary_pack(
    cx: &mut Cx,
    codec: &Symbol,
    source_path: impl Into<std::path::PathBuf>,
    text: impl Into<String>,
) -> Result<Vec<u8>> {
    let pack = compile_lisp_source_text_to_pack(cx, codec, source_path, text)?;
    super::binary_pack::encode_binary_lib_pack(&pack)
}

#[cfg(all(feature = "codec-lisp", feature = "codec-binary"))]
/// Reads a Lisp source file, compiles it, and writes a binary lib pack to disk.
pub fn export_lisp_source_file_to_binary_pack(
    cx: &mut Cx,
    codec: &Symbol,
    source_path: impl AsRef<std::path::Path>,
    output_path: impl AsRef<std::path::Path>,
) -> Result<()> {
    let source_path = source_path.as_ref();
    let output_path = output_path.as_ref();
    let text = std::fs::read_to_string(source_path).map_err(|err| {
        sim_kernel::Error::HostError(format!(
            "failed to read lisp source {}: {err}",
            source_path.display()
        ))
    })?;
    let bytes = encode_lisp_source_text_to_binary_pack(cx, codec, source_path.to_path_buf(), text)?;
    std::fs::write(output_path, bytes).map_err(|err| {
        sim_kernel::Error::HostError(format!(
            "failed to write binary lib pack {}: {err}",
            output_path.display()
        ))
    })?;
    Ok(())
}

#[cfg(all(feature = "codec-lisp", not(feature = "codec-binary")))]
fn compile_lisp_source_lib(
    path: std::path::PathBuf,
    expr: sim_kernel::Expr,
) -> Result<Box<dyn Lib>> {
    let compiled = compile::compile_lisp_source_parts(path, expr)?;
    #[cfg(feature = "shape")]
    {
        Ok(Box::new(SourceLib::new(
            compiled.manifest,
            compiled.exports,
            compiled.macros,
        )))
    }
    #[cfg(not(feature = "shape"))]
    {
        if !compiled.macros.is_empty() {
            return Err(sim_kernel::Error::Lib(
                "lisp-source defmacro requires the shape feature".to_owned(),
            ));
        }
        Ok(Box::new(ReexportLib::new(
            compiled.manifest,
            compiled.exports,
        )))
    }
}

#[cfg(all(feature = "codec-lisp", feature = "codec-binary"))]
fn compile_lisp_source_lib(
    path: std::path::PathBuf,
    expr: sim_kernel::Expr,
) -> Result<Box<dyn Lib>> {
    let compiled = compile::compile_lisp_source_parts(path, expr)?;
    #[cfg(feature = "shape")]
    {
        Ok(Box::new(SourceLib::new(
            compiled.manifest,
            compiled.exports,
            compiled.macros,
        )))
    }
    #[cfg(not(feature = "shape"))]
    {
        if !compiled.macros.is_empty() {
            return Err(sim_kernel::Error::Lib(
                "lisp-source defmacro requires the shape feature".to_owned(),
            ));
        }
        Ok(Box::new(ReexportLib::new(
            compiled.manifest,
            compiled.exports,
        )))
    }
}

#[cfg(all(feature = "codec-lisp", feature = "codec-binary"))]
/// Compiles an already-decoded Lisp source expression into a binary lib pack.
pub fn compile_lisp_source_pack(
    path: std::path::PathBuf,
    expr: sim_kernel::Expr,
) -> Result<super::binary_pack::BinaryLibPack> {
    let compiled = compile::compile_lisp_source_parts(path, expr)?;
    if !compiled.macros.is_empty() {
        return Err(sim_kernel::Error::Lib(
            "binary lib packs cannot yet encode Lisp-authored defmacro bodies".to_owned(),
        ));
    }
    Ok(super::binary_pack::BinaryLibPack {
        manifest: compiled.manifest,
        exports: compiled.exports,
    })
}
