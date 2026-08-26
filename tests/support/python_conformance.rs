//! Shared launcher for checked Python conformance specimens.

use std::process::Command;

pub fn run(script: &str) {
    let status = Command::new("python3")
        .arg(script)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap_or_else(|error| panic!("run {script}: {error}"));
    assert!(status.success(), "{script} exited with {status}");
}
