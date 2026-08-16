// conformance: the standard JVM facade exports the complete public front door.

#![cfg(feature = "standard-jvm")]

#[test]
fn sdk_exports_the_complete_jvm_front_door() {
    let _ = std::any::type_name::<sim::codec_classfile::ClassShell>();
    let _ = sim::lib_lang_jvm::jvm_language_profile();
    let _ = std::any::type_name::<sim::source_authority::SourceAuthority>();
    let _ = std::any::type_name::<sim::lib_lang_jvm::JvmLanguageLib>();
    let _ = std::any::type_name::<sim::lib_lang_jvm::JvmProductSpecimen>();
}
