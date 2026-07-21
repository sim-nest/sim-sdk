mod browse;
mod capabilities;
#[cfg(feature = "cookbook")]
pub mod cookbook_directory;
#[cfg(feature = "cookbook")]
mod cookbook_discovery;
mod eval_policy;
mod help;
mod install;
mod lambda;
mod lists;
mod realize;
#[cfg(feature = "device-reference")]
pub mod reference_device;
mod shape_ops;
mod tables;
pub(crate) mod test_runs;
mod testing;
#[cfg(feature = "watch-modeled")]
pub mod watch;

pub use capabilities::{
    browse_internal_capability, browse_read_capability, browse_run_tests_capability,
    config_list_impl_capability, config_table_impl_capability,
};
pub use install::{CoreRuntimeLib, install_core_runtime};
#[cfg(feature = "device")]
pub use reference_device::install_device_base;
pub use testing::{SimTest, TestExpected};
#[cfg(feature = "watch")]
pub use watch::{WatchInstallMode, install_watch_stack};

#[cfg(test)]
mod tests;
