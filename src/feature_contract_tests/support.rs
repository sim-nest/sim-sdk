use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn collect_declared_features(cargo_toml: &str) -> BTreeSet<String> {
    let mut features = BTreeSet::new();
    let mut in_features = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_features = trimmed == "[features]";
            continue;
        }
        if !in_features || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            let name = name.trim();
            if name != "default" {
                features.insert(name.to_owned());
            }
        }
    }
    features
}

pub fn collect_feature_dependencies(cargo_toml: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut features = BTreeMap::new();
    let mut in_features = false;
    let mut current: Option<String> = None;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_features = trimmed == "[features]";
            current = None;
            continue;
        }
        if !in_features || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(name) = current.as_ref() {
            extract_quoted_items(trimmed, features.entry(name.clone()).or_default());
            if trimmed.contains(']') {
                current = None;
            }
            continue;
        }
        let Some((name, rhs)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim().to_owned();
        let deps = features.entry(name.clone()).or_default();
        extract_quoted_items(rhs, deps);
        if rhs.contains('[') && !rhs.contains(']') {
            current = Some(name);
        }
    }
    features
}

fn extract_quoted_items(text: &str, out: &mut BTreeSet<String>) {
    let mut rest = text;
    while let Some(start) = rest.find('"') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find('"') else {
            break;
        };
        out.insert(rest[..end].to_owned());
        rest = &rest[end + 1..];
    }
}

pub fn collect_cfg_features(root: &Path) -> BTreeSet<String> {
    let mut features = BTreeSet::new();
    collect_cfg_features_in_dir(&root.join("src"), &mut features);
    collect_cfg_features_in_dir(&root.join("tests"), &mut features);
    features
}

fn collect_cfg_features_in_dir(dir: &Path, features: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_cfg_features_in_dir(&path, features);
            continue;
        }
        let Some(extension) = path.extension() else {
            continue;
        };
        if extension != "rs"
            && path.file_name().and_then(|name| name.to_str()) != Some("Cargo.toml")
        {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let pattern = "feature = \"";
        let mut rest = contents.as_str();
        while let Some(index) = rest.find(pattern) {
            let start = index + pattern.len();
            rest = &rest[start..];
            let Some(end) = rest.find('"') else {
                break;
            };
            features.insert(rest[..end].to_owned());
            rest = &rest[end + 1..];
        }
    }
}

pub fn assert_feature_includes(
    features: &BTreeMap<String, BTreeSet<String>>,
    feature: &str,
    expected: &[&str],
) {
    let actual = features
        .get(feature)
        .unwrap_or_else(|| panic!("missing feature {feature}"));
    let missing = expected
        .iter()
        .filter(|item| !actual.contains(**item))
        .copied()
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "feature {feature} is missing implications: {missing:?}"
    );
}

pub fn assert_crate_cargo_tomls_do_not_contain(root: &Path, prefix: &str, forbidden: &[&str]) {
    let crates_dir = root.join("crates");
    let entries = fs::read_dir(&crates_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", crates_dir.display()));
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with(prefix) {
            continue;
        }
        let cargo_toml = path.join("Cargo.toml");
        let contents = fs::read_to_string(&cargo_toml)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", cargo_toml.display()));
        for forbidden_dep in forbidden {
            assert!(
                !contents.contains(forbidden_dep),
                "{} must not depend on {forbidden_dep}",
                cargo_toml.display()
            );
        }
    }
}
