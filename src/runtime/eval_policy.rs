use std::sync::Arc;

use sim_kernel::{
    Args, CORE_FUNCTION_CLASS_ID, Callable, ClassRef, Cx, EagerPolicy, Error, EvalPolicyRef, Expr,
    HybridPolicy, LazyPolicy, NeedPolicy, Object, QuoteMode, RawArgs, Result, StrictByShapePolicy,
    Symbol, Value,
};

#[derive(Clone)]
pub(crate) struct WithEvalPolicyFunction;

impl Object for WithEvalPolicyFunction {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("#<function core/with-eval-policy>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for WithEvalPolicyFunction {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&Symbol::qualified("core", "Function"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            CORE_FUNCTION_CLASS_ID,
            Symbol::qualified("core", "Function"),
        )
    }

    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for WithEvalPolicyFunction {
    fn call(&self, _cx: &mut Cx, _args: Args) -> Result<Value> {
        Err(Error::Eval(
            "with-eval-policy must be called with unevaluated policy and body".to_owned(),
        ))
    }

    fn call_exprs(&self, cx: &mut Cx, args: RawArgs) -> Result<Value> {
        let args = args.into_exprs();
        let Some((policy_expr, body)) = args.split_first() else {
            return Err(Error::Eval(
                "with-eval-policy expects a policy and body expression".to_owned(),
            ));
        };
        if body.is_empty() {
            return Err(Error::Eval(
                "with-eval-policy expects at least one body expression".to_owned(),
            ));
        }
        let policy = eval_policy_from_expr(policy_expr)?;
        let saved = cx.eval_policy_ref();
        cx.set_eval_policy(policy);
        let result = eval_body(cx, body);
        cx.set_eval_policy(saved);
        result
    }
}

fn eval_body(cx: &mut Cx, body: &[Expr]) -> Result<Value> {
    let mut last = cx.factory().nil()?;
    for expr in body {
        last = cx.eval_expr(expr.clone())?;
    }
    Ok(last)
}

fn eval_policy_from_expr(expr: &Expr) -> Result<EvalPolicyRef> {
    match expr {
        Expr::Symbol(symbol) => eval_policy_from_symbol(symbol),
        Expr::String(value) => eval_policy_from_text(value),
        Expr::Quote {
            mode: QuoteMode::Quote,
            expr,
        } => eval_policy_from_expr(expr),
        _ => Err(Error::Eval(
            "with-eval-policy policy must be a symbol or string".to_owned(),
        )),
    }
}

fn eval_policy_from_symbol(symbol: &Symbol) -> Result<EvalPolicyRef> {
    if let Some(namespace) = &symbol.namespace
        && namespace.as_ref() != "core"
    {
        return Err(Error::Eval(format!(
            "unknown eval policy namespace `{namespace}`"
        )));
    }
    eval_policy_from_name(symbol.name.as_ref())
}

fn eval_policy_from_text(value: &str) -> Result<EvalPolicyRef> {
    let name = value
        .rsplit_once('/')
        .map(|(_, name)| name)
        .unwrap_or(value);
    eval_policy_from_name(name)
}

fn eval_policy_from_name(name: &str) -> Result<EvalPolicyRef> {
    match name {
        "eager" => Ok(Arc::new(EagerPolicy)),
        "lazy" => Ok(Arc::new(LazyPolicy)),
        "lazy-by-need" | "need" => Ok(Arc::new(NeedPolicy)),
        "strict-by-shape" => Ok(Arc::new(StrictByShapePolicy)),
        "hybrid" => Ok(Arc::new(HybridPolicy)),
        other => Err(Error::Eval(format!("unknown eval policy `{other}`"))),
    }
}
