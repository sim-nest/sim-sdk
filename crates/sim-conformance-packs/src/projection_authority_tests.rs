use sim_conformance_core::{
    CheckScopeId, CheckerResultId, EvidenceGrade, IdKind, LiveCheckerError, RevocationStatus,
    SemanticId,
};
use sim_kernel::{ContentId, Datum, Symbol};

use super::*;
use crate::projection_authority::{
    ProjectionAdmissionAuthorityError, ProjectionAdmissionOwner, projection_check_inputs,
    projection_provenance_id_for_test, projection_support_id_for_test,
    verified_producer_records_for_test,
};

fn sid<K: IdKind>(value: &str) -> SemanticId<K> {
    SemanticId::from_text(value).unwrap()
}

fn id(value: &str) -> ContentId {
    Datum::String(value.to_owned()).content_id().unwrap()
}

fn evidence(value: &str) -> OperationLocalEvidence {
    let facts = crate::packs::boundary::PROJECTION_ADMISSION_FACTS
        .into_iter()
        .fold(MemorySubject::default(), |subject, key| {
            subject.with(key, value)
        });
    OperationLocalEvidence::capture(&facts).unwrap()
}

fn support(suffix: &str) -> OperationLocalSupportDefinition {
    OperationLocalSupportDefinition::new(
        crate::packs::boundary::PROJECTION_ADMISSION_FACTS
            .into_iter()
            .map(|name| (name.to_owned(), id(&format!("support/{name}/{suffix}")))),
    )
    .unwrap()
}

fn identity_fixture(suffix: &str) -> (OperationLocalEvidence, OperationLocalSupportDefinition) {
    let support = support(suffix);
    let evidence = support.identity_evidence().unwrap();
    (evidence, support)
}

fn owner(byte: u8) -> ProjectionAdmissionOwner {
    ProjectionAdmissionOwner::boot(ContentId::from_bytes(
        Symbol::qualified("projection", "test-owner-generation"),
        [byte; 32],
    ))
    .unwrap()
}

fn expected_result(
    invocation: &sim_conformance_core::CheckInvocation,
    evidence: &OperationLocalEvidence,
) -> CheckerResultId {
    let binding_text = render(invocation.binding().content_id());
    let subject_text = render(invocation.subject().content_id());
    match packs::boundary::check(&PackRequest {
        checker: "checker/c-boundary",
        binding: &binding_text,
        subject: &subject_text,
        scope: "boundary/projection-admission",
        evidence,
    }) {
        PackVerdict::Pass { result, .. } => result,
        verdict => panic!("expected passing projection fixture, got {verdict:?}"),
    }
}

fn producer_records(
    invocation: &sim_conformance_core::CheckInvocation,
    evidence: &OperationLocalEvidence,
    support_definition: &OperationLocalSupportDefinition,
) -> VerifiedProjectionProducerRecords {
    verified_producer_records_for_test(
        invocation,
        expected_result(invocation, evidence),
        EvidenceGrade::Reproducible,
        projection_provenance_id_for_test(invocation, support_definition),
        projection_support_id_for_test(invocation, support_definition),
        support_definition,
    )
}

fn issue(
    owner: &ProjectionAdmissionOwner,
    evidence: &OperationLocalEvidence,
    support_definition: &OperationLocalSupportDefinition,
) -> QualifiedProjectionAdmissionCheck {
    let authority = owner.authority();
    let invocation = authority.invocation(evidence, support_definition).unwrap();
    let records = producer_records(&invocation, evidence, support_definition);
    owner.mark_current(&invocation).unwrap();
    owner
        .check_and_issue(invocation, evidence, support_definition, records)
        .unwrap()
}

#[test]
fn owner_issues_exact_receipt_and_reobserves_currentness() {
    let owner = owner(1);
    let authority = owner.authority();
    let (evidence, support) = identity_fixture("exact");
    let qualified = issue(&owner, &evidence, &support);

    assert_eq!(
        qualified.binding(),
        &find_pack("checker/c-boundary")
            .unwrap()
            .native_projection_binding()
            .unwrap()
    );
    assert_eq!(
        qualified.invocation().subject(),
        qualified.receipt().subject()
    );
    assert_eq!(qualified.invocation().scope(), qualified.receipt().scope());
    assert_eq!(
        qualified.invocation().id(),
        qualified.receipt().invocation()
    );
    assert_eq!(
        qualified.receipt().result(),
        &expected_result(qualified.invocation(), &evidence)
    );
    assert_eq!(qualified.receipt().grade(), EvidenceGrade::Reproducible);
    assert_eq!(
        qualified.receipt().provenance(),
        &projection_provenance_id_for_test(qualified.invocation(), &support)
    );
    assert_eq!(
        qualified.receipt().support(),
        &projection_support_id_for_test(qualified.invocation(), &support)
    );
    assert_eq!(
        qualified.issue_observation().receipt(),
        qualified.receipt().id()
    );
    assert_eq!(
        qualified.issue_observation().status(),
        RevocationStatus::Current
    );
    assert_eq!(qualified.support_definition(), support.identity());
    assert_eq!(qualified.observations().len(), 22);
    assert!(qualified.observations().iter().all(|observation| {
        support
            .members()
            .get(&observation.key)
            .is_some_and(|identity| observation.value == render(identity))
    }));
    qualified.verify_current(&authority).unwrap();
    qualified.while_current(&authority, || ()).unwrap();

    owner.revoke(qualified.invocation()).unwrap();
    assert!(qualified.verify_current(&authority).is_err());
}

#[test]
fn boolean_or_malformed_identity_evidence_is_refused() {
    let owner = owner(11);
    let authority = owner.authority();
    let (exact_evidence, exact_support) = identity_fixture("exact");
    let exact_invocation = authority
        .invocation(&exact_evidence, &exact_support)
        .unwrap();
    let boolean_evidence = evidence("true");

    assert!(matches!(
        authority.invocation(&boolean_evidence, &exact_support),
        Err(ProjectionAdmissionAuthorityError::Unsupported)
    ));

    let binding_text = render(exact_invocation.binding().content_id());
    let subject_text = render(exact_invocation.subject().content_id());
    assert!(matches!(
        packs::boundary::check(&PackRequest {
            checker: "checker/c-boundary",
            binding: &binding_text,
            subject: &subject_text,
            scope: "boundary/projection-admission",
            evidence: &boolean_evidence,
        }),
        PackVerdict::Refused(PackFailure {
            code: "malformed-content-identity",
            ..
        })
    ));
}

#[test]
fn missing_extra_duplicate_and_substituted_support_is_refused() {
    let owner = owner(12);
    let authority = owner.authority();
    let (exact_evidence, exact_support) = identity_fixture("exact");

    let missing = OperationLocalSupportDefinition::new(
        exact_support
            .members()
            .iter()
            .skip(1)
            .map(|(name, identity)| (name.clone(), identity.clone())),
    )
    .unwrap();
    let missing_evidence = missing.identity_evidence().unwrap();
    assert!(matches!(
        authority.invocation(&missing_evidence, &missing),
        Err(ProjectionAdmissionAuthorityError::Unsupported)
    ));

    let extra = OperationLocalSupportDefinition::new(
        exact_support
            .members()
            .iter()
            .map(|(name, identity)| (name.clone(), identity.clone()))
            .chain([(String::from("z.extra-owner"), id("extra-owner"))]),
    )
    .unwrap();
    let extra_evidence = extra.identity_evidence().unwrap();
    assert!(matches!(
        authority.invocation(&extra_evidence, &extra),
        Err(ProjectionAdmissionAuthorityError::Unsupported)
    ));

    let duplicate_name = crate::packs::boundary::PROJECTION_ADMISSION_FACTS[0].to_owned();
    assert!(matches!(
        OperationLocalSupportDefinition::new([
            (duplicate_name.clone(), id("first")),
            (duplicate_name.clone(), id("second")),
        ]),
        Err(OperationLocalSupportError::DuplicateMember(name)) if name == duplicate_name
    ));

    let duplicated_identity = id("duplicated-owner-identity");
    let duplicated_identity_support =
        OperationLocalSupportDefinition::new(exact_support.members().iter().enumerate().map(
            |(index, (name, identity))| {
                (
                    name.clone(),
                    if index < 2 {
                        duplicated_identity.clone()
                    } else {
                        identity.clone()
                    },
                )
            },
        ))
        .unwrap();
    let duplicated_identity_evidence = duplicated_identity_support.identity_evidence().unwrap();
    assert!(matches!(
        authority.invocation(&duplicated_identity_evidence, &duplicated_identity_support,),
        Err(ProjectionAdmissionAuthorityError::Unsupported)
    ));

    let substituted_support = support("substituted");
    let substituted_evidence = substituted_support.identity_evidence().unwrap();
    let exact_invocation = authority
        .invocation(&exact_evidence, &exact_support)
        .unwrap();
    let exact_records = producer_records(&exact_invocation, &exact_evidence, &exact_support);
    owner.mark_current(&exact_invocation).unwrap();
    assert!(matches!(
        owner.check_and_issue(
            exact_invocation,
            &substituted_evidence,
            &substituted_support,
            exact_records,
        ),
        Err(ProjectionAdmissionAuthorityError::Unsupported)
    ));
}

#[test]
fn wrong_subject_scope_and_result_are_refused() {
    let owner = owner(2);
    let authority = owner.authority();
    let (exact_evidence, exact_support) = identity_fixture("exact");
    let invocation = authority
        .invocation(&exact_evidence, &exact_support)
        .unwrap();
    owner.mark_current(&invocation).unwrap();

    let (changed_evidence, changed_support) = identity_fixture("changed");
    let changed_records = producer_records(&invocation, &exact_evidence, &exact_support);
    assert!(matches!(
        owner.check_and_issue(
            invocation.clone(),
            &changed_evidence,
            &changed_support,
            changed_records,
        ),
        Err(ProjectionAdmissionAuthorityError::Unsupported)
    ));

    let spec = find_pack("checker/c-boundary").unwrap();
    let command_binding = spec.checker_binding().unwrap();
    let inputs = projection_check_inputs(&exact_evidence, &exact_support).unwrap();
    let wrong_scope = command_binding
        .instantiate(
            spec.checker_code_id().unwrap(),
            spec.pack_id().unwrap(),
            inputs.subject().clone(),
            CheckScopeId::from_text("boundary/inventory").unwrap(),
            inputs.invocation_input().clone(),
        )
        .unwrap();
    let wrong_scope_records = verified_producer_records_for_test(
        &wrong_scope,
        expected_result(&invocation, &exact_evidence),
        EvidenceGrade::Reproducible,
        sid("provenance/wrong-scope"),
        sid("support/wrong-scope"),
        &exact_support,
    );
    assert!(matches!(
        owner.check_and_issue(
            wrong_scope,
            &exact_evidence,
            &exact_support,
            wrong_scope_records,
        ),
        Err(ProjectionAdmissionAuthorityError::Unsupported)
    ));

    let wrong_result_records = verified_producer_records_for_test(
        &invocation,
        sid("result/forged"),
        EvidenceGrade::Reproducible,
        sid("provenance/exact"),
        projection_support_id_for_test(&invocation, &exact_support),
        &exact_support,
    );
    assert!(matches!(
        owner.check_and_issue(
            invocation,
            &exact_evidence,
            &exact_support,
            wrong_result_records,
        ),
        Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
            "result"
        ))
    ));
}

#[test]
fn forged_or_substituted_support_and_bootstrap_grade_are_refused() {
    let owner = owner(3);
    let authority = owner.authority();
    let (evidence, exact_support) = identity_fixture("exact");
    let substituted_support = support("substituted");
    let invocation = authority.invocation(&evidence, &exact_support).unwrap();
    owner.mark_current(&invocation).unwrap();

    let substituted_records = producer_records(&invocation, &evidence, &exact_support);
    assert!(matches!(
        owner.check_and_issue(
            invocation.clone(),
            &evidence,
            &substituted_support,
            substituted_records,
        ),
        Err(ProjectionAdmissionAuthorityError::Unsupported)
    ));

    let forged_provenance = verified_producer_records_for_test(
        &invocation,
        expected_result(&invocation, &evidence),
        EvidenceGrade::Reproducible,
        sid("provenance/forged"),
        projection_support_id_for_test(&invocation, &exact_support),
        &exact_support,
    );
    assert!(matches!(
        owner.check_and_issue(
            invocation.clone(),
            &evidence,
            &exact_support,
            forged_provenance,
        ),
        Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
            "provenance"
        ))
    ));

    let forged_support = verified_producer_records_for_test(
        &invocation,
        expected_result(&invocation, &evidence),
        EvidenceGrade::Reproducible,
        projection_provenance_id_for_test(&invocation, &exact_support),
        sid("support/forged"),
        &exact_support,
    );
    assert!(matches!(
        owner.check_and_issue(
            invocation.clone(),
            &evidence,
            &exact_support,
            forged_support,
        ),
        Err(ProjectionAdmissionAuthorityError::ProducerMismatch(
            "support"
        ))
    ));

    let bootstrap = verified_producer_records_for_test(
        &invocation,
        expected_result(&invocation, &evidence),
        EvidenceGrade::Bootstrap,
        sid("provenance/bootstrap"),
        projection_support_id_for_test(&invocation, &exact_support),
        &exact_support,
    );
    assert!(matches!(
        owner.check_and_issue(invocation, &evidence, &exact_support, bootstrap),
        Err(ProjectionAdmissionAuthorityError::ProducerMismatch("grade"))
    ));
}

#[test]
fn owner_death_and_foreign_generation_invalidate_qualified_result() {
    let first = owner(4);
    let first_authority = first.authority();
    let (evidence, support) = identity_fixture("exact");
    let qualified = issue(&first, &evidence, &support);

    let second = owner(5);
    assert!(qualified.verify_current(&second.authority()).is_err());
    drop(first);
    assert!(matches!(
        first_authority.ensure_live(),
        Err(ProjectionAdmissionAuthorityError::Authority(
            LiveCheckerError::Unavailable
        ))
    ));
    assert!(qualified.verify_current(&first_authority).is_err());
}

#[test]
fn unauthorized_scope_cannot_be_instantiated_on_native_binding() {
    let spec = find_pack("checker/c-boundary").unwrap();
    let binding = spec.native_projection_binding().unwrap();
    assert!(
        binding
            .instantiate(
                spec.checker_code_id().unwrap(),
                spec.pack_id().unwrap(),
                sid("subject/exact"),
                CheckScopeId::from_text("operation/local").unwrap(),
                sid("input/exact"),
            )
            .is_err()
    );
}

fn render(id: &ContentId) -> String {
    let digest = id
        .bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}:{digest}", id.algorithm.as_qualified_str())
}
