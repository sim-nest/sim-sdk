#[cfg(feature = "agent-runner-core")]
use sim::lib_agent_runner_core as _;
#[cfg(feature = "agent-runner-http")]
use sim::lib_agent_runner_http as _;
#[cfg(feature = "agent-runner-process")]
use sim::lib_agent_runner_process as _;
#[cfg(feature = "discrete")]
use sim::lib_discrete as _;
#[cfg(feature = "view")]
use sim::lib_view as _;
#[cfg(feature = "view-agent")]
use sim::lib_view_agent as _;
#[cfg(feature = "view-bridge")]
use sim::lib_view_bridge as _;
#[cfg(feature = "view-codec")]
use sim::lib_view_codec as _;
#[cfg(feature = "view-daw")]
use sim::lib_view_daw as _;
#[cfg(feature = "view-doc")]
use sim::lib_view_doc as _;
#[cfg(feature = "view-math")]
use sim::lib_view_math as _;
#[cfg(feature = "web-wasm-frame")]
use sim::lib_view_wasm_frame as _;
#[cfg(feature = "web-layout")]
use sim::lib_web_layout as _;

#[test]
fn readme_default_quickstart_compiles_and_installs_core_runtime() {
    use std::sync::Arc;

    use sim::kernel::{Cx, DefaultFactory, EagerPolicy, Symbol};
    use sim::runtime::install_core_runtime;

    let mut cx = Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);

    cx.resolve_shape(&Symbol::qualified("core", "Expr"))
        .expect("README quickstart installs the core shape catalog");
}

#[test]
fn public_feature_aliases_compile() {}
