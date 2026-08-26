// conformance: the estate facade exposes portable identities without selecting a provider.

#[test]
fn estate_facade_preserves_portable_project_identity() {
    let first = crate::estate::core::ProjectFingerprint::of(b"portable-estate");
    let second = crate::estate::core::ProjectFingerprint::of(b"portable-estate");
    assert_eq!(first, second);
}
