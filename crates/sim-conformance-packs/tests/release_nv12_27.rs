// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack};

fn hex(byte: u8, len: usize) -> String {
    format!("{byte:02x}").repeat(len / 2)
}

fn record(
    name: &str,
    version: &str,
    checksum: &str,
    pin: &str,
    ci: &str,
    boot: &str,
    deps: &str,
) -> String {
    let tag = format!("{name}-v{version}");
    format!(
        "crate {name}\nversion {version}\nchecksum {checksum}\ntag {tag}\npin {pin}\nmirror {pin}\nci {ci}\nboot {boot}\ndeps{deps}"
    )
}

fn pair() -> (String, String) {
    let kernel = record(
        "sim-kernel",
        "0.4.0",
        &hex(0x11, 64),
        &hex(0x22, 40),
        &hex(0x33, 64),
        &hex(0x44, 64),
        "",
    );
    let runtime = record(
        "sim-runtime",
        "0.2.1",
        &hex(0x55, 64),
        &hex(0x66, 40),
        &hex(0x77, 64),
        &hex(0x88, 64),
        " sim-kernel",
    );
    (kernel, runtime)
}

fn closure(blocks: &[String]) -> String {
    blocks.join("\n\n")
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn binding() -> String {
    let spec = find_pack("checker/c-release").expect("release pack");
    assert_eq!(
        spec.activation_binding,
        "core/sha256-datum-v1:f12e44d0825b1b54bec8ddfab96007eccc97b9dbcf99ac8ddfe6cb34fc720cf3"
    );
    assert!(spec.implemented_scopes.contains(&"release/nv12-27"));
    assert!(!spec.implemented_scopes.contains(&"release/nv12-26"));
    assert!(!spec.implemented_scopes.contains(&"release/nv12-28"));
    render(spec.checker_binding().expect("binding").id().content_id())
}

fn check(scope: &str, evidence: &MemorySubject) -> PackVerdict {
    let binding = binding();
    sim_conformance_packs::packs::release::check(&PackRequest {
        checker: "checker/c-release",
        binding: &binding,
        subject: "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        scope,
        evidence,
    })
}

fn code(verdict: &PackVerdict) -> &str {
    match verdict {
        PackVerdict::Refused(failure) => failure.code,
        PackVerdict::UnimplementedPack { .. } => "unimplemented",
        PackVerdict::Pass { .. } => "pass",
    }
}

fn digest_of(verdict: &PackVerdict) -> String {
    match verdict {
        PackVerdict::Pass { observations, .. } => observations
            .iter()
            .find(|item| item.key == "release.closure-digest")
            .expect("digest")
            .value
            .clone(),
        other => panic!("expected a consistent closure, got {other:?}"),
    }
}

#[test]
fn reconciled_closure_is_consistent_and_does_not_claim_publication() {
    let (kernel, runtime) = pair();
    let text = closure(&[kernel, runtime]);
    let verdict = check(
        "release/nv12-27",
        &MemorySubject::default().with("release.closure", text.clone()),
    );
    match verdict {
        PackVerdict::Pass { observations, .. } => {
            assert_eq!(
                observations
                    .iter()
                    .find(|item| item.key == "release.outward")
                    .expect("outward")
                    .value,
                "consistent"
            );
            assert_eq!(
                observations
                    .iter()
                    .find(|item| item.key == "release.order")
                    .expect("order")
                    .value,
                "sim-kernel,sim-runtime"
            );
            assert_eq!(
                observations
                    .iter()
                    .find(|item| item.key == "release.crate-count")
                    .expect("count")
                    .value,
                "2"
            );
            assert!(observations.iter().all(|item| item.value != "true"));
            assert!(observations.iter().all(|item| item.value != "published"));
        }
        other => panic!("expected a consistent closure, got {other:?}"),
    }
    let with_newline = check(
        "release/nv12-27",
        &MemorySubject::default().with("release.closure", format!("{text}\n")),
    );
    assert_eq!(
        digest_of(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", text.clone()),
        )),
        digest_of(&with_newline)
    );
}

#[test]
fn closure_identity_follows_bytes_order_and_dependencies() {
    let (kernel, runtime) = pair();
    let forward = digest_of(&check(
        "release/nv12-27",
        &MemorySubject::default().with(
            "release.closure",
            closure(&[kernel.clone(), runtime.clone()]),
        ),
    ));
    let first = record(
        "sim-kernel",
        "0.4.0",
        &hex(0x11, 64),
        &hex(0x22, 40),
        &hex(0x33, 64),
        &hex(0x44, 64),
        "",
    );
    let second = record(
        "sim-runtime",
        "0.2.1",
        &hex(0x55, 64),
        &hex(0x66, 40),
        &hex(0x77, 64),
        &hex(0x88, 64),
        "",
    );
    let ordered = digest_of(&check(
        "release/nv12-27",
        &MemorySubject::default()
            .with("release.closure", closure(&[first.clone(), second.clone()])),
    ));
    let reversed = digest_of(&check(
        "release/nv12-27",
        &MemorySubject::default().with("release.closure", closure(&[second, first])),
    ));
    assert_ne!(ordered, reversed);
    let repeated = record(
        "sim-kernel",
        "0.4.0",
        &hex(0x11, 64),
        &hex(0x22, 40),
        &hex(0x33, 64),
        &hex(0x44, 64),
        " sim-runtime sim-runtime",
    );
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", repeated),
        )),
        "duplicate-dependency"
    );
    let swapped = digest_of(&check(
        "release/nv12-27",
        &MemorySubject::default().with(
            "release.closure",
            closure(&[
                record(
                    "sim-kernel",
                    "0.4.0",
                    &hex(0x55, 64),
                    &hex(0x22, 40),
                    &hex(0x33, 64),
                    &hex(0x44, 64),
                    "",
                ),
                runtime,
            ]),
        ),
    ));
    assert_ne!(forward, swapped);
}

#[test]
fn local_publication_flags_and_partial_closures_are_refused() {
    let (kernel, runtime) = pair();
    let text = closure(&[kernel, runtime]);
    let good = MemorySubject::default().with("release.closure", text.clone());
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &good.clone().with("release.publication-confirmed", "true"),
        )),
        "local-publication-claim"
    );
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.publication-confirmed", "true"),
        )),
        "local-publication-claim"
    );
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.dry-run", "sim-kernel 0.4.0"),
        )),
        "publication-unreconciled"
    );
    assert_eq!(
        code(&check("release/nv12-27", &MemorySubject::default())),
        "missing-closure"
    );
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &good.clone().with("release.packages-assembled", "true"),
        )),
        "unexpected-fact"
    );
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &good
                .clone()
                .with("release.dry-run", "sim-kernel 0.4.1\nsim-runtime 0.2.1\n"),
        )),
        "dry-run-mismatch"
    );
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &good.with("release.dry-run", "sim-kernel 0.4.0\nsim-runtime 0.2.1"),
        )),
        "pass"
    );
    let mut broken = text.replace(&hex(0x11, 64), &"A".repeat(64));
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", broken.clone()),
        )),
        "bad-checksum"
    );
    broken.clone_from(&text);
    let bad_tag = broken.replace("tag sim-kernel-v0.4.0", "tag sim-kernel-v0.4.1");
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", bad_tag),
        )),
        "tag-mismatch"
    );
    let bad_mirror = text.replace(
        &format!("mirror {}", hex(0x22, 40)),
        &format!("mirror {}", hex(0x99, 40)),
    );
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", bad_mirror),
        )),
        "mirror-mismatch"
    );
    let bad_ci = text.replace(&format!("ci {}", hex(0x33, 64)), "ci true");
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", bad_ci),
        )),
        "bad-ci"
    );
    let bad_boot = text.replace(&format!("boot {}", hex(0x44, 64)), "boot true");
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", bad_boot),
        )),
        "bad-boot"
    );
    let (kernel, _) = pair();
    let duplicated = closure(&[kernel.clone(), kernel]);
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", duplicated),
        )),
        "duplicate-crate"
    );
    let self_dep = record(
        "sim-kernel",
        "0.4.0",
        &hex(0x11, 64),
        &hex(0x22, 40),
        &hex(0x33, 64),
        &hex(0x44, 64),
        " sim-kernel",
    );
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", self_dep),
        )),
        "order"
    );
    let missing = record(
        "sim-runtime",
        "0.2.1",
        &hex(0x55, 64),
        &hex(0x66, 40),
        &hex(0x77, 64),
        &hex(0x88, 64),
        " sim-kernel",
    );
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", missing),
        )),
        "missing-dependency"
    );
    let early = closure(&[
        record(
            "sim-runtime",
            "0.2.1",
            &hex(0x55, 64),
            &hex(0x66, 40),
            &hex(0x77, 64),
            &hex(0x88, 64),
            " sim-kernel",
        ),
        record(
            "sim-kernel",
            "0.4.0",
            &hex(0x11, 64),
            &hex(0x22, 40),
            &hex(0x33, 64),
            &hex(0x44, 64),
            "",
        ),
    ]);
    assert_eq!(
        code(&check(
            "release/nv12-27",
            &MemorySubject::default().with("release.closure", early),
        )),
        "order"
    );
    assert_eq!(
        code(&check(
            "release/nv12-26",
            &MemorySubject::default().with("release.closure", text),
        )),
        "unimplemented"
    );
}
