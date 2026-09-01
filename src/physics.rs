#[cfg(feature = "physics-adapter")]
/// Domain-neutral adapter conformance contracts.
pub use sim_lib_physics_adapter as adapter;
#[cfg(feature = "physics-audit")]
/// Stored-energy audit contracts.
pub use sim_lib_physics_audit as audit;
/// Boundary and event topology contracts.
pub use sim_lib_physics_core as core;
#[cfg(feature = "physics-findings")]
/// Immutable finding histories and projections.
pub use sim_lib_physics_findings as findings;
#[cfg(feature = "physics-influence")]
/// No-energy-selection influence contracts.
pub use sim_lib_physics_influence as influence;
#[cfg(feature = "physics-power")]
/// Conjugate-port power and work contracts.
pub use sim_lib_physics_power as power;
#[cfg(feature = "physics-proof")]
/// Refinement and certified-verdict contracts.
pub use sim_lib_physics_proof as proof;
/// Loadable layer-card composition.
pub use sim_lib_physics_runtime as runtime;
#[cfg(feature = "physics-study")]
/// Placement-transparent study contracts.
pub use sim_lib_physics_study as study;
