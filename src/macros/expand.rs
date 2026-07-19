use std::sync::Arc;

use sim_kernel::{
    Cx, Demand, Diagnostic, Error, Expr, MacroExpander, MacroId, Phase, PreparedArgs, Result,
    Symbol, Value,
};
use sim_shape::{AnyShape, CaptureShape, ExactExprShape, ListShape, Shape};

use crate::{
    functions::{FunctionCase, FunctionObject},
    macros::{
        LispMacro, MacroCx, MacroExpansionLimits, MacroObject, macro_value_with_parser_trust,
    },
};

/// Registers a macro in the context, trusting its parser, and returns its id.
pub fn register_macro(cx: &mut Cx, mac: Arc<dyn LispMacro>) -> Result<MacroId> {
    register_macro_with_parser_trust(cx, mac, true)
}

/// Registers a macro with an explicit parser-trust flag and returns its id.
pub fn register_macro_with_parser_trust(
    cx: &mut Cx,
    mac: Arc<dyn LispMacro>,
    parser_trusted: bool,
) -> Result<MacroId> {
    let symbol = mac.symbol();
    cx.registry_mut()
        .register_macro_value(symbol, macro_value_with_parser_trust(mac, parser_trusted))
}

/// Macro expander that resolves macros through the context registry, bounded by
/// configurable expansion limits.
pub struct RegistryMacroExpander {
    limits: MacroExpansionLimits,
}

impl RegistryMacroExpander {
    /// Creates an expander with the default expansion limits.
    pub fn new() -> Self {
        Self {
            limits: MacroExpansionLimits::default(),
        }
    }

    /// Creates an expander with the given expansion limits.
    pub fn with_limits(limits: MacroExpansionLimits) -> Self {
        Self { limits }
    }
}

impl Default for RegistryMacroExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroExpander for RegistryMacroExpander {
    fn expand_expr(&self, cx: &mut Cx, phase: Phase, expr: Expr) -> Result<Expr> {
        let mut macro_cx = MacroCx::with_limits(cx, phase, self.limits);
        expand_expr(&mut macro_cx, expr)
    }
}

/// Fully expands macros in an expression within a macro context.
pub fn expand_expr(cx: &mut MacroCx<'_>, expr: Expr) -> Result<Expr> {
    expand_expr_with_depth(cx, expr, 0)
}

fn expand_expr_with_depth(cx: &mut MacroCx<'_>, expr: Expr, depth: usize) -> Result<Expr> {
    cx.charge(1)?;
    if depth > cx.max_depth() {
        return Err(cx.budget_error(format!(
            "macro expansion exceeded depth limit of {}",
            cx.max_depth()
        )));
    }

    match expr {
        Expr::List(items) => expand_list(cx, items, depth),
        Expr::Call { operator, args } => expand_call(cx, *operator, args, depth),
        Expr::Vector(items) => Ok(Expr::Vector(expand_many(cx, items, depth)?)),
        Expr::Map(entries) => Ok(Expr::Map(
            entries
                .into_iter()
                .map(|(key, value)| {
                    Ok((
                        expand_expr_with_depth(cx, key, depth)?,
                        expand_expr_with_depth(cx, value, depth)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?,
        )),
        Expr::Set(items) => Ok(Expr::Set(expand_many(cx, items, depth)?)),
        Expr::Infix {
            operator,
            left,
            right,
        } => Ok(Expr::Infix {
            operator,
            left: Box::new(expand_expr_with_depth(cx, *left, depth)?),
            right: Box::new(expand_expr_with_depth(cx, *right, depth)?),
        }),
        Expr::Prefix { operator, arg } => Ok(Expr::Prefix {
            operator,
            arg: Box::new(expand_expr_with_depth(cx, *arg, depth)?),
        }),
        Expr::Postfix { operator, arg } => Ok(Expr::Postfix {
            operator,
            arg: Box::new(expand_expr_with_depth(cx, *arg, depth)?),
        }),
        Expr::Block(items) => Ok(Expr::Block(expand_many(cx, items, depth)?)),
        Expr::Annotated { expr, annotations } => Ok(Expr::Annotated {
            expr: Box::new(expand_expr_with_depth(cx, *expr, depth)?),
            annotations: annotations
                .into_iter()
                .map(|(symbol, value)| Ok((symbol, expand_expr_with_depth(cx, value, depth)?)))
                .collect::<Result<Vec<_>>>()?,
        }),
        Expr::Extension { tag, payload } => Ok(Expr::Extension {
            tag,
            payload: Box::new(expand_expr_with_depth(cx, *payload, depth)?),
        }),
        Expr::Quote { .. }
        | Expr::Nil
        | Expr::Bool(_)
        | Expr::Number(_)
        | Expr::Symbol(_)
        | Expr::Local(_)
        | Expr::String(_)
        | Expr::Bytes(_) => Ok(expr),
    }
}

fn expand_list(cx: &mut MacroCx<'_>, items: Vec<Expr>, depth: usize) -> Result<Expr> {
    let Some(Expr::Symbol(symbol)) = items.first() else {
        return Ok(Expr::List(expand_many(cx, items, depth)?));
    };
    let Some(value) = cx.cx.registry().macro_by_symbol(symbol).cloned() else {
        return Ok(Expr::List(expand_many(cx, items, depth)?));
    };
    expand_macro_form(cx, value, Expr::List(items), depth)
}

fn expand_call(
    cx: &mut MacroCx<'_>,
    operator: Expr,
    args: Vec<Expr>,
    depth: usize,
) -> Result<Expr> {
    if let Expr::Symbol(symbol) = &operator
        && let Some(value) = cx.cx.registry().macro_by_symbol(symbol).cloned()
    {
        let input = Expr::List(std::iter::once(operator).chain(args).collect::<Vec<_>>());
        return expand_macro_form(cx, value, input, depth);
    }

    Ok(Expr::Call {
        operator: Box::new(expand_expr_with_depth(cx, operator, depth)?),
        args: expand_many(cx, args, depth)?,
    })
}

fn expand_macro_form(
    cx: &mut MacroCx<'_>,
    value: Value,
    input: Expr,
    depth: usize,
) -> Result<Expr> {
    let symbol = macro_head_symbol(&input);
    if !cx.cx.eval_policy().allow_macro_expansion(cx.phase()) {
        let name = symbol
            .as_ref()
            .map(Symbol::to_string)
            .unwrap_or_else(|| "<unknown>".to_owned());
        return Err(Error::Eval(format!(
            "macro expansion for {name} is not allowed during {:?} by {} eval policy",
            cx.phase(),
            cx.cx.eval_policy_name()
        )));
    }
    cx.cx.require(&macro_phase_capability(cx.phase()))?;

    let mac = ResolvedMacro::from_value(&value).ok_or(Error::TypeMismatch {
        expected: "macro object",
        found: "non-macro object",
    })?;
    let shape = mac.syntax_shape();
    if shape.is_effectful() && !mac.parser_trusted() {
        return Err(Error::Eval(format!(
            "macro {} uses an effectful syntax shape in an untrusted parse position",
            mac.symbol()
        )));
    }
    let macro_symbol = mac.symbol();
    let (matched, expansion_input) = match shape.check_expr(cx.cx, &input)? {
        matched if matched.accepted => (matched, input),
        rejected => {
            let Some(alias) = symbol.as_ref() else {
                return Err(wrong_macro_shape(
                    cx,
                    &macro_symbol,
                    shape.as_ref(),
                    rejected,
                ));
            };
            if alias == &macro_symbol {
                return Err(wrong_macro_shape(
                    cx,
                    &macro_symbol,
                    shape.as_ref(),
                    rejected,
                ));
            }
            let normalized = retarget_macro_head(input.clone(), macro_symbol.clone());
            match shape.check_expr(cx.cx, &normalized)? {
                matched if matched.accepted => (matched, normalized),
                _ => {
                    return Err(wrong_macro_shape(
                        cx,
                        &macro_symbol,
                        shape.as_ref(),
                        rejected,
                    ));
                }
            }
        }
    };
    if !matched.accepted {
        return Err(wrong_macro_shape(
            cx,
            &macro_symbol,
            shape.as_ref(),
            matched,
        ));
    }

    let symbol = symbol.unwrap_or(macro_symbol);
    cx.stack.push(symbol);
    let expanded = mac.expand(cx, expansion_input, matched.captures);
    let expanded = match expanded {
        Ok(expanded) => expanded,
        Err(error) => {
            cx.stack.pop();
            return Err(error);
        }
    };
    let result = expand_expr_with_depth(cx, expanded, depth + 1);
    cx.stack.pop();
    result
}

enum ResolvedMacro<'a> {
    Sdk(&'a MacroObject),
    #[cfg(all(feature = "codec-lisp", feature = "shape"))]
    LoaderSource(&'a sim_run_loaders::SourceTemplateMacro),
    #[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
    Native(&'a sim_run_loaders::NativeAbiMacro),
}

impl<'a> ResolvedMacro<'a> {
    fn from_value(value: &'a Value) -> Option<Self> {
        if let Some(mac) = value.object().downcast_ref::<MacroObject>() {
            return Some(Self::Sdk(mac));
        }
        #[cfg(all(feature = "codec-lisp", feature = "shape"))]
        if let Some(mac) = value
            .object()
            .downcast_ref::<sim_run_loaders::SourceTemplateMacro>()
        {
            return Some(Self::LoaderSource(mac));
        }
        #[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
        if let Some(mac) = value
            .object()
            .downcast_ref::<sim_run_loaders::NativeAbiMacro>()
        {
            return Some(Self::Native(mac));
        }
        None
    }

    fn symbol(&self) -> Symbol {
        match self {
            Self::Sdk(mac) => mac.macro_ref().symbol(),
            #[cfg(all(feature = "codec-lisp", feature = "shape"))]
            Self::LoaderSource(mac) => mac.symbol(),
            #[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
            Self::Native(mac) => mac.symbol(),
        }
    }

    fn syntax_shape(&self) -> Arc<dyn Shape> {
        match self {
            Self::Sdk(mac) => mac.syntax_shape(),
            #[cfg(all(feature = "codec-lisp", feature = "shape"))]
            Self::LoaderSource(mac) => mac.syntax_shape(),
            #[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
            Self::Native(mac) => mac.syntax_shape(),
        }
    }

    fn parser_trusted(&self) -> bool {
        match self {
            Self::Sdk(mac) => mac.parser_trusted(),
            #[cfg(all(feature = "codec-lisp", feature = "shape"))]
            Self::LoaderSource(mac) => mac.parser_trusted(),
            #[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
            Self::Native(mac) => mac.parser_trusted(),
        }
    }

    fn expand(
        &self,
        cx: &mut MacroCx<'_>,
        input: Expr,
        captures: sim_shape::Bindings,
    ) -> Result<Expr> {
        match self {
            Self::Sdk(mac) => mac.macro_ref().expand(cx, input, captures),
            #[cfg(all(feature = "codec-lisp", feature = "shape"))]
            Self::LoaderSource(mac) => {
                let _ = cx;
                mac.expand(input, captures)
            }
            #[cfg(all(feature = "dynamic-native", not(target_arch = "wasm32")))]
            Self::Native(mac) => {
                let _ = captures;
                mac.expand(input)
            }
        }
    }
}

fn macro_head_symbol(input: &Expr) -> Option<Symbol> {
    match input {
        Expr::List(items) => items.first().and_then(|item| match item {
            Expr::Symbol(symbol) => Some(symbol.clone()),
            _ => None,
        }),
        _ => None,
    }
}

fn retarget_macro_head(input: Expr, target: Symbol) -> Expr {
    match input {
        Expr::List(mut items) if !items.is_empty() => {
            items[0] = Expr::Symbol(target);
            Expr::List(items)
        }
        other => other,
    }
}

fn wrong_macro_shape(
    cx: &mut MacroCx<'_>,
    macro_symbol: &Symbol,
    shape: &dyn Shape,
    matched: sim_shape::ShapeMatch,
) -> Error {
    let mut diagnostics = vec![Diagnostic::error(format!(
        "macro {} rejected syntax during {:?}",
        macro_symbol,
        cx.phase(),
    ))];
    if let Ok(doc) = shape.describe(cx.cx) {
        diagnostics.push(Diagnostic::error(format!("syntax shape: {}", doc.name)));
        diagnostics.extend(
            doc.details
                .into_iter()
                .map(|detail| Diagnostic::error(format!("syntax detail: {detail}"))),
        );
    }
    diagnostics.extend(matched.diagnostics);
    Error::WrongShape {
        expected: shape.id().unwrap_or(sim_kernel::ShapeId(0)),
        diagnostics,
    }
}

fn macro_phase_capability(phase: Phase) -> sim_kernel::CapabilityName {
    match phase {
        Phase::Read => sim_kernel::macro_expand_read_capability(),
        Phase::Expand => sim_kernel::macro_expand_capability(),
        Phase::Compile => sim_kernel::macro_expand_compile_capability(),
        Phase::Eval => sim_kernel::macro_expand_eval_capability(),
    }
}

fn expand_many(cx: &mut MacroCx<'_>, items: Vec<Expr>, depth: usize) -> Result<Vec<Expr>> {
    items
        .into_iter()
        .map(|item| expand_expr_with_depth(cx, item, depth))
        .collect()
}

/// Builds the `macroexpand` function object that expands a quoted expression.
pub fn macroexpand_function(
    case_id: sim_kernel::CaseId,
    function_id: sim_kernel::FunctionId,
    symbol: Symbol,
) -> FunctionObject {
    FunctionObject::new(
        function_id,
        symbol.clone(),
        vec![FunctionCase {
            id: case_id,
            name: Symbol::qualified(symbol.to_string(), "expr"),
            args: Arc::new(ListShape::new(vec![Arc::new(CaptureShape::new(
                Symbol::new("expr"),
                Arc::new(AnyShape),
            ))])),
            result: Some(Arc::new(AnyShape)),
            demand: vec![Demand::Value],
            priority: 10,
            implementation: macroexpand_impl,
        }],
    )
}

fn macroexpand_impl(
    cx: &mut Cx,
    prepared: &PreparedArgs,
    _bindings: sim_shape::Bindings,
) -> Result<Value> {
    let expr = prepared
        .get(0)
        .ok_or_else(|| Error::Eval("macroexpand expects one expression".to_owned()))?
        .object()
        .as_expr(cx)?;
    let expanded = cx.expand_macros(Phase::Expand, expr)?;
    cx.factory().expr(expanded)
}

/// Builds a shape matching an exact head symbol, for macro syntax.
pub fn literal_head_shape(symbol: Symbol) -> Arc<dyn Shape> {
    Arc::new(ExactExprShape::new(Expr::Symbol(symbol)))
}

/// Builds a list shape with a fixed head symbol followed by the given tail.
pub fn list_macro_shape(head: Symbol, tail: Vec<Arc<dyn Shape>>) -> Arc<dyn Shape> {
    let items = std::iter::once(literal_head_shape(head))
        .chain(tail)
        .collect::<Vec<_>>();
    Arc::new(ListShape::new(items))
}

/// Builds a list shape with a fixed head and tail plus a trailing rest shape.
pub fn list_macro_shape_with_rest(
    head: Symbol,
    fixed_tail: Vec<Arc<dyn Shape>>,
    rest: Arc<dyn Shape>,
) -> Arc<dyn Shape> {
    let items = std::iter::once(literal_head_shape(head))
        .chain(fixed_tail)
        .collect::<Vec<_>>();
    Arc::new(ListShape::with_rest(items, rest))
}

/// Builds a macro syntax shape that captures named positional parameters and an
/// optional rest parameter after the head symbol.
pub fn positional_macro_shape(
    head: Symbol,
    fixed: &[Symbol],
    rest: Option<&Symbol>,
) -> Arc<dyn Shape> {
    let fixed_tail = fixed
        .iter()
        .cloned()
        .map(|name| Arc::new(CaptureShape::new(name, Arc::new(AnyShape))) as Arc<dyn Shape>)
        .collect::<Vec<_>>();
    match rest {
        Some(rest) => list_macro_shape_with_rest(
            head,
            fixed_tail,
            Arc::new(CaptureShape::new(rest.clone(), Arc::new(AnyShape))),
        ),
        None => list_macro_shape(head, fixed_tail),
    }
}
