pub use sim_lib_platform::{
    Activation, BoundServices, BundleComposition, BundleContent, BundleManifest, BundleRefusal,
    CapsuleArtifact, CapsuleAttestation, CapsuleManifest, ComposedBundle, ContractProvenance,
    ExecutionEvidence, FactPort, LibraryLoadPlan, Lifecycle, OpenSymbol, PlatformCard,
    PlatformProviderAuthor, PlatformRecordError, PlatformRequest, PlatformSupportRow,
    PureBootEnvelope, RefusalKind, Requirement, RequirementBuilder, ResolutionReceipt,
    ResolutionRefusal, ServiceBinding, ServiceOffer, compose_bundle, platform_require,
    platform_support_matrix,
};

/// SDK entry paths consume the same pure load plan as the bootloader.
#[must_use]
pub fn sdk_load_plan(envelope: &PureBootEnvelope) -> &LibraryLoadPlan {
    &envelope.load_plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_preserves_the_canonical_boot_load_plan() {
        let application = BundleContent {
            id: OpenSymbol("application/portable".into()),
            content_digest: "sha256:portable".into(),
            capabilities: vec![],
        };
        let envelope = PureBootEnvelope {
            schema: OpenSymbol("boot/envelope/v1".into()),
            capsule: OpenSymbol("platform/site/model".into()),
            bootstrap: OpenSymbol("bootstrap/sim-native-abi-v1".into()),
            load_plan: LibraryLoadPlan {
                application: application.clone(),
                libraries: vec![],
            },
        };
        assert_eq!(sdk_load_plan(&envelope).application, application);
    }
}
