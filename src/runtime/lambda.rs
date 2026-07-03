use std::sync::Arc;

use sim_kernel::{
    Args, CORE_FUNCTION_CLASS_ID, Callable, ClassRef, Cx, Error, Expr, Object, RawArgs, Result,
    ShapeId, Value,
};
use sim_shape::{
    AnyShape, CaptureShape, FieldShape, FieldSpec, ListShape, Shape, ShapeMatch, parse_shape_expr,
};

#[derive(Clone)]
pub(crate) struct LambdaObject {
    args_shape: Arc<dyn Shape>,
    body: Vec<Expr>,
    env: sim_kernel::Env,
}

impl LambdaObject {
    fn new(args_shape: Arc<dyn Shape>, body: Vec<Expr>, env: sim_kernel::Env) -> Self {
        Self {
            args_shape,
            body,
            env,
        }
    }

    fn match_prepared(&self, cx: &mut Cx, prepared: &[Value]) -> Result<ShapeMatch> {
        let args = cx.new_list(prepared.to_vec())?;
        self.args_shape.check_value(cx, args)
    }

    fn eval_body_with_match(&self, cx: &mut Cx, matched: ShapeMatch) -> Result<Value> {
        let env = cx.with_env(self.env.clone(), |cx| matched.captures.into_child_env(cx))?;
        cx.with_env(env, |cx| {
            let mut last = cx.factory().nil()?;
            for expr in &self.body {
                last = cx.eval_expr(expr.clone())?;
            }
            Ok(last)
        })
    }
}

impl Object for LambdaObject {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("#<lambda>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for LambdaObject {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&sim_kernel::Symbol::qualified("core", "Function"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            CORE_FUNCTION_CLASS_ID,
            sim_kernel::Symbol::qualified("core", "Function"),
        )
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for LambdaObject {
    fn call(&self, cx: &mut Cx, args: Args) -> Result<Value> {
        let matched = self.match_prepared(cx, args.values())?;
        if !matched.accepted {
            return Err(Error::WrongShape {
                expected: self.args_shape.id().unwrap_or(ShapeId(0)),
                diagnostics: matched.diagnostics,
            });
        }
        self.eval_body_with_match(cx, matched)
    }
}

#[derive(Clone)]
pub(crate) struct LambdaBuilder;

impl Object for LambdaBuilder {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("#<function core/lambda>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for LambdaBuilder {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&sim_kernel::Symbol::qualified("core", "Function"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            CORE_FUNCTION_CLASS_ID,
            sim_kernel::Symbol::qualified("core", "Function"),
        )
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for LambdaBuilder {
    fn call(&self, _cx: &mut Cx, _args: Args) -> Result<Value> {
        Err(Error::Eval(
            "lambda must be called with unevaluated params and body".to_owned(),
        ))
    }

    fn call_exprs(&self, cx: &mut Cx, args: RawArgs) -> Result<Value> {
        let args = args.into_exprs();
        let Some((params, body)) = args.split_first() else {
            return Err(Error::Eval(
                "lambda expects a parameter list and at least one body expression".to_owned(),
            ));
        };
        if body.is_empty() {
            return Err(Error::Eval(
                "lambda expects at least one body expression".to_owned(),
            ));
        }
        let args_shape = parse_lambda_params(params)?;
        cx.factory().opaque(Arc::new(LambdaObject::new(
            args_shape,
            body.to_vec(),
            cx.env().clone(),
        )))
    }
}

fn parse_lambda_params(expr: &Expr) -> Result<Arc<dyn Shape>> {
    let Expr::List(items) = expr else {
        return Err(Error::Eval(
            "lambda parameter spec must be a list of parameter shapes".to_owned(),
        ));
    };
    let items = items
        .iter()
        .map(parse_lambda_param)
        .collect::<Result<Vec<_>>>()?;
    Ok(Arc::new(ListShape::new(items)))
}

fn parse_lambda_param(expr: &Expr) -> Result<Arc<dyn Shape>> {
    match expr {
        Expr::Symbol(symbol) => Ok(Arc::new(CaptureShape::new(
            symbol.clone(),
            Arc::new(AnyShape),
        ))),
        Expr::List(items) => {
            let Some(Expr::Symbol(head)) = items.first() else {
                return parse_shape_expr(expr);
            };
            if head.namespace.is_none() && head.name.as_ref() == "capture" {
                return parse_shape_expr(expr);
            }
            if items.len() >= 2 {
                let fields = items
                    .iter()
                    .skip(1)
                    .map(|field| match field {
                        Expr::Symbol(field) => Ok(FieldSpec::required(
                            field.clone(),
                            Arc::new(CaptureShape::new(field.clone(), Arc::new(AnyShape))),
                        )),
                        _ => Err(Error::Eval(
                            "lambda class field destructuring expects symbol field names"
                                .to_owned(),
                        )),
                    })
                    .collect::<Result<Vec<_>>>()?;
                return Ok(Arc::new(FieldShape::new(head.clone(), fields)));
            }
            parse_shape_expr(expr)
        }
        _ => parse_shape_expr(expr),
    }
}
