// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sim_conformance_v02::CheckScopeId;

use super::{
    bind_binding_catalog, bind_identity_roundtrip, observe_binding_catalog,
    observe_identity_roundtrip,
};

const SUBJECT: &str = "subject/binding-catalog";

#[test]
fn binding_catalog_rebuilds_the_static_checkers() {
    let observed = observe_binding_catalog(SUBJECT).expect("bindings");
    assert_eq!(observed.result, "binding-catalog");
    assert!(
        observed
            .checkers
            .iter()
            .any(|checker| checker == "checker/c-release")
    );
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/binding-catalog").expect("scope")
    );
    bind_binding_catalog(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-25".to_owned();
    assert_eq!(
        bind_binding_catalog(&relabelled).expect_err("relabel").code,
        "claim-relabelled"
    );
}

#[test]
fn identity_roundtrip_matches_the_golden_vector() {
    let observed = observe_identity_roundtrip(SUBJECT).expect("identity");
    assert_eq!(observed.result, "identity-roundtrip");
    assert_eq!(
        observed.vector,
        "c13d17c133a40252864f27f34aeadaeb273a83dadd2f9726754d31a2e02d6e68"
    );
    assert_eq!(
        observed.scope,
        CheckScopeId::from_text("release/identity-roundtrip").expect("scope")
    );
    bind_identity_roundtrip(&observed).expect("bind");
    let mut relabelled = observed;
    relabelled.result = "completion/claim-5".to_owned();
    assert_eq!(
        bind_identity_roundtrip(&relabelled)
            .expect_err("relabel")
            .code,
        "claim-relabelled"
    );
}

#[test]
fn a_claim_string_is_not_a_binding_execution() {
    let spec = crate::find_pack("checker/c-release").expect("pack");
    let binding = spec.checker_binding().expect("binding");
    let subject = sim_conformance_v02::CheckedSubjectId::from_text(SUBJECT).expect("subject");
    let verdict = super::super::check(&crate::PackRequest {
        checker: "checker/c-release",
        binding: &render(binding.id().content_id()),
        subject: &render(subject.content_id()),
        scope: "release/binding-catalog",
        evidence: &crate::MemorySubject::default().with("release.bindings", "completion/claim-25"),
    });
    match verdict {
        crate::PackVerdict::Refused(failure) => assert_eq!(failure.code, "claim-relabelled"),
        other => panic!("claim text was accepted: {other:?}"),
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
