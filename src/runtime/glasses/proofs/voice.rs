use sim_kernel::{Cx, Expr, Result};
use sim_lib_intent::intent_kind_of;
use sim_lib_stream_device::ModeledSource;
use sim_lib_stream_xr::ModeledHaloMicSource;
use sim_lib_view_device::{ConsentReceipt, EdgeId};
use sim_lib_view_spatial::{AsrSite, XrMicChunkRef, glasses_mic_grant, voice_intent_via_site};
use sim_value::build;

use crate::runtime::glasses::modeled_asr_site_symbol;

/// Result of the consent-gated modeled ASR site proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceSiteProof {
    /// Whether the modeled ASR runtime is registered as a site export.
    pub site_exported: bool,
    /// Whether the visible microphone receipt is bound to this session.
    pub session_bound: bool,
    /// Whether ASR receives audio by reference rather than inline bytes.
    pub by_reference: bool,
    /// Intent kind returned by the site.
    pub intent_kind: String,
}

impl VoiceSiteProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("glasses/sdk", "voice-site-proof")),
            ("site-exported", Expr::Bool(self.site_exported)),
            ("session-bound", Expr::Bool(self.session_bound)),
            ("by-reference", Expr::Bool(self.by_reference)),
            ("intent-kind", build::text(&self.intent_kind)),
        ])
    }
}

/// Turns a canned microphone chunk reference into an Intent through `Export::Site`.
pub fn prove_voice_site(cx: &mut Cx) -> Result<VoiceSiteProof> {
    let source_chunk = ModeledHaloMicSource.at(42);
    let chunk = XrMicChunkRef::new(
        source_chunk.store_key().clone(),
        source_chunk.seq(),
        16_000,
        1,
        u64::from(source_chunk.ms()) * 32,
    )?;
    let site_value = cx
        .registry()
        .site_by_symbol(&modeled_asr_site_symbol())
        .cloned()
        .ok_or_else(|| {
            sim_kernel::Error::HostError("modeled glasses ASR site missing".to_owned())
        })?;
    let fabric = site_value.object().as_eval_fabric().ok_or_else(|| {
        sim_kernel::Error::HostError("modeled glasses ASR export is not a fabric".to_owned())
    })?;
    let site = AsrSite::local(fabric);
    let session = EdgeId::named("sdk-glasses-voice");
    let receipt = ConsentReceipt::new(
        vec![glasses_mic_grant()],
        1_000,
        Vec::new(),
        session.clone(),
        42,
    );
    let intent = voice_intent_via_site(cx, &chunk, Some(&site), &receipt, &session)?;
    let kind = intent_kind_of(&intent)
        .map(|kind| kind.name.to_string())
        .unwrap_or_default();

    Ok(VoiceSiteProof {
        site_exported: true,
        session_bound: receipt.session == session,
        by_reference: matches!(
            sim_value::access::field(&chunk.to_expr(), "ref"),
            Some(Expr::Symbol(_))
        ),
        intent_kind: kind,
    })
}
