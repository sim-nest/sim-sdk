use sim_kernel::{
    Cx, Error, EvalFabric, EvalReply, EvalRequest, Expr, Object, ObjectCompat, Result, Symbol,
};
use sim_lib_intent::{Origin, intent};
use sim_lib_view_spatial::{XrMicChunkRef, glasses_mic_capability};
use sim_value::build;

/// Returns the placement symbol for the hardware-free cookbook ASR site.
pub fn modeled_asr_site_symbol() -> Symbol {
    Symbol::qualified("asr/site", "glasses-modeled")
}

pub(super) struct ModeledAsrSite;

impl Object for ModeledAsrSite {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("#<asr-site glasses-modeled>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ObjectCompat for ModeledAsrSite {
    fn as_expr(&self, _cx: &mut Cx) -> Result<Expr> {
        Ok(Expr::Symbol(modeled_asr_site_symbol()))
    }

    fn as_eval_fabric(&self) -> Option<&dyn EvalFabric> {
        Some(self)
    }
}

impl EvalFabric for ModeledAsrSite {
    fn realize(&self, cx: &mut Cx, request: EvalRequest) -> Result<EvalReply> {
        if !request
            .required_capabilities
            .contains(&glasses_mic_capability())
        {
            return Err(Error::CapabilityDenied {
                capability: glasses_mic_capability(),
            });
        }
        cx.require(&glasses_mic_capability())?;
        let chunk = XrMicChunkRef::from_expr(&request.expr)?;
        let response = intent(
            "invoke",
            Origin::agent(chunk.seq),
            vec![
                ("target", build::sym("focused")),
                (
                    "op",
                    Expr::Symbol(Symbol::qualified("glasses/voice", "modeled-asr")),
                ),
                (
                    "args",
                    build::list(vec![
                        Expr::Symbol(chunk.ref_id),
                        build::map(vec![("bytes", build::uint(chunk.byte_len))]),
                    ]),
                ),
            ],
        );
        Ok(EvalReply {
            value: cx.factory().expr(response)?,
            diagnostics: Vec::new(),
            trace: None,
        })
    }
}
