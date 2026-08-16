use sim::lib_lang_jvm::{
    GeneratedLambdaClassSpace, JavaLambdaCallable, SimFunctionalAdapter,
    executor_admitted_lambda_protocols, jvm_functional_adapter_capability,
};

fn main() {
    assert_eq!(executor_admitted_lambda_protocols().len(), 2);
    let _classes = GeneratedLambdaClassSpace::new();
    assert!(!std::any::type_name::<JavaLambdaCallable>().is_empty());
    assert!(!std::any::type_name::<SimFunctionalAdapter>().is_empty());
    assert_eq!(
        jvm_functional_adapter_capability().as_str(),
        "jvm.functional-adapter"
    );
}
