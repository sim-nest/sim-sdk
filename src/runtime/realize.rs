use std::time::Duration;

use crate::shapes::{check_shape_value, shape_ref_as_shape};
use sim_kernel::{
    Args, CORE_FUNCTION_CLASS_ID, CORE_LOCAL_EVAL_FABRIC_CLASS_ID, Callable, ClassRef, Consistency,
    Cx, Error, EvalFabric, EvalMode, EvalReply, EvalRequest, Object, RawArgs, Result, ShapeRef,
    Value, eval_fabric_capability, eval_remote_capability,
};

pub(crate) struct LocalEvalFabricObject;

impl Object for LocalEvalFabricObject {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("#<local-eval-fabric>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for LocalEvalFabricObject {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&sim_kernel::Symbol::qualified("core", "LocalEvalFabric"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            CORE_LOCAL_EVAL_FABRIC_CLASS_ID,
            sim_kernel::Symbol::qualified("core", "LocalEvalFabric"),
        )
    }
    fn as_table(&self, cx: &mut Cx) -> Result<Value> {
        cx.factory().table(vec![
            (
                sim_kernel::Symbol::new("kind"),
                cx.factory()
                    .symbol(sim_kernel::Symbol::new("local-fabric"))?,
            ),
            (
                sim_kernel::Symbol::new("boundary-codec"),
                cx.factory()
                    .symbol(sim_kernel::Symbol::qualified("codec", "binary"))?,
            ),
        ])
    }
    fn as_eval_fabric(&self) -> Option<&dyn EvalFabric> {
        Some(self)
    }
}

impl EvalFabric for LocalEvalFabricObject {
    fn realize(&self, cx: &mut Cx, request: EvalRequest) -> Result<EvalReply> {
        if matches!(request.consistency, Consistency::RemoteOnly) {
            return Err(Error::CapabilityDenied {
                capability: eval_remote_capability(),
            });
        }
        for capability in &request.required_capabilities {
            cx.require(capability)?;
        }
        let value = match request.mode {
            EvalMode::Eval => cx.eval_expr(request.expr)?,
            EvalMode::Logic => {
                #[cfg(feature = "logic-core")]
                {
                    crate::lib_logic::realize_logic(
                        cx,
                        request.expr,
                        request.answer_limit,
                        request.stream_buffer,
                        request.stream,
                    )?
                }
                #[cfg(not(feature = "logic-core"))]
                {
                    return Err(Error::Eval(
                        "logic realize mode requires feature logic-core".to_owned(),
                    ));
                }
            }
        };
        if let Some(shape) = &request.result_shape {
            let matched = check_shape_value(cx, shape, value.clone())?;
            if !matched.accepted {
                return Err(Error::WrongShape {
                    expected: shape_ref_as_shape(shape)?
                        .id()
                        .unwrap_or(sim_kernel::ShapeId(0)),
                    diagnostics: matched.diagnostics,
                });
            }
        }
        Ok(EvalReply {
            value,
            diagnostics: cx.take_diagnostics(),
            trace: request
                .trace
                .then(|| cx.factory().symbol(sim_kernel::Symbol::new("local")).ok())
                .flatten(),
        })
    }
}

pub(crate) struct RealizeFunction;

impl Object for RealizeFunction {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok("#<function realize>".to_owned())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for RealizeFunction {
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

impl Callable for RealizeFunction {
    fn call(&self, _cx: &mut Cx, _args: Args) -> Result<Value> {
        Err(Error::Eval(
            "realize must be called with an expression and optional keyword arguments".to_owned(),
        ))
    }

    fn call_exprs(&self, cx: &mut Cx, args: RawArgs) -> Result<Value> {
        cx.require(&eval_fabric_capability())?;

        let args = args.into_exprs();
        let Some((expr, options)) = args.split_first() else {
            return Err(Error::Eval(
                "realize expects an expression argument".to_owned(),
            ));
        };
        let (request, fabric) = build_realize_invocation(cx, expr.clone(), options)?;
        let fabric = fabric.unwrap_or(default_eval_fabric(cx)?);
        let Some(fabric) = fabric.object().as_eval_fabric() else {
            return Err(Error::TypeMismatch {
                expected: "eval-fabric",
                found: "non-eval-fabric",
            });
        };
        let reply = fabric.realize(cx, request)?;
        Ok(reply.value)
    }
}

fn build_realize_invocation(
    cx: &mut Cx,
    expr: sim_kernel::Expr,
    options: &[sim_kernel::Expr],
) -> Result<(EvalRequest, Option<Value>)> {
    if !options.len().is_multiple_of(2) {
        return Err(Error::Eval(
            "realize keyword arguments must be key/value pairs".to_owned(),
        ));
    }

    let mut result_shape = None;
    let mut required_capabilities = Vec::new();
    let mut deadline = None;
    let mut consistency = Consistency::LocalFirst;
    let mut mode = EvalMode::Eval;
    let mut answer_limit = None;
    let mut stream_buffer = None;
    let mut stream = false;
    let mut trace = false;
    let mut fabric = None;

    for pair in options.chunks(2) {
        let key = realize_keyword(&pair[0])?;
        match key {
            "fabric" => {
                fabric = Some(cx.eval_expr(pair[1].clone())?);
            }
            "result" => {
                let value = cx.eval_expr(pair[1].clone())?;
                result_shape = coerce_result_shape(cx, value)?;
            }
            "requires" => {
                let value = cx.eval_expr(pair[1].clone())?;
                required_capabilities = parse_capability_names(cx, value)?;
            }
            "deadline" => {
                let value = cx.eval_expr(pair[1].clone())?;
                deadline = Some(parse_duration_value(cx, value)?);
            }
            "consistency" => {
                let value = cx.eval_expr(pair[1].clone())?;
                consistency = parse_consistency_value(cx, value)?;
            }
            "mode" => {
                let value = cx.eval_expr(pair[1].clone())?;
                mode = parse_mode_value(cx, value)?;
            }
            "answer-limit" => {
                let value = cx.eval_expr(pair[1].clone())?;
                answer_limit = Some(parse_usize_value(cx, value, ":answer-limit")?);
            }
            "buffer" => {
                let value = cx.eval_expr(pair[1].clone())?;
                stream_buffer = Some(parse_usize_value(cx, value, ":buffer")?);
            }
            "stream" => {
                let value = cx.eval_expr(pair[1].clone())?;
                stream = value.object().truth(cx)?;
            }
            "trace" => {
                let value = cx.eval_expr(pair[1].clone())?;
                trace = value.object().truth(cx)?;
            }
            other => {
                return Err(Error::Eval(format!(
                    "realize does not support option :{other}"
                )));
            }
        }
    }

    Ok((
        EvalRequest {
            expr,
            result_shape,
            required_capabilities,
            deadline,
            consistency,
            mode,
            answer_limit,
            stream_buffer,
            stream,
            trace,
        },
        fabric,
    ))
}

fn default_eval_fabric(cx: &mut Cx) -> Result<Value> {
    cx.resolve_value(&sim_kernel::Symbol::qualified("core", "local-fabric"))
        .map_err(|_| Error::Eval("core/local-fabric is not installed".to_owned()))
}

fn realize_keyword(expr: &sim_kernel::Expr) -> Result<&str> {
    let sim_kernel::Expr::Symbol(symbol) = expr else {
        return Err(Error::TypeMismatch {
            expected: "keyword symbol",
            found: "non-symbol",
        });
    };
    Ok(symbol
        .name
        .strip_prefix(':')
        .unwrap_or(symbol.name.as_ref()))
}

fn parse_capability_names(cx: &mut Cx, value: Value) -> Result<Vec<sim_kernel::CapabilityName>> {
    let expr = value.object().as_expr(cx)?;
    match expr {
        sim_kernel::Expr::Nil => Ok(Vec::new()),
        sim_kernel::Expr::List(items) | sim_kernel::Expr::Vector(items) => {
            items.into_iter().map(capability_name_from_expr).collect()
        }
        sim_kernel::Expr::Symbol(_) | sim_kernel::Expr::String(_) => {
            Ok(vec![capability_name_from_expr(expr)?])
        }
        _ => Err(Error::TypeMismatch {
            expected: "capability list",
            found: "non-list",
        }),
    }
}

fn capability_name_from_expr(expr: sim_kernel::Expr) -> Result<sim_kernel::CapabilityName> {
    match expr {
        sim_kernel::Expr::Symbol(symbol) => Ok(sim_kernel::CapabilityName::new(symbol.to_string())),
        sim_kernel::Expr::String(text) => Ok(sim_kernel::CapabilityName::new(text)),
        _ => Err(Error::TypeMismatch {
            expected: "capability symbol or string",
            found: "non-capability",
        }),
    }
}

fn parse_duration_value(cx: &mut Cx, value: Value) -> Result<Duration> {
    match value.object().as_expr(cx)? {
        sim_kernel::Expr::String(text) => parse_duration_text(&text),
        sim_kernel::Expr::Number(number) => {
            let millis = number.canonical.parse::<u64>().map_err(|_| {
                Error::Eval(format!(
                    "deadline {} is not an integer millisecond count",
                    number.canonical
                ))
            })?;
            Ok(Duration::from_millis(millis))
        }
        _ => Err(Error::TypeMismatch {
            expected: "deadline string or integer number",
            found: "non-deadline",
        }),
    }
}

fn parse_duration_text(text: &str) -> Result<Duration> {
    let (number, unit) = if let Some(number) = text.strip_suffix("ms") {
        (number, "ms")
    } else if let Some(number) = text.strip_suffix('s') {
        (number, "s")
    } else if let Some(number) = text.strip_suffix('m') {
        (number, "m")
    } else if let Some(number) = text.strip_suffix('h') {
        (number, "h")
    } else {
        return Err(Error::Eval(format!(
            "deadline {text} must end with ms, s, m, or h"
        )));
    };

    let value = number
        .parse::<u64>()
        .map_err(|_| Error::Eval(format!("deadline {text} has an invalid numeric prefix")))?;
    Ok(match unit {
        "ms" => Duration::from_millis(value),
        "s" => Duration::from_secs(value),
        "m" => Duration::from_secs(value.saturating_mul(60)),
        "h" => Duration::from_secs(value.saturating_mul(60 * 60)),
        _ => unreachable!(),
    })
}

fn parse_consistency_value(cx: &mut Cx, value: Value) -> Result<Consistency> {
    let name = match value.object().as_expr(cx)? {
        sim_kernel::Expr::Symbol(symbol) => symbol.to_string(),
        sim_kernel::Expr::String(text) => text,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "consistency symbol or string",
                found: "non-consistency",
            });
        }
    };
    match name.as_str() {
        "local-only" => Ok(Consistency::LocalOnly),
        "local-first" => Ok(Consistency::LocalFirst),
        "remote-only" => Ok(Consistency::RemoteOnly),
        _ => Err(Error::Eval(format!(
            "unsupported realize consistency {name}"
        ))),
    }
}

fn parse_mode_value(cx: &mut Cx, value: Value) -> Result<EvalMode> {
    let text = match value.object().as_expr(cx)? {
        sim_kernel::Expr::Symbol(symbol) => symbol.to_string(),
        sim_kernel::Expr::String(text) => text,
        _ => {
            return Err(Error::TypeMismatch {
                expected: "eval mode symbol or string",
                found: "non-mode",
            });
        }
    };
    match text.as_str() {
        "eval" => Ok(EvalMode::Eval),
        "logic" => Ok(EvalMode::Logic),
        other => Err(Error::Eval(format!("unsupported realize mode {other}"))),
    }
}

fn parse_usize_value(cx: &mut Cx, value: Value, field: &'static str) -> Result<usize> {
    match value.object().as_expr(cx)? {
        sim_kernel::Expr::Number(number) => number
            .canonical
            .parse::<usize>()
            .map_err(|_| Error::Eval(format!("realize {field} expects a non-negative integer"))),
        sim_kernel::Expr::String(text) => text
            .parse::<usize>()
            .map_err(|_| Error::Eval(format!("realize {field} expects a non-negative integer"))),
        _ => Err(Error::TypeMismatch {
            expected: "usize",
            found: "non-usize",
        }),
    }
}

fn coerce_result_shape(cx: &mut Cx, value: Value) -> Result<Option<ShapeRef>> {
    if matches!(value.object().as_expr(cx)?, sim_kernel::Expr::Nil) {
        return Ok(None);
    }
    if value.object().as_shape().is_some() {
        return Ok(Some(value));
    }
    if let Some(class) = value.object().as_class() {
        return Ok(Some(class.instance_shape(cx)?));
    }
    Err(Error::TypeMismatch {
        expected: "shape or class",
        found: "non-shape",
    })
}
