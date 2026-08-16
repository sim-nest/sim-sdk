use sim::{codec_classfile, lib_lang_jvm, source_authority};

fn main() {
    let _decoder = std::any::type_name::<codec_classfile::ClassShell>();
    let _authority = std::any::type_name::<source_authority::SourceAuthority>();
    let _profile = lib_lang_jvm::jvm_language_profile();
    let _library = lib_lang_jvm::JvmLanguageLib::default();
    let _invoke = lib_lang_jvm::jvm_invoke_capability();
    let _specimen = lib_lang_jvm::run_product_specimen;
}
