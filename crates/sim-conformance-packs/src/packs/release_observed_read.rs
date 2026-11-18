// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Crate, tag, mirror, pin, and checkout reads for one published obligation.

use std::collections::BTreeSet;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::PackFailure;
use crate::packs::completion_claim::execute_process;

pub(super) struct ParsedManifest {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) deps: BTreeSet<String>,
}

pub(super) struct ReadPublished {
    pub(super) parsed: ParsedManifest,
    pub(super) observed: super::ObservedPublished,
}

pub(super) fn read_published(
    package: &super::PublishedObligation,
) -> Result<ReadPublished, PackFailure> {
    let bytes = read_limited(&package.krate, super::MAX_CRATE, "crate")?;
    let checksum = super::hex(&Sha256::digest(&bytes));
    let index = std::fs::read_to_string(&package.index)
        .map_err(|_| super::fail("missing-source", "index"))?;
    if index.len() > super::MAX_INDEX || index.contains('\\') {
        return Err(super::fail("bound", "index"));
    }
    let (name, version) = match_index(&index, &checksum)?;
    let crate_manifest = tar_member(&package.krate, &format!("{name}-{version}/Cargo.toml"))?;
    let vcs = tar_member(
        &package.krate,
        &format!("{name}-{version}/.cargo_vcs_info.json"),
    )?;
    let parsed = parse_manifest(&crate_manifest, true)?;
    if parsed.name != name || parsed.version != version {
        return Err(super::fail("evidence-mismatch", "manifest"));
    }
    let release = json_string(&vcs, "sha1")?.to_owned();
    if !super::is_rev(&release) {
        return Err(super::fail("bad-record", "sha1"));
    }
    let path_in_vcs = optional_string(&vcs, "path_in_vcs")?;
    let tagged = git_show(
        &package.repository,
        &release,
        &manifest_in_vcs(&path_in_vcs)?,
    )?;
    let tagged = parse_manifest(&tagged, true)?;
    if tagged.name != parsed.name || tagged.version != parsed.version || tagged.deps != parsed.deps
    {
        return Err(super::fail("evidence-mismatch", "tag-manifest"));
    }
    let tag = format!("refs/tags/{name}-v{version}^{{commit}}");
    let tagged_commit = git_rev(&package.repository, &tag)?;
    if tagged_commit != release {
        return Err(super::fail("tag-mismatch", &name));
    }
    let selected = pin_commit(&package.pin_manifest, &package.pin_name)?;
    let head = git_rev(&package.checkout, "HEAD^{commit}")?;
    if head != selected {
        return Err(super::fail("checkout-mismatch", &package.pin_name));
    }
    let left = remote_arg(&package.mirror_left)?;
    let right = remote_arg(&package.mirror_right)?;
    if left == right {
        return Err(super::fail("mirror-same", &name));
    }
    if git_remote(&left)? != selected || git_remote(&right)? != selected {
        return Err(super::fail("mirror-mismatch", &name));
    }
    release_contained(&package.checkout, &release, &selected)?;
    Ok(ReadPublished {
        parsed,
        observed: super::ObservedPublished {
            name,
            version,
            checksum,
            commit: release,
            selected,
        },
    })
}

fn match_index(index: &str, checksum: &str) -> Result<(String, String), PackFailure> {
    let mut found = None;
    for line in index.lines() {
        if line.trim().is_empty() || !line.contains("\"cksum\"") {
            continue;
        }
        let cksum = json_string(line, "cksum")?;
        if cksum != checksum {
            continue;
        }
        if found.is_some() {
            return Err(super::fail("duplicate-crate", "index"));
        }
        if json_word(line, "yanked")? != "false" {
            return Err(super::fail("yanked", cksum));
        }
        let name = json_string(line, "name")?.to_owned();
        let version = json_string(line, "vers")?.to_owned();
        if !super::is_name(&name) || !super::is_version(&version) {
            return Err(super::fail("bad-record", &name));
        }
        found = Some((name, version));
    }
    found.ok_or_else(|| super::fail("checksum-mismatch", "index"))
}

pub(super) fn parse_manifest(blob: &str, published: bool) -> Result<ParsedManifest, PackFailure> {
    if blob.len() > 65_536 || blob.contains('\0') {
        return Err(super::fail("bound", "manifest"));
    }
    let root: toml::Value =
        toml::from_str(blob).map_err(|_| super::fail("bad-record", "manifest"))?;
    let package = root
        .get("package")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| super::fail("missing-source", "package"))?;
    match package.get("publish") {
        Some(toml::Value::Boolean(false)) if published => {
            return Err(super::fail("publish-flag", "false"));
        }
        Some(toml::Value::Boolean(true)) | Some(toml::Value::Boolean(false)) | None => {}
        Some(_) => return Err(super::fail("bad-record", "publish")),
    }
    let name = require_string(package, "name")?.to_owned();
    let version = require_string(package, "version")?.to_owned();
    if !super::is_name(&name) || !super::is_version(&version) {
        return Err(super::fail("bad-record", &name));
    }
    let mut deps = BTreeSet::new();
    if let Some(table) = root.get("dependencies").and_then(toml::Value::as_table) {
        for (key, value) in table {
            let dep = match value.as_table().and_then(|table| table.get("package")) {
                Some(toml::Value::String(package)) => package.clone(),
                _ => key.clone(),
            };
            if !super::is_name(&dep) {
                return Err(super::fail("bad-record", &dep));
            }
            deps.insert(dep);
        }
    }
    Ok(ParsedManifest {
        name,
        version,
        deps,
    })
}

fn pin_commit(manifest: &Path, name: &str) -> Result<String, PackFailure> {
    super::token(name, "pin")?;
    let text =
        std::fs::read_to_string(manifest).map_err(|_| super::fail("missing-source", "pin"))?;
    if text.len() > super::MAX_INDEX {
        return Err(super::fail("bound", "pin"));
    }
    let root: toml::Value = toml::from_str(&text).map_err(|_| super::fail("bad-record", "pin"))?;
    let repos = root
        .get("repo")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| super::fail("missing-source", "pin"))?;
    let mut found = None;
    for repo in repos {
        let table = repo
            .as_table()
            .ok_or_else(|| super::fail("bad-record", "pin"))?;
        if table.get("name").and_then(toml::Value::as_str) != Some(name) {
            continue;
        }
        if found.is_some() {
            return Err(super::fail("duplicate-crate", name));
        }
        let commit = require_string(table, "commit")?;
        if !super::is_rev(commit) {
            return Err(super::fail("bad-record", "commit"));
        }
        found = Some(commit.to_owned());
    }
    found.ok_or_else(|| super::fail("pin-mismatch", name))
}

pub(super) fn git_show(
    repository: &Path,
    commit: &str,
    manifest: &str,
) -> Result<String, PackFailure> {
    if !super::is_rev(commit) {
        return Err(super::fail("bad-record", "commit"));
    }
    let repo = canonical_dir(repository, "repository")?;
    let spec = format!("{commit}:{manifest}");
    let shown = execute_process("git", &["--no-replace-objects", "-C", &repo, "show", &spec])?;
    String::from_utf8(shown.stdout).map_err(|_| super::fail("bad-record", "manifest"))
}

fn git_rev(repository: &Path, spec: &str) -> Result<String, PackFailure> {
    let repo = canonical_dir(repository, "repository")?;
    let shown = execute_process(
        "git",
        &[
            "--no-replace-objects",
            "-C",
            &repo,
            "rev-parse",
            "--verify",
            spec,
        ],
    )?;
    one_rev(&shown.stdout)
}

fn git_remote(url: &str) -> Result<String, PackFailure> {
    let shown = execute_process(
        "git",
        &["--no-replace-objects", "ls-remote", url, "refs/heads/main"],
    )?;
    one_rev(&shown.stdout)
}

fn release_contained(checkout: &Path, release: &str, selected: &str) -> Result<(), PackFailure> {
    if !super::is_rev(release) || !super::is_rev(selected) {
        return Err(super::fail("bad-record", "revision"));
    }
    let repo = canonical_dir(checkout, "checkout")?;
    let output = std::process::Command::new("git")
        .args([
            "--no-replace-objects",
            "-C",
            &repo,
            "merge-base",
            "--is-ancestor",
            release,
            selected,
        ])
        .output()
        .map_err(|_| super::fail("process-unrun", "git"))?;
    if output.stdout.len() > super::MAX_INDEX || output.stderr.len() > super::MAX_INDEX {
        return Err(super::fail("process-output", "git"));
    }
    match output.status.code() {
        Some(0) => Ok(()),
        Some(1) => Err(super::fail("release-uncontained", release)),
        Some(status) => Err(super::fail("process-failed", &status.to_string())),
        None => Err(super::fail("process-unrun", "signal")),
    }
}

fn tar_member(krate: &Path, member: &str) -> Result<String, PackFailure> {
    let path = canonical_file(krate, "crate")?;
    let shown = execute_process("tar", &["-xOzf", &path, member])?;
    String::from_utf8(shown.stdout).map_err(|_| super::fail("bad-record", member))
}

fn manifest_in_vcs(path_in_vcs: &str) -> Result<String, PackFailure> {
    if path_in_vcs.is_empty() {
        return Ok("Cargo.toml".to_owned());
    }
    if path_in_vcs.starts_with('/') || path_in_vcs.contains("..") {
        return Err(super::fail("bad-record", "path_in_vcs"));
    }
    Ok(format!("{path_in_vcs}/Cargo.toml"))
}

fn optional_string(text: &str, key: &str) -> Result<String, PackFailure> {
    let needle = format!("\"{key}\"");
    if !text.contains(&needle) {
        return Ok(String::new());
    }
    Ok(json_string(text, key)?.to_owned())
}

fn json_string<'a>(line: &'a str, key: &str) -> Result<&'a str, PackFailure> {
    let rest = after_key(line, key)?;
    let rest = rest
        .strip_prefix('"')
        .ok_or_else(|| super::fail("bad-record", key))?;
    let end = rest
        .find('"')
        .ok_or_else(|| super::fail("bad-record", key))?;
    Ok(&rest[..end])
}

fn json_word<'a>(line: &'a str, key: &str) -> Result<&'a str, PackFailure> {
    let rest = after_key(line, key)?;
    if rest.starts_with('"') {
        return Err(super::fail("bad-record", key));
    }
    let end = rest
        .find(|symbol: char| symbol == ',' || symbol == '}' || symbol.is_whitespace())
        .unwrap_or(rest.len());
    if end == 0 {
        return Err(super::fail("bad-record", key));
    }
    Ok(&rest[..end])
}

fn after_key<'a>(line: &'a str, key: &str) -> Result<&'a str, PackFailure> {
    let needle = format!("\"{key}\"");
    let start = line
        .find(&needle)
        .ok_or_else(|| super::fail("bad-record", key))?;
    let rest = line[start + needle.len()..].trim_start();
    rest.strip_prefix(':')
        .ok_or_else(|| super::fail("bad-record", key))
        .map(str::trim_start)
}

fn one_rev(stdout: &[u8]) -> Result<String, PackFailure> {
    let text = std::str::from_utf8(stdout).map_err(|_| super::fail("bad-record", "rev"))?;
    let mut lines = text.lines().filter(|line| !line.is_empty());
    let Some(line) = lines.next() else {
        return Err(super::fail("missing-source", "rev"));
    };
    if lines.next().is_some() {
        return Err(super::fail("evidence-mismatch", "rev"));
    }
    let rev = line.split([' ', '\t']).next().unwrap_or("");
    if !super::is_rev(rev) {
        return Err(super::fail("bad-record", "rev"));
    }
    Ok(rev.to_owned())
}

fn remote_arg(value: &str) -> Result<String, PackFailure> {
    super::token(value, "mirror")?;
    if value.contains("://") || value.starts_with("git@") {
        return Ok(value.to_owned());
    }
    canonical_dir(Path::new(value), "mirror")
}

fn canonical_dir(path: &Path, code: &str) -> Result<String, PackFailure> {
    let canonical = std::fs::canonicalize(path).map_err(|_| super::fail("missing-source", code))?;
    if !canonical.is_dir() {
        return Err(super::fail("bad-record", code));
    }
    canonical
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| super::fail("bad-record", code))
}

fn canonical_file(path: &Path, code: &str) -> Result<String, PackFailure> {
    let canonical = std::fs::canonicalize(path).map_err(|_| super::fail("missing-source", code))?;
    if !canonical.is_file() {
        return Err(super::fail("bad-record", code));
    }
    canonical
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| super::fail("bad-record", code))
}

fn read_limited(path: &Path, max: usize, code: &str) -> Result<Vec<u8>, PackFailure> {
    let bytes = std::fs::read(path).map_err(|_| super::fail("missing-source", code))?;
    if bytes.len() > max {
        return Err(super::fail("bound", code));
    }
    Ok(bytes)
}

fn require_string<'a>(table: &'a toml::Table, key: &str) -> Result<&'a str, PackFailure> {
    match table.get(key) {
        Some(toml::Value::String(value)) => Ok(value),
        _ => Err(super::fail("bad-record", key)),
    }
}
