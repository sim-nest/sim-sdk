use std::sync::Arc;

use sim_conformance_core::{
    CheckInputClosureId, CheckScopeId, CheckedSubjectId, EvidenceGrade, EvidenceProvenanceId,
    EvidenceSetId, IdKind, SemanticId,
};
use sim_conformance_packs::{
    MemorySubject, OperationLocalAuthorityError, bind_operation_local_owner, find_pack,
};
use sim_kernel::{Cx, DefaultFactory, EagerPolicy, HandleSeed};

fn sid<K: IdKind>(value: &str) -> SemanticId<K> {
    SemanticId::from_text(value).unwrap()
}

fn provenance(value: &str) -> EvidenceProvenanceId {
    sid(value)
}

fn support(value: &str) -> EvidenceSetId {
    sid(value)
}

fn evidence() -> MemorySubject {
    [
        "operation.local-port-is-portable",
        "operation.local-command-is-installed-and-allowlisted",
        "operation.local-command-id-binds-complete-spec",
        "operation.local-manifest-script-bytes-exact",
        "operation.local-model-interpolation-refused",
        "operation.local-environment-sealed",
        "operation.local-writable-roots-confined",
        "operation.local-network-absent",
        "operation.local-network-grant-separate",
        "operation.local-release-credentials-absent",
        "operation.local-formatter-mutation-observed",
        "operation.local-test-success-observed",
        "operation.local-test-failure-observed",
        "operation.local-validation-command-exact",
        "operation.local-docs-command-exact",
        "operation.local-timeout-terminates-group",
        "operation.local-cancellation-terminates-group",
        "operation.local-signal-escalation-bounded",
        "operation.local-descendants-zero",
        "operation.local-scratch-zero",
        "operation.local-independent-postcondition",
        "operation.local-later-owner-gates-reachable",
    ]
    .into_iter()
    .fold(MemorySubject::default(), |subject, key| {
        subject.with(key, "true")
    })
}

fn context(seed: u64) -> (Cx, sim_kernel::GrantSeat) {
    Cx::new_seated(
        Arc::new(EagerPolicy),
        Arc::new(DefaultFactory),
        HandleSeed::new(seed),
    )
}

fn invocation(
    subject: CheckedSubjectId,
) -> (
    sim_conformance_core::CheckerBinding,
    sim_conformance_core::CheckInvocation,
) {
    let spec = find_pack("checker/c-op").unwrap();
    let binding = spec.checker_binding().unwrap();
    let invocation = binding
        .instantiate(
            spec.checker_code_id().unwrap(),
            spec.pack_id().unwrap(),
            subject,
            CheckScopeId::from_text("operation/local").unwrap(),
            CheckInputClosureId::from_text("closure/exact").unwrap(),
        )
        .unwrap();
    (binding, invocation)
}

#[test]
fn owner_handle_issues_and_rechecks_without_accepting_snapshots() {
    let (mut cx, seat) = context(1);
    let (authority, mut control) = bind_operation_local_owner(&mut cx, &seat).unwrap();
    let subject: CheckedSubjectId = sid("subject/a");
    control.mark_current(subject.clone()).unwrap();
    let (binding, invocation) = invocation(subject.clone());
    let qualified = authority
        .check_and_issue(
            binding,
            invocation,
            &evidence(),
            EvidenceGrade::Reproducible,
            provenance("provenance/held-output"),
            support("support/exact"),
        )
        .unwrap();
    authority.observe_receipt(&qualified).unwrap();

    control.revoke(subject).unwrap();
    assert!(qualified.verify_current(&authority).is_err());
}

#[test]
fn old_owner_and_foreign_owner_cannot_be_substituted() {
    let (mut first_cx, first_seat) = context(2);
    let (first, mut first_control) =
        bind_operation_local_owner(&mut first_cx, &first_seat).unwrap();
    let subject: CheckedSubjectId = sid("subject/a");
    first_control.mark_current(subject.clone()).unwrap();
    let (binding, invocation) = invocation(subject);
    let qualified = first
        .check_and_issue(
            binding,
            invocation,
            &evidence(),
            EvidenceGrade::Bootstrap,
            provenance("provenance/a"),
            support("support/a"),
        )
        .unwrap();

    let (mut second_cx, second_seat) = context(3);
    let (second, _) = bind_operation_local_owner(&mut second_cx, &second_seat).unwrap();
    assert!(matches!(
        second.observe_receipt(&qualified),
        Err(OperationLocalAuthorityError::ForeignAuthority)
    ));
}

#[test]
fn missing_owner_decision_fails_closed() {
    let (mut cx, seat) = context(4);
    let (authority, _) = bind_operation_local_owner(&mut cx, &seat).unwrap();
    let (binding, invocation) = invocation(sid("subject/missing"));
    assert!(
        authority
            .check_and_issue(
                binding,
                invocation,
                &evidence(),
                EvidenceGrade::Bootstrap,
                provenance("provenance/missing"),
                support("support/missing"),
            )
            .is_err()
    );
}

#[test]
fn foreign_host_seat_cannot_bind_an_owner() {
    let (mut first, _) = context(5);
    let (_, foreign_seat) = context(6);
    assert!(matches!(
        bind_operation_local_owner(&mut first, &foreign_seat),
        Err(OperationLocalAuthorityError::Kernel(
            sim_kernel::Error::ForeignGrantSeat
        ))
    ));
}
