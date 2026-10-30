use sim_conformance_core::{
    CheckInputClosureId, CheckScopeId, CheckedSubjectId, IdKind, SemanticId,
};
use sim_conformance_packs::{
    MemorySubject, OperationLocalAuthorityError, OperationLocalEvidence,
    OperationLocalEvidenceError, OperationLocalOwner, OperationLocalSupportDefinition, PackSubject,
    find_pack,
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
    let binding = spec.native_operation_local_binding().unwrap();
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

fn support(evidence: &OperationLocalEvidence) -> OperationLocalSupportDefinition {
    OperationLocalSupportDefinition::new(evidence.canonical().lines().map(|line| {
        let (name, _) = line.split_once('=').unwrap();
        (
            name.to_owned(),
            Datum::String(format!("support:{name}"))
                .content_id()
                .unwrap(),
        )
    }))
    .unwrap()
}

fn issue(
    owner: &OperationLocalOwner,
    facts: &MemorySubject,
) -> sim_conformance_packs::QualifiedOperationLocalCheck {
    owner.mark_current(subject(facts)).unwrap();
    let authority = owner.operation_handle();
    let (binding, invocation) = invocation(facts);
    let frozen = OperationLocalEvidence::capture(facts).unwrap();
    let support = support(&frozen);
    authority
        .check_and_issue(
            binding,
            invocation,
            &frozen,
            &support,
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
    let frozen = OperationLocalEvidence::capture(&facts).unwrap();
    let support = support(&frozen);
    assert!(
        authority
            .check_and_issue(
                binding,
                invocation,
                &frozen,
                &support,
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
    let substituted = OperationLocalEvidence::capture(&substituted).unwrap();
    let support = support(&substituted);
    assert!(matches!(
        owner.operation_handle().check_and_issue(
            binding,
            invocation,
            &substituted,
            &support,
            sid("provenance/substituted"),
            sid("support/substituted"),
        ),
        Err(OperationLocalAuthorityError::Unsupported)
    ));
}

#[test]
fn stateful_subject_is_read_once_before_identity_and_check() {
    use std::cell::Cell;

    struct SwitchingSubject {
        base: MemorySubject,
        target_reads: Cell<usize>,
    }
    impl PackSubject for SwitchingSubject {
        fn fact(&self, key: &str) -> Option<&str> {
            if key == "operation.local-network-absent" {
                let reads = self.target_reads.get();
                self.target_reads.set(reads + 1);
                return Some(if reads == 0 { "false" } else { "true" });
            }
            self.base.fact(key)
        }

        fn fact_keys(&self) -> Vec<&str> {
            self.base.fact_keys()
        }
    }

    let changing = SwitchingSubject {
        base: evidence(),
        target_reads: Cell::new(0),
    };
    let frozen = OperationLocalEvidence::capture(&changing).unwrap();
    assert_eq!(changing.target_reads.get(), 1);
    let owner = owner(7);
    owner.mark_current(frozen.subject_id().unwrap()).unwrap();
    let spec = find_pack("checker/c-op").unwrap();
    let binding = spec.native_operation_local_binding().unwrap();
    let invocation = binding
        .instantiate(
            spec.checker_code_id().unwrap(),
            spec.pack_id().unwrap(),
            frozen.subject_id().unwrap(),
            CheckScopeId::from_text("operation/local").unwrap(),
            frozen.input_closure_id().unwrap(),
        )
        .unwrap();
    let support = support(&frozen);
    assert!(matches!(
        owner.operation_handle().check_and_issue(
            binding,
            invocation,
            &frozen,
            &support,
            sid("provenance/stateful"),
            sid("support/stateful"),
        ),
        Err(OperationLocalAuthorityError::Refused(_))
    ));
    assert_eq!(changing.target_reads.get(), 1);
}

#[test]
fn delimiter_collisions_are_refused_before_identity_derivation() {
    let split = evidence().with("extra.a", "left").with("extra.b", "right");
    let injected = evidence().with("extra.a", "left\nextra.b=right");
    let split = OperationLocalEvidence::capture(&split).unwrap();
    assert!(matches!(
        OperationLocalEvidence::capture(&injected),
        Err(OperationLocalEvidenceError::Malformed(_))
    ));
    assert!(split.canonical().contains("extra.a=left\nextra.b=right\n"));

    let ambiguous_key = evidence().with("extra.a=left", "right");
    assert!(OperationLocalEvidence::capture(&ambiguous_key).is_err());
}

#[test]
fn every_p3_forbidden_evidence_class_fails_at_the_shared_constructor() {
    for malformed in [
        b"".as_slice(),
        b"a=true".as_slice(),
        b"a=true\r\n".as_slice(),
        b"a=\n".as_slice(),
        b"a =true\n".as_slice(),
        b"a=bad\x01value\n".as_slice(),
        b"b=true\na=true\n".as_slice(),
        b"a=true\na=false\n".as_slice(),
        b"no-separator\n".as_slice(),
        &[0xff, b'=', b'x', b'\n'],
    ] {
        assert!(OperationLocalEvidence::parse(malformed).is_err());
    }

    let long_key = format!("{}=true\n", "k".repeat(257));
    assert!(OperationLocalEvidence::parse(long_key.as_bytes()).is_err());
    let long_value = format!("key={}\n", "v".repeat(4_097));
    assert!(OperationLocalEvidence::parse(long_value.as_bytes()).is_err());

    let too_many = (0..257)
        .map(|index| format!("key{index:03}=true\n"))
        .collect::<String>();
    assert!(matches!(
        OperationLocalEvidence::parse(too_many.as_bytes()),
        Err(OperationLocalEvidenceError::FactBound)
    ));

    let too_large = (0..5)
        .map(|index| format!("key{index}={}\n", "v".repeat(4_096)))
        .collect::<String>();
    assert!(matches!(
        OperationLocalEvidence::parse(too_large.as_bytes()),
        Err(OperationLocalEvidenceError::ByteBound)
    ));
}

#[test]
fn support_definition_is_order_independent_and_duplicate_safe() {
    let alpha = Datum::String("alpha-record".into()).content_id().unwrap();
    let beta = Datum::String("beta-record".into()).content_id().unwrap();
    let forward = OperationLocalSupportDefinition::new(vec![
        ("fact.alpha".into(), alpha.clone()),
        ("fact.beta".into(), beta.clone()),
    ])
    .unwrap();
    let reverse = OperationLocalSupportDefinition::new(vec![
        ("fact.beta".into(), beta),
        ("fact.alpha".into(), alpha.clone()),
    ])
    .unwrap();
    assert_eq!(forward.identity(), reverse.identity());
    assert!(
        OperationLocalSupportDefinition::new(vec![
            ("fact.alpha".into(), alpha.clone()),
            ("fact.alpha".into(), alpha),
        ])
        .is_err()
    );
}
