#![forbid(unsafe_code)]

use sim_kernel::{AbiVersion, CapabilityName, Export, LibManifest, LibTarget, Symbol, Version};

/// Order-independent, runtime-id-independent digest of a lib manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ManifestHash(pub [u8; 32]);

/// Digest of a shape's stable identity and contract flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShapeHash(pub [u8; 32]);

/// Digest of a codec's surface contract, independent of its runtime ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CodecHash(pub [u8; 32]);

/// Computes the stable [`ManifestHash`] of a lib manifest, ignoring
/// declaration order and runtime-assigned ids.
pub fn hash_manifest(manifest: &LibManifest) -> ManifestHash {
    let mut hasher = CanonicalHasher::new("sim.manifest.v1");
    hash_manifest_into(&mut hasher, manifest);
    ManifestHash(hasher.finish())
}

/// Computes the stable [`ShapeHash`] of a shape registered under `symbol`.
#[cfg(feature = "shape")]
pub fn hash_shape(symbol: &Symbol, shape: &dyn sim_shape::Shape) -> ShapeHash {
    let mut hasher = CanonicalHasher::new("sim.shape.v1");
    write_symbol(&mut hasher, symbol);
    write_optional_u32(&mut hasher, shape.id().map(|id| id.0));
    write_bool(&mut hasher, shape.is_effectful());
    ShapeHash(hasher.finish())
}

#[cfg(any(
    feature = "codec-lisp",
    feature = "codec-json",
    feature = "codec-binary",
    feature = "codec-binary-base64",
    feature = "codec-chat",
    feature = "codec-algol"
))]
/// Computes the stable [`CodecHash`] of a runtime codec from its surface
/// contract, ignoring runtime-assigned ids.
pub fn hash_codec(
    cx: &mut sim_kernel::Cx,
    codec: &sim_codec::CodecRuntime,
) -> sim_kernel::Result<CodecHash> {
    use sim_kernel::Expr;

    fn shape_symbol(
        cx: &mut sim_kernel::Cx,
        value: &sim_kernel::Value,
    ) -> sim_kernel::Result<Symbol> {
        match value.object().as_expr(cx)? {
            Expr::Symbol(symbol) => Ok(symbol),
            other => Err(sim_kernel::Error::HostError(format!(
                "shape ref did not lower to a symbol: {:?}",
                other
            ))),
        }
    }

    let mut hasher = CanonicalHasher::new("sim.codec.v1");
    write_symbol(&mut hasher, &codec.symbol);
    write_bool(&mut hasher, codec.decoder.is_some());
    write_bool(&mut hasher, codec.located_decoder.is_some());
    write_bool(&mut hasher, codec.tree_decoder.is_some());
    write_bool(&mut hasher, codec.encoder.is_some());
    write_bool(&mut hasher, codec.located_encoder.is_some());
    write_bool(&mut hasher, codec.tree_encoder.is_some());
    write_str(&mut hasher, codec.default_decode.as_symbol_name());
    write_symbol(&mut hasher, &shape_symbol(cx, &codec.expr_shape)?);
    write_symbol(&mut hasher, &shape_symbol(cx, &codec.options_shape)?);
    Ok(CodecHash(hasher.finish()))
}

#[derive(Clone, Copy)]
struct CanonicalHasher {
    state: [u64; 4],
}

impl CanonicalHasher {
    fn new(domain: &'static str) -> Self {
        let mut hasher = Self {
            state: [
                0x243f_6a88_85a3_08d3,
                0x1319_8a2e_0370_7344,
                0xa409_3822_299f_31d0,
                0x082e_fa98_ec4e_6c89,
            ],
        };
        hasher.write_bytes(domain.as_bytes());
        hasher
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.mix_len(bytes.len() as u64);
        for (index, byte) in bytes.iter().enumerate() {
            let lane = index & 3;
            self.state[lane] ^= u64::from(*byte) + ((index as u64) << 8);
            self.state[lane] = self.state[lane]
                .rotate_left(13)
                .wrapping_mul(0x1000_0000_01b3);
            self.state[(lane + 1) & 3] ^= self.state[lane].rotate_right(7);
        }
    }

    fn mix_len(&mut self, len: u64) {
        for lane in 0..4 {
            self.state[lane] ^= len.wrapping_mul((lane as u64) + 0x9e37_79b9);
            self.state[lane] = self.state[lane].rotate_left(17);
        }
    }

    fn finish(mut self) -> [u8; 32] {
        self.mix_len(0xff);
        for lane in 0..4 {
            let other = self.state[(lane + 1) & 3];
            self.state[lane] ^= other.rotate_left(11);
            self.state[lane] = self.state[lane].wrapping_mul(0x9e37_79b1_85eb_ca87);
        }

        let mut out = [0u8; 32];
        for (index, lane) in self.state.into_iter().enumerate() {
            out[index * 8..(index + 1) * 8].copy_from_slice(&lane.to_le_bytes());
        }
        out
    }
}

fn hash_manifest_into(hasher: &mut CanonicalHasher, manifest: &LibManifest) {
    write_symbol(hasher, &manifest.id);
    write_version(hasher, &manifest.version);
    write_abi_version(hasher, manifest.abi);
    write_lib_target(hasher, manifest.target.clone());

    let mut requires = manifest.requires.clone();
    requires.sort_by(|left, right| {
        (
            left.id.namespace.as_ref().map(|value| value.as_ref()),
            left.id.name.as_ref(),
            left.minimum_version.as_ref().map(|value| value.0.as_str()),
        )
            .cmp(&(
                right.id.namespace.as_ref().map(|value| value.as_ref()),
                right.id.name.as_ref(),
                right.minimum_version.as_ref().map(|value| value.0.as_str()),
            ))
    });
    write_usize(hasher, requires.len());
    for dependency in &requires {
        write_symbol(hasher, &dependency.id);
        write_optional_version(hasher, dependency.minimum_version.as_ref());
    }

    let mut capabilities = manifest.capabilities.clone();
    capabilities.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    write_usize(hasher, capabilities.len());
    for capability in &capabilities {
        write_capability(hasher, capability);
    }

    let mut exports = manifest.exports.clone();
    exports.sort_by(|left, right| {
        (left.kind(), left.symbol().as_qualified_str())
            .cmp(&(right.kind(), right.symbol().as_qualified_str()))
    });
    write_usize(hasher, exports.len());
    for export in &exports {
        hash_export_into(hasher, export);
    }
}

fn hash_export_into(hasher: &mut CanonicalHasher, export: &Export) {
    write_str(hasher, export.kind());
    write_symbol(hasher, export.symbol());
}

fn write_lib_target(hasher: &mut CanonicalHasher, target: LibTarget) {
    write_str(hasher, &target.to_symbol().as_qualified_str());
}

fn write_symbol(hasher: &mut CanonicalHasher, symbol: &Symbol) {
    write_optional_str(hasher, symbol.namespace.as_deref());
    write_str(hasher, symbol.name.as_ref());
}

fn write_version(hasher: &mut CanonicalHasher, version: &Version) {
    write_str(hasher, &version.0);
}

fn write_optional_version(hasher: &mut CanonicalHasher, version: Option<&Version>) {
    match version {
        Some(version) => {
            write_bool(hasher, true);
            write_version(hasher, version);
        }
        None => write_bool(hasher, false),
    }
}

fn write_abi_version(hasher: &mut CanonicalHasher, abi: AbiVersion) {
    write_u16(hasher, abi.major);
    write_u16(hasher, abi.minor);
}

fn write_capability(hasher: &mut CanonicalHasher, capability: &CapabilityName) {
    write_str(hasher, capability.as_str());
}

fn write_optional_str(hasher: &mut CanonicalHasher, value: Option<&str>) {
    match value {
        Some(value) => {
            write_bool(hasher, true);
            write_str(hasher, value);
        }
        None => write_bool(hasher, false),
    }
}

#[cfg(feature = "shape")]
fn write_optional_u32(hasher: &mut CanonicalHasher, value: Option<u32>) {
    match value {
        Some(value) => {
            write_bool(hasher, true);
            write_u32(hasher, value);
        }
        None => write_bool(hasher, false),
    }
}

fn write_bool(hasher: &mut CanonicalHasher, value: bool) {
    hasher.write_bytes(&[u8::from(value)]);
}

fn write_u16(hasher: &mut CanonicalHasher, value: u16) {
    hasher.write_bytes(&value.to_le_bytes());
}

#[cfg(feature = "shape")]
fn write_u32(hasher: &mut CanonicalHasher, value: u32) {
    hasher.write_bytes(&value.to_le_bytes());
}

fn write_usize(hasher: &mut CanonicalHasher, value: usize) {
    hasher.write_bytes(&(value as u64).to_le_bytes());
}

fn write_str(hasher: &mut CanonicalHasher, value: &str) {
    hasher.write_bytes(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use sim_kernel::{CapabilityName, Export, LibManifest, LibTarget, Symbol, Version};

    use crate::compat::hash_manifest;

    fn sample_manifest() -> LibManifest {
        LibManifest {
            id: Symbol::qualified("demo", "lib"),
            version: Version("1.2.3".to_owned()),
            abi: sim_kernel::AbiVersion { major: 1, minor: 0 },
            target: LibTarget::CodecSource(Symbol::qualified("codec", "lisp")),
            requires: vec![
                sim_kernel::Dependency {
                    id: Symbol::qualified("z", "later"),
                    minimum_version: Some(Version("2.0.0".to_owned())),
                },
                sim_kernel::Dependency {
                    id: Symbol::qualified("a", "first"),
                    minimum_version: None,
                },
            ],
            capabilities: vec![
                CapabilityName::new("macro.expand"),
                CapabilityName::new("read-construct"),
            ],
            exports: vec![
                Export::Function {
                    symbol: Symbol::qualified("demo", "f"),
                    function_id: Some(sim_kernel::FunctionId(7)),
                },
                Export::Class {
                    symbol: Symbol::qualified("demo", "C"),
                    class_id: Some(sim_kernel::ClassId(222)),
                },
            ],
        }
    }

    #[test]
    fn manifest_hash_ignores_declaration_order_and_runtime_ids() {
        let left = sample_manifest();
        let mut right = sample_manifest();
        right.requires.reverse();
        right.capabilities.reverse();
        right.exports.reverse();
        right.exports = vec![
            Export::Class {
                symbol: Symbol::qualified("demo", "C"),
                class_id: None,
            },
            Export::Function {
                symbol: Symbol::qualified("demo", "f"),
                function_id: None,
            },
        ];

        assert_eq!(hash_manifest(&left), hash_manifest(&right));
    }

    #[test]
    #[cfg(feature = "shape")]
    fn shape_hash_uses_symbol_and_canonical_shape_flags() {
        use std::sync::Arc;

        use crate::compat::hash_shape;
        use crate::shape::{AnyShape, EffectfulShape};

        let plain = AnyShape;
        let effectful = EffectfulShape::new(Arc::new(AnyShape));

        assert_eq!(
            hash_shape(&Symbol::qualified("demo", "shape"), &plain),
            hash_shape(&Symbol::qualified("demo", "shape"), &AnyShape)
        );
        assert_ne!(
            hash_shape(&Symbol::qualified("demo", "shape"), &plain),
            hash_shape(&Symbol::qualified("demo", "shape"), &effectful)
        );
    }

    #[test]
    #[cfg(all(
        feature = "shape",
        any(
            feature = "codec-lisp",
            feature = "codec-json",
            feature = "codec-binary",
            feature = "codec-binary-base64",
            feature = "codec-chat",
            feature = "codec-algol"
        )
    ))]
    fn codec_hash_ignores_runtime_ids_and_uses_surface_contract() {
        use std::sync::Arc;

        use sim_kernel::{DefaultFactory, EagerPolicy};

        use crate::{
            codec::{CodecDefaultDecode, CodecRuntime},
            compat::hash_codec,
            runtime::install_core_runtime,
        };

        let mut cx = sim_kernel::Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
        install_core_runtime(&mut cx);
        let expr_shape = cx
            .registry()
            .shape_by_symbol(&Symbol::qualified("core", "Expr"))
            .cloned()
            .unwrap();
        let options_shape = cx
            .registry()
            .shape_by_symbol(&Symbol::qualified("core", "EncodeOptions"))
            .cloned()
            .unwrap();

        let left = CodecRuntime {
            id: sim_kernel::CodecId(1),
            symbol: Symbol::qualified("codec", "demo"),
            decoder: None,
            located_decoder: None,
            tree_decoder: None,
            encoder: None,
            located_encoder: None,
            tree_encoder: None,
            expr_shape: expr_shape.clone(),
            options_shape: options_shape.clone(),
            default_decode: CodecDefaultDecode::Datum,
        };
        let right = CodecRuntime {
            id: sim_kernel::CodecId(99),
            symbol: Symbol::qualified("codec", "demo"),
            decoder: None,
            located_decoder: None,
            tree_decoder: None,
            encoder: None,
            located_encoder: None,
            tree_encoder: None,
            expr_shape,
            options_shape,
            default_decode: CodecDefaultDecode::Datum,
        };

        assert_eq!(
            hash_codec(&mut cx, &left).unwrap(),
            hash_codec(&mut cx, &right).unwrap()
        );
    }
}
