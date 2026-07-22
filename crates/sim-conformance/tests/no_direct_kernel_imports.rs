use std::{
    fs,
    path::{Path, PathBuf},
};

const ALLOWED_DIRECT_IMPORT_PATH: &str = "conformance_support/mod.rs";

#[test]
fn conformance_specs_use_the_public_facade() {
    let tests_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut offenders = Vec::new();
    collect_offenders(&tests_root, &mut offenders);
    offenders.sort();

    assert!(
        offenders.is_empty(),
        "conformance specs must use sim::kernel and facade aliases; direct sim_lib_* imports are confined to {ALLOWED_DIRECT_IMPORT_PATH}: {offenders:?}"
    );
}

fn collect_offenders(root: &Path, offenders: &mut Vec<String>) {
    for entry in fs::read_dir(root).unwrap_or_else(|err| panic!("read {}: {err}", root.display())) {
        let entry = entry.unwrap_or_else(|err| panic!("read {}: {err}", root.display()));
        let path = entry.path();
        if path.is_dir() {
            collect_offenders(&path, offenders);
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && !is_allowed_direct_import_path(&path)
            && !path.ends_with("no_direct_kernel_imports.rs")
        {
            inspect_file(&path, offenders);
        }
    }
}

fn inspect_file(path: &Path, offenders: &mut Vec<String>) {
    let text =
        fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    for (index, line) in text.lines().enumerate() {
        if contains_direct_import(line) {
            offenders.push(format!("{}:{}", path.display(), index + 1));
        }
    }
}

fn contains_direct_import(line: &str) -> bool {
    line.contains("use sim_kernel")
        || line.contains("sim_kernel::")
        || line.contains("use sim_lib_")
        || line.contains("sim_lib_::")
}

fn is_allowed_direct_import_path(path: &Path) -> bool {
    path.components()
        .rev()
        .take(2)
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        == ["mod.rs", "conformance_support"]
}
