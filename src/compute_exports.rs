#[cfg(feature = "compute-auto")]
pub use sim_lib_compute_auto as compute_auto;
#[cfg(feature = "compute-cli")]
pub use sim_lib_compute_cli as compute_cli;
#[cfg(feature = "compute-cuda")]
pub use sim_lib_compute_cuda as compute_cuda;
#[cfg(feature = "compute-femm")]
pub use sim_lib_compute_femm as compute_femm;
#[cfg(feature = "compute-model")]
pub use sim_lib_compute_model as compute_model;
#[cfg(feature = "compute-rocm")]
pub use sim_lib_compute_rocm as compute_rocm;
#[cfg(feature = "compute-wgpu")]
pub use sim_lib_compute_wgpu as compute_wgpu;
