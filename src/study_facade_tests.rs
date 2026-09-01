// conformance: the SDK study feature re-exports the canonical bounded command vocabulary.

#[test]
fn study_facade_preserves_execution_and_write_boundaries() {
    use crate::study::product::StudyVerb;

    assert_eq!(StudyVerb::parse("run"), Some(StudyVerb::Run));
    assert!(StudyVerb::Run.may_execute());
    assert!(!StudyVerb::Report.may_execute());
    assert!(!StudyVerb::Plan.may_write(false));
}
