use sim::lib_control::{AdmissionLimit, WorkLimit};
use sim::lib_lang_javascript::{
    Completion, JavascriptEvalPolicy, JavascriptJobs, JavascriptPromise,
    JavascriptPromiseState, JavascriptState, JavascriptValue, javascript_fidelity_dimensions,
};

pub fn bounded_javascript_module() -> Result<(), Box<dyn std::error::Error>> {
    let tree = sim::codec_javascript::parse_module("const answer = 40 + 2; answer;")?;
    let lowered = sim::codec_javascript::lower_javascript(&tree);
    let result = JavascriptEvalPolicy::new(128)?
        .eval_lowered(&lowered, &mut JavascriptState::default())?;
    assert_eq!(result, Completion::Normal(JavascriptValue::Number(42.0)));

    let promise = JavascriptPromise::default();
    let mut jobs = JavascriptJobs::new(AdmissionLimit(1));
    promise
        .resolve(&mut jobs, JavascriptValue::Number(42.0))
        .map_err(|error| std::io::Error::other(format!("promise admission failed: {error:?}")))?;
    assert_eq!(promise.state(), JavascriptPromiseState::Pending);
    let receipt = jobs
        .microtask_checkpoint(WorkLimit(1))
        .map_err(|error| std::io::Error::other(format!("microtask drain failed: {error:?}")))?;
    assert_eq!(receipt.completed.len(), 1);
    assert_eq!(promise.state(), JavascriptPromiseState::Fulfilled(JavascriptValue::Number(42.0)));

    let gaps = javascript_fidelity_dimensions().iter()
        .find(|dimension| dimension.name == "expected-gaps")
        .expect("fidelity publishes expected gaps");
    assert!(gaps.evidence.contains("Node"));
    assert!(javascript_fidelity_dimensions().iter().any(|dimension| {
        dimension.name == "direct-evaluation" && dimension.evidence.contains("no compiler")
    }));
    Ok(())
}
