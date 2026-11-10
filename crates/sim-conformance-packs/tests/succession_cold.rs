// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_packs::{MemorySubject, PackRequest, PackVerdict, find_pack};

fn hex(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn record() -> String {
    format!(
        "\
change shade-1
project lantern
origin registry
warm false
fixture-change false
observed 3
proved 3
seal 2
converge-first 1
converge-second 0
land 1
deliver-first 1
deliver-second 0
recovery match
recovery-reads 2
holds 0
linux absent
closed portable
preimage {pre}
postimage {post}
",
        pre = hex(0x11),
        post = hex(0x22),
    )
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn binding(checker: &str) -> String {
    let spec = find_pack(checker).expect("pack");
    render(spec.checker_binding().expect("binding").id().content_id())
}

fn check(checker: &str, scope: &str, evidence: &MemorySubject) -> PackVerdict {
    let binding = binding(checker);
    let check = match checker {
        "checker/c-succeed" => sim_conformance_packs::packs::succession::check,
        "checker/c-own" => sim_conformance_packs::packs::ownership::check,
        other => panic!("unexpected checker {other}"),
    };
    check(&PackRequest {
        checker,
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
fn cold_change_derives_portable_closure() {
    let spec = find_pack("checker/c-succeed").expect("succeed");
    assert!(spec.implemented_scopes.contains(&"succession/cold-change"));
    assert!(
        !spec
            .implemented_scopes
            .contains(&"succession/composed-precheck")
    );
    let verdict = check(
        "checker/c-succeed",
        "succession/cold-change",
        &MemorySubject::default().with("succession.record", record()),
    );
    assert_eq!(code(&verdict), "pass");
    assert_eq!(value(&verdict, "succession.closed"), "portable");
    assert_eq!(value(&verdict, "succession.change"), "shade-1");
    assert_eq!(value(&verdict, "succession.project"), "lantern");
    assert_eq!(value(&verdict, "succession.second-converge"), "0");
    assert_eq!(value(&verdict, "succession.second-deliver"), "0");
    assert_eq!(value(&verdict, "succession.holds"), "0");
    assert_eq!(value(&verdict, "succession.linux"), "absent");
}

#[test]
fn composed_precheck_stays_unimplemented() {
    let verdict = check(
        "checker/c-succeed",
        "succession/composed-precheck",
        &MemorySubject::default().with("succession.record", record()),
    );
    assert_eq!(code(&verdict), "unimplemented");
}

#[test]
fn a_caller_boolean_second_work_or_fixture_change_is_refused() {
    let mut forced = MemorySubject::default().with("succession.record", record());
    forced = forced.with("succession.portable-closed", "true");
    assert_eq!(
        code(&check(
            "checker/c-succeed",
            "succession/cold-change",
            &forced
        )),
        "unexpected-fact"
    );
    let second = record().replace("deliver-second 0", "deliver-second 1");
    assert_eq!(
        code(&check(
            "checker/c-succeed",
            "succession/cold-change",
            &MemorySubject::default().with("succession.record", second)
        )),
        "second-work"
    );
    let fixture = record().replace("change shade-1", "change lantern");
    assert_eq!(
        code(&check(
            "checker/c-succeed",
            "succession/cold-change",
            &MemorySubject::default().with("succession.record", fixture)
        )),
        "fixture-change"
    );
    let open = record().replace("closed portable", "closed open");
    assert_eq!(
        code(&check(
            "checker/c-succeed",
            "succession/cold-change",
            &MemorySubject::default().with("succession.record", open)
        )),
        "not-closed"
    );
    for name in [
        "owning",
        "stub",
        "real",
        "precise-role",
        "git-missing",
        "version-mismatch",
    ] {
        let fixture = record().replace("change shade-1", &format!("change {name}"));
        assert_eq!(
            code(&check(
                "checker/c-succeed",
                "succession/cold-change",
                &MemorySubject::default().with("succession.record", fixture)
            )),
            "fixture-change",
            "{name}"
        );
    }
    for name in [
        "checkout",
        "my-transcript",
        "foo.git",
        "a..b",
        "Checkout",
        "file://x",
        "/tmp",
        "cache/obj",
    ] {
        let warm = record().replace("change shade-1", &format!("change {name}"));
        assert_eq!(
            code(&check(
                "checker/c-succeed",
                "succession/cold-change",
                &MemorySubject::default().with("succession.record", warm)
            )),
            "warm-state",
            "{name}"
        );
    }
    let malformed = record().replace("change shade-1", "change bad:token");
    assert_eq!(
        code(&check(
            "checker/c-succeed",
            "succession/cold-change",
            &MemorySubject::default().with("succession.record", malformed)
        )),
        "bad-record"
    );
    let same = record().replace(&hex(0x22), &hex(0x11));
    assert_eq!(
        code(&check(
            "checker/c-succeed",
            "succession/cold-change",
            &MemorySubject::default().with("succession.record", same)
        )),
        "seal"
    );
}

#[test]
fn roadmap_final_derives_portable_closure_from_the_same_record() {
    let spec = find_pack("checker/c-own").expect("own");
    assert!(spec.implemented_scopes.contains(&"ownership/roadmap-final"));
    let verdict = check(
        "checker/c-own",
        "ownership/roadmap-final",
        &MemorySubject::default().with("ownership.succession-record", record()),
    );
    assert_eq!(code(&verdict), "pass");
    assert_eq!(value(&verdict, "ownership.closed"), "portable");
    assert_eq!(value(&verdict, "ownership.cycles"), "0");
    assert_eq!(value(&verdict, "ownership.edges"), "0");
    assert_eq!(value(&verdict, "ownership.planned-targets"), "0");
    assert_eq!(value(&verdict, "ownership.linux"), "absent");
    assert_eq!(value(&verdict, "ownership.second-deliver"), "0");
    let forced = MemorySubject::default()
        .with("ownership.succession-record", record())
        .with("ownership.roadmap-final", "true");
    assert_eq!(
        code(&check("checker/c-own", "ownership/roadmap-final", &forced)),
        "unexpected-fact"
    );
    let edged = MemorySubject::default()
        .with("ownership.succession-record", record())
        .with("ownership.edge.support", "lantern");
    assert_eq!(
        code(&check("checker/c-own", "ownership/roadmap-final", &edged)),
        "graph"
    );
    let planned = MemorySubject::default()
        .with("ownership.succession-record", record())
        .with("ownership.planned.world", "world");
    assert_eq!(
        code(&check("checker/c-own", "ownership/roadmap-final", &planned)),
        "graph"
    );
}
