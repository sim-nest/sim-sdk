use sim::lib_lang_jvm::{
    GeneratedLambdaClassSpace, JavaLambdaCallable, SimFunctionalAdapter,
    executor_admitted_lambda_protocols, jvm_functional_adapter_capability,
};

fn main() {
    // Java-to-SIM projection and SIM-to-Java adaptation are direct public SDK
    // exports. Loader-local class metadata starts empty; no global cache exists.
    let protocols = executor_admitted_lambda_protocols();
    assert_eq!(protocols.len(), 2);
    let _classes = GeneratedLambdaClassSpace::new();

    let _java_to_sim = std::any::type_name::<JavaLambdaCallable>();
    let _sim_to_java = std::any::type_name::<SimFunctionalAdapter>();
    let capability = jvm_functional_adapter_capability();
    assert_eq!(capability.as_str(), "jvm.functional-adapter");
}
