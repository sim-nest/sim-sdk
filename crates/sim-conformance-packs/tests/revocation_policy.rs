use sim_conformance_core::{
    CheckInputClosureId, CheckScopeId, CheckedSubjectId, CheckerReceipt, CheckerResultClaim,
    CheckerRevocationKey, EvidenceGrade, EvidenceProvenanceId, EvidenceSetId, IdKind, PolicyId,
    RevocationStatus, SemanticId,
};
use sim_conformance_packs::{
    OperationLocalRevocationDecision, find_pack, issue_operation_local_revocation_set,
    operation_local_policy_id,
};

fn sid<K: IdKind>(value: &str) -> SemanticId<K> {
    SemanticId::from_text(value).unwrap()
}

#[test]
fn owner_snapshot_is_explicit_fail_closed_and_rechecked_on_reuse() {
    let spec = find_pack("checker/c-op").unwrap();
    let binding = spec.checker_binding().unwrap();
    let code = spec.checker_code_id().unwrap();
    let subject_a: CheckedSubjectId = sid("subject/a");
    let subject_b: CheckedSubjectId = sid("subject/b");
    let scope: CheckScopeId = sid("operation/local");
    let closure: CheckInputClosureId = sid("closure/exact");
    let invocation = |subject: CheckedSubjectId| {
        binding
            .instantiate(
                code.clone(),
                spec.pack_id().unwrap(),
                subject,
                scope.clone(),
                closure.clone(),
            )
            .unwrap()
    };
    let invocation_a = invocation(subject_a.clone());
    let invocation_b = invocation(subject_b.clone());
    let first = issue_operation_local_revocation_set(
        sid("owner-head/one"),
        [
            OperationLocalRevocationDecision::current(subject_a.clone()),
            OperationLocalRevocationDecision::current(subject_b.clone()),
        ],
    )
    .unwrap();
    let key_a =
        CheckerRevocationKey::for_invocation(first.source().clone(), &invocation_a).unwrap();
    let key_b =
        CheckerRevocationKey::for_invocation(first.source().clone(), &invocation_b).unwrap();
    let result = sid("result/pass");
    let provenance: EvidenceProvenanceId = sid("provenance/held-output");
    let policy: PolicyId = operation_local_policy_id().unwrap();
    let support: EvidenceSetId = sid("support/exact");
    let issue = |invocation, key: &CheckerRevocationKey| {
        CheckerReceipt::passing(
            &binding,
            invocation,
            result.clone(),
            EvidenceGrade::Reproducible,
            provenance.clone(),
            policy.clone(),
            support.clone(),
            &first.lookup(key),
        )
        .unwrap()
    };
    let receipt_a = issue(&invocation_a, &key_a);
    let receipt_b = issue(&invocation_b, &key_b);

    let second = issue_operation_local_revocation_set(
        sid("owner-head/two"),
        [
            OperationLocalRevocationDecision::revoked(subject_a),
            OperationLocalRevocationDecision::current(subject_b),
        ],
    )
    .unwrap();
    let expected = CheckerResultClaim {
        result: &result,
        minimum_grade: EvidenceGrade::Reproducible,
        provenance: &provenance,
        policy: &policy,
        support: &support,
        revocation_set: second.id(),
        revocation_head: second.head(),
    };
    assert_eq!(second.lookup(&key_a).status(), RevocationStatus::Revoked);
    assert!(
        receipt_a
            .verify_claim(
                &binding,
                &invocation_a,
                &expected,
                &second.lookup(&key_a).bind_receipt(receipt_a.id().clone()),
            )
            .is_err()
    );
    receipt_b
        .verify_claim(
            &binding,
            &invocation_b,
            &expected,
            &second.lookup(&key_b).bind_receipt(receipt_b.id().clone()),
        )
        .unwrap();

    let missing = issue_operation_local_revocation_set(sid("owner-head/three"), []).unwrap();
    assert_eq!(missing.lookup(&key_b).status(), RevocationStatus::Unknown);
}
