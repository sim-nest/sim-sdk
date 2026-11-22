// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::BTreeMap;

use sim_kernel::Datum;
use sim_lib_world::completion::{SuppliedOwnerClaim, final_catalog, supply_owner_claim};

use super::{CHANGE, EXPORT, SCOPE, read_change};
use crate::{MemorySubject, PackRequest, PackVerdict, all_packs, find_pack};

#[test]
fn the_scope_stays_on_the_existing_pack() {
    let spec = find_pack("checker/c-succeed").expect("succeed");
    assert!(spec.implemented_scopes.contains(&SCOPE));
    assert!(
        spec.implemented_scopes
            .contains(&"succession/composed-precheck")
    );
    assert!(spec.implemented_scopes.contains(&"succession/cold-change"));
    assert_eq!(all_packs().len(), 21);
    assert_eq!(
        spec.activation_binding,
        "core/sha256-datum-v1:f14267fae4d45eb57c175546ebd809831a0cfa9510bd109f04ec9a280269feab"
    );
}

#[test]
fn a_claim_label_is_refused() {
    let error = super::observe_fresh_command("completion/claim-23").expect_err("refused");
    assert_eq!(error.code, "bad-record");
}

#[test]
fn a_publication_flag_is_refused() {
    let verdict = check(&MemorySubject::default().with("release.publication-confirmed", "true"));
    assert_eq!(code(&verdict), "unexpected-fact");
}

#[test]
fn the_construction_specimen_is_refused() {
    let error = read_change(EXPORT, "shade-1 harbor provider/shade observed shade-next")
        .expect_err("specimen");
    assert_eq!(error.code, "fresh-unqualified");
    assert_ne!(CHANGE, "shade-1 harbor provider/shade observed shade-next");
}

#[test]
fn the_installed_later_change_admits_claim_23() {
    let before = final_catalog("").expect("catalog");
    let subject = field(&before, "subject").expect("subject");
    assert!(
        !claims(&before).iter().any(|claim| claim == "23"),
        "claim 23 is not proved before the fresh command"
    );
    let observed = super::observe_fresh_command(&subject).expect("admitted");
    assert_eq!(observed.result, "completion/claim-23");
    assert_eq!(observed.subject, subject);
    let mut revocations = BTreeMap::new();
    revocations.insert(observed.checker.clone(), observed.revocation);
    supply_owner_claim(SuppliedOwnerClaim {
        subject: observed.subject,
        claim: 23,
        admitted: observed.admitted,
        checker: observed.checker,
        catalog: observed.catalog,
        scope: observed.scope,
        revocations,
    });
    let after = final_catalog("").expect("catalog");
    assert_eq!(claims(&after), vec!["23".to_owned()]);
    assert_eq!(field(&after, "subject").as_deref(), Some(subject.as_str()));
    assert_eq!(field(&after, "seal").as_deref(), Some("unclaimed"));
    let fresh = child(&after, "fresh");
    let admitted = child(&fresh, "admitted");
    assert_eq!(
        text(&child(&fresh, "specimen"), "world/fresh-gap-v1", "reason"),
        "construction-specimen"
    );
    assert_eq!(text(&admitted, "world/fresh-v1", "change"), "later-0310");
    assert_eq!(text(&admitted, "world/fresh-v1", "project"), "harbor");
    assert_eq!(text(&admitted, "world/fresh-v1", "seal"), "unclaimed");
    assert_eq!(text(&admitted, "world/fresh-v1", "succession"), "portable");
    assert_eq!(text(&admitted, "world/fresh-v1", "linux"), "absent");
}

fn check(evidence: &MemorySubject) -> PackVerdict {
    let spec = find_pack("checker/c-succeed").expect("succeed");
    let binding = render(spec.checker_binding().expect("binding").id().content_id());
    super::super::check(&PackRequest {
        checker: "checker/c-succeed",
        binding: &binding,
        subject: "core/sha256-datum-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        scope: SCOPE,
        evidence,
    })
}

fn render(id: &sim_kernel::ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}

fn code(verdict: &PackVerdict) -> &str {
    match verdict {
        PackVerdict::Refused(failure) => failure.code,
        PackVerdict::UnimplementedPack { .. } => "unimplemented",
        PackVerdict::Pass { .. } => "pass",
    }
}

fn field(datum: &Datum, name: &str) -> Option<String> {
    let Datum::Node { fields, .. } = datum else {
        return None;
    };
    fields.iter().find_map(|(symbol, value)| {
        (symbol.as_qualified_str() == name).then(|| match value {
            Datum::String(text) => text.clone(),
            _ => String::new(),
        })
    })
}

fn claims(datum: &Datum) -> Vec<String> {
    let Datum::Node { fields, .. } = datum else {
        return Vec::new();
    };
    let Some(Datum::Vector(items)) = fields
        .iter()
        .find(|(symbol, _)| symbol.as_qualified_str() == "admissions")
        .map(|(_, value)| value)
    else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| match item {
            Datum::String(text) => text.clone(),
            _ => String::new(),
        })
        .collect()
}

fn child(record: &Datum, name: &str) -> Datum {
    let Datum::Node { fields, .. } = record else {
        panic!("node");
    };
    fields
        .iter()
        .find(|(field, _)| field.as_qualified_str() == name)
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn text(record: &Datum, tag: &str, name: &str) -> String {
    let Datum::Node {
        tag: found, fields, ..
    } = record
    else {
        panic!("node");
    };
    assert_eq!(found.as_qualified_str(), tag);
    match fields
        .iter()
        .find(|(field, _)| field.as_qualified_str() == name)
        .map(|(_, value)| value)
    {
        Some(Datum::String(value)) => value.clone(),
        other => panic!("missing {name}: {other:?}"),
    }
}
