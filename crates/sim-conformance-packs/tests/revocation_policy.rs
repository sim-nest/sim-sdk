use sim_conformance_core::{
    CheckInputClosureId, CheckScopeId, CheckedSubjectId, EvidenceGrade, IdKind, SemanticId,
};
use sim_conformance_packs::{
    MemorySubject, OperationLocalAuthorityError, OperationLocalOwner, find_pack,
};
use sim_kernel::{ContentId, Datum, Symbol};

fn sid<K: IdKind>(value: &str) -> SemanticId<K> {
    SemanticId::from_text(value).unwrap()
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

fn canonical(facts: &MemorySubject) -> String {
    facts
        .facts()
        .iter()
        .map(|(key, value)| format!("{key}={value}\n"))
        .collect()
}

fn subject(facts: &MemorySubject) -> CheckedSubjectId {
    CheckedSubjectId::from_fields(vec![(
        Symbol::qualified("conformance", "evidence"),
        Datum::String(canonical(facts)),
    )])
    .unwrap()
}

fn invocation(
    facts: &MemorySubject,
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
            subject(facts),
            CheckScopeId::from_text("operation/local").unwrap(),
            CheckInputClosureId::from_fields(vec![(
                Symbol::qualified("conformance", "evidence"),
                Datum::String(canonical(facts)),
            )])
            .unwrap(),
        )
        .unwrap();
    (binding, invocation)
}

fn owner(byte: u8) -> OperationLocalOwner {
    OperationLocalOwner::boot(ContentId::from_bytes(
        Symbol::qualified("conformance", "runner-generation"),
        [byte; 32],
    ))
}

fn issue(
    owner: &OperationLocalOwner,
    facts: &MemorySubject,
) -> sim_conformance_packs::QualifiedOperationLocalCheck {
    owner.mark_current(subject(facts)).unwrap();
    let authority = owner.operation_handle();
    let (binding, invocation) = invocation(facts);
    authority
        .check_and_issue(
            binding,
            invocation,
            facts,
            EvidenceGrade::Reproducible,
            sid("provenance/held-output"),
            sid("support/exact"),
        )
        .unwrap()
}

#[test]
fn owner_handle_issues_and_rechecks_without_accepting_snapshots() {
    let owner = owner(1);
    let facts = evidence();
    let authority = owner.operation_handle();
    let qualified = issue(&owner, &facts);
    authority.observe_receipt(&qualified).unwrap();

    owner.revoke(subject(&facts)).unwrap();
    assert!(qualified.verify_current(&authority).is_err());
}

#[test]
fn foreign_generation_cannot_be_substituted() {
    let first_owner = owner(2);
    let qualified = issue(&first_owner, &evidence());
    let second_owner = owner(3);
    let second = second_owner.operation_handle();
    assert!(matches!(
        second.observe_receipt(&qualified),
        Err(OperationLocalAuthorityError::ForeignAuthority)
    ));
}

#[test]
fn missing_owner_decision_fails_closed() {
    let owner = owner(4);
    let authority = owner.operation_handle();
    let facts = evidence();
    let (binding, invocation) = invocation(&facts);
    assert!(
        authority
            .check_and_issue(
                binding,
                invocation,
                &facts,
                EvidenceGrade::Bootstrap,
                sid("provenance/missing"),
                sid("support/missing"),
            )
            .is_err()
    );
}

#[test]
fn owner_drop_invalidates_handle_and_qualified_check() {
    let owner = owner(5);
    let authority = owner.operation_handle();
    let qualified = issue(&owner, &evidence());
    drop(owner);
    assert!(matches!(
        authority.ensure_live(),
        Err(OperationLocalAuthorityError::Unavailable)
    ));
    assert!(qualified.verify_current(&authority).is_err());
}

#[test]
fn substituted_facts_cannot_use_a_genuine_current_invocation() {
    let owner = owner(6);
    let original = evidence();
    owner.mark_current(subject(&original)).unwrap();
    let (binding, invocation) = invocation(&original);
    let substituted = original.clone().with("subject.variant", "substituted");
    assert!(matches!(
        owner.operation_handle().check_and_issue(
            binding,
            invocation,
            &substituted,
            EvidenceGrade::Reproducible,
            sid("provenance/substituted"),
            sid("support/substituted"),
        ),
        Err(OperationLocalAuthorityError::Unsupported)
    ));
}
