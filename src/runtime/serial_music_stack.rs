//! Serial-music SDK cookbook bundle and runtime installer.
//!
//! The curated serial facade is intentionally thin. This runtime-side wrapper
//! only gives the cookbook directory one stable loadable row and embeds the
//! SDK-owned serial recipes.

use sim_kernel::{AbiVersion, Cx, Lib, LibManifest, LibTarget, Linker, Result, Symbol, Version};

#[cfg(feature = "cookbook")]
mod cookbook;

#[cfg(feature = "cookbook")]
pub use cookbook::RECIPES;

/// Host-registered cookbook row for the curated serial-music SDK facade.
pub struct SerialMusicCookbookLib;

impl Lib for SerialMusicCookbookLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: serial_music_manifest_symbol(),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: Vec::new(),
            capabilities: Vec::new(),
            exports: Vec::new(),
        }
    }

    fn load(&self, _cx: &mut sim_kernel::LoadCx, _linker: &mut Linker<'_>) -> Result<()> {
        Ok(())
    }
}

/// Installs the serial-music cookbook row into a context.
pub fn install_serial_music_stack(cx: &mut Cx) -> Result<()> {
    sim_lib_core::install_once(cx, &SerialMusicCookbookLib)?;
    Ok(())
}

/// Returns the manifest id for the serial-music SDK cookbook row.
pub fn serial_music_manifest_symbol() -> Symbol {
    Symbol::qualified("sdk", "serial-music")
}
