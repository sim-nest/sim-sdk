// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_packs::{
    MemorySubject, PackRequest, PackSubject, PackVerdict, compiled_executor_closure,
    executor_support_evidence, find_pack, packs,
};

const SUBJECT: &str =
    "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn request<'a>(checker: &'a str, scope: &'a str, evidence: &'a MemorySubject) -> PackRequest<'a> {
    let binding = find_pack(checker).unwrap().checker_binding().unwrap();
    let binding = Box::leak(Box::new(render(binding.id().content_id())));
    PackRequest {
        checker,
        binding,
        subject: SUBJECT,
        scope,
        evidence,
    }
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn run(checker: &str, scope: &str, evidence: &MemorySubject) -> PackVerdict {
    let request = request(checker, scope, evidence);
    match checker {
        "checker/c-source" => packs::source::check(&request),
        "checker/c-closure" => packs::closure::check(&request),
        "checker/c-control" => packs::control::check(&request),
        "checker/c-drive" => packs::drive::check(&request),
        "checker/c-converge" => packs::convergence::check(&request),
        other => panic!("unchecked {other}"),
    }
}

#[test]
fn funded_scopes_require_executor_support_and_native_drive_stays_open() {
    let empty = MemorySubject::default();
    let echoed = MemorySubject::default()
        .with("support.subject", SUBJECT)
        .with("support.closure", compiled_executor_closure());
    let issued = executor_support_evidence(SUBJECT).expect("executor support");
    let entrypoint = issued
        .support_qualification()
        .expect("receipt")
        .invocation()
        .execution()
        .entrypoint();
    assert_eq!(entrypoint, "sim_conformance_packs::executor_support::prove");
    let copied = issued
        .facts()
        .iter()
        .fold(MemorySubject::default(), |subject, (key, value)| {
            subject.with(key.as_str(), value.as_str())
        });
    let cloned = issued.clone();
    let funded = [
        ("checker/c-source", "source/admission"),
        ("checker/c-source", "source/performance"),
        ("checker/c-source", "source/composed"),
        ("checker/c-closure", "closure/projection"),
        ("checker/c-closure", "closure/plan"),
        ("checker/c-closure", "closure/frontier"),
        ("checker/c-closure", "closure/resources"),
        ("checker/c-closure", "closure/composed"),
        ("checker/c-control", "control/liveness"),
        ("checker/c-control", "control/composed"),
        ("checker/c-drive", "drive/hot-no-op"),
        ("checker/c-drive", "drive/composed"),
        ("checker/c-converge", "convergence/candidate"),
        ("checker/c-converge", "convergence/fixed-point"),
        ("checker/c-converge", "convergence/composed"),
    ];
    for (checker, scope) in funded {
        match run(checker, scope, &empty) {
            PackVerdict::Refused(failure) => {
                assert_eq!(failure.code, "unqualified-support", "{checker} {scope}");
            }
            other => panic!("{checker} {scope} accepted empty evidence: {other:?}"),
        }
        match run(checker, scope, &echoed) {
            PackVerdict::Refused(failure) => {
                assert_eq!(failure.code, "unqualified-support", "{checker} {scope}");
            }
            other => panic!("{checker} {scope} accepted a copied closure: {other:?}"),
        }
        match run(checker, scope, &copied) {
            PackVerdict::Refused(failure) => {
                assert_eq!(failure.code, "unqualified-support", "{checker} {scope}");
            }
            other => panic!("{checker} {scope} accepted a fact-only copy: {other:?}"),
        }
        match run(checker, scope, &issued) {
            PackVerdict::Pass { observations, .. } => {
                assert!(
                    observations.iter().any(|item| item.value == SUBJECT),
                    "{checker} {scope} dropped the subject: {observations:?}"
                );
            }
            other => panic!("{checker} {scope} did not pass with recorded support: {other:?}"),
        }
    }
    match run("checker/c-source", "source/admission", &cloned) {
        PackVerdict::Pass { .. } => {}
        other => panic!("rejected the issued support object: {other:?}"),
    }
    let other = executor_support_evidence("subject/other").expect("other subject");
    match run("checker/c-source", "source/admission", &other) {
        PackVerdict::Refused(failure) => {
            assert_eq!(failure.code, "unqualified-support");
        }
        other => panic!("accepted support issued for another subject: {other:?}"),
    }
    for evidence in [&empty, &issued] {
        assert!(matches!(
            run("checker/c-drive", "drive/native", evidence),
            PackVerdict::UnimplementedPack { ref scope, .. } if scope == "drive/native"
        ));
    }
}
