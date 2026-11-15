// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack};

fn hex(byte: u8, len: usize) -> String {
    format!("{byte:02x}").repeat(len / 2)
}

fn record(name: &str, version: &str, deps: &str) -> String {
    let tag = format!("{name}-v{version}");
    let pin = hex(0x22, 40);
    format!(
        "crate {name}\nversion {version}\nchecksum {}\ntag {tag}\npin {pin}\nmirror {pin}\nci {}\nboot {}\ndeps{deps}",
        hex(0x11, 64),
        hex(0x33, 64),
        hex(0x44, 64),
    )
}

fn closure() -> String {
    format!(
        "{}\n\n{}",
        record("sim-kernel", "0.4.0", ""),
        record("sim-runtime", "0.2.1", " sim-kernel")
    )
}

fn succession() -> String {
    format!(
        "change shade-1\nproject lantern\norigin registry\nwarm false\nfixture-change false\nobserved 3\nproved 3\nseal 2\nconverge-first 1\nconverge-second 0\nland 1\ndeliver-first 1\ndeliver-second 0\nrecovery match\nrecovery-reads 2\nholds 0\nlinux absent\nclosed portable\npreimage {}\npostimage {}\n",
        hex(0x11, 64),
        hex(0x22, 64),
    )
}

fn binding() -> String {
    let spec = find_pack("checker/c-release").expect("release pack");
    let digest = spec
        .checker_binding()
        .expect("binding")
        .id()
        .content_id()
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "{}:{digest}",
        spec.checker_binding()
            .expect("binding")
            .id()
            .content_id()
            .algorithm
            .as_qualified_str()
    )
}

fn check(evidence: &MemorySubject) -> PackVerdict {
    sim_conformance_packs::packs::release::check(&PackRequest {
        checker: "checker/c-release",
        binding: &binding(),
        subject: "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        scope: "release/nv12-28",
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

fn value<'a>(verdict: &'a PackVerdict, key: &str) -> &'a str {
    match verdict {
        PackVerdict::Pass { observations, .. } => observations
            .iter()
            .find(|item| item.key == key)
            .unwrap_or_else(|| panic!("missing {key}"))
            .value
            .as_str(),
        other => panic!("expected a pass, got {other:?}"),
    }
}

#[test]
fn derived_succession_does_not_claim_publication_or_completion() {
    let spec = find_pack("checker/c-release").expect("release pack");
    assert!(spec.implemented_scopes.contains(&"release/nv12-28"));
    assert!(!spec.implemented_scopes.contains(&"release/nv12-26"));
    let evidence = MemorySubject::default()
        .with("release.closure", closure())
        .with("succession.record", succession());
    let verdict = check(&evidence);
    assert_eq!(code(&verdict), "pass");
    assert_eq!(value(&verdict, "release.outward"), "consistent");
    assert_eq!(value(&verdict, "release.order"), "sim-kernel,sim-runtime");
    assert_eq!(value(&verdict, "release.succession"), "derived");
    assert_eq!(value(&verdict, "succession.closed"), "portable");
    assert_eq!(value(&verdict, "succession.change"), "shade-1");
    assert_eq!(value(&verdict, "succession.second-converge"), "0");
    assert_eq!(value(&verdict, "succession.second-deliver"), "0");
    assert_eq!(value(&verdict, "succession.holds"), "0");
    assert_eq!(value(&verdict, "succession.linux"), "absent");
    match &verdict {
        PackVerdict::Pass { observations, .. } => {
            assert!(observations.iter().all(|item| item.value != "true"));
            assert!(observations.iter().all(|item| item.value != "published"));
            assert!(
                observations
                    .iter()
                    .all(|item| item.value != "PortableClosed")
            );
            assert!(
                observations
                    .iter()
                    .all(|item| item.key != "release.completion")
            );
        }
        other => panic!("expected a pass, got {other:?}"),
    }
    let matched = check(&evidence.with("release.dry-run", "sim-kernel 0.4.0\nsim-runtime 0.2.1\n"));
    assert_eq!(code(&matched), "pass");
}

#[test]
fn publication_flags_partial_records_and_second_work_are_refused() {
    let closure = closure();
    let succession = succession();
    let both = MemorySubject::default()
        .with("release.closure", closure.clone())
        .with("succession.record", succession.clone());
    assert_eq!(
        code(&check(
            &both.clone().with("release.publication-confirmed", "true")
        )),
        "local-publication-claim"
    );
    assert_eq!(
        code(&check(
            &both.clone().with("succession.portable-closed", "true")
        )),
        "unexpected-fact"
    );
    assert_eq!(
        code(&check(
            &MemorySubject::default().with("succession.record", succession.clone())
        )),
        "missing-closure"
    );
    assert_eq!(
        code(&check(
            &MemorySubject::default().with("release.closure", closure.clone())
        )),
        "missing-succession"
    );
    assert_eq!(code(&check(&MemorySubject::default())), "missing-closure");
    assert_eq!(
        code(&check(
            &MemorySubject::default().with("release.dry-run", "sim-kernel 0.4.0\n")
        )),
        "publication-unreconciled"
    );
    assert_eq!(
        code(&check(&both.clone().with(
            "release.dry-run",
            "sim-kernel 0.4.1\nsim-runtime 0.2.1\n"
        ))),
        "dry-run-mismatch"
    );
    assert_eq!(
        code(&check(&both.clone().with("release.completion", "portable"))),
        "unexpected-fact"
    );
    let second = succession.replace("deliver-second 0", "deliver-second 1");
    assert_eq!(
        code(&check(
            &MemorySubject::default()
                .with("release.closure", closure.clone())
                .with("succession.record", second)
        )),
        "second-work"
    );
    for word in ["PortableClosed", "published", "true"] {
        let renamed = succession.replace("change shade-1", &format!("change {word}"));
        assert_eq!(
            code(&check(
                &MemorySubject::default()
                    .with("release.closure", closure.clone())
                    .with("succession.record", renamed)
            )),
            "claim-value"
        );
    }
    for name in ["true", "published"] {
        assert_eq!(
            code(&check(
                &MemorySubject::default()
                    .with("release.closure", record(name, "0.4.0", ""))
                    .with("succession.record", succession.clone())
            )),
            "claim-value"
        );
    }
}
