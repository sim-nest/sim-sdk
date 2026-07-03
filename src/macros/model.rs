use std::sync::Arc;

use sim_kernel::{Cx, Env, Error, Expr, Factory, Phase, Result, Symbol, Value};
use sim_shape::{Bindings, Shape};

const MAX_MACRO_EXPANSION_DEPTH: usize = 128;
const MAX_MACRO_EXPANSION_STEPS: usize = 16_384;
const MAX_MACRO_STACK_DISPLAY: usize = 8;

/// Signature of a native macro expansion function.
pub type NativeMacroImpl = fn(&mut MacroCx<'_>, Expr, Bindings) -> Result<Expr>;

/// Contract for an expandable macro: its name, its syntax shape, and the
/// expansion that rewrites a matched form into a new expression.
pub trait LispMacro: Send + Sync {
    /// Returns the symbol the macro is registered under.
    fn symbol(&self) -> Symbol;
    /// Returns the shape the macro's call syntax must match.
    fn syntax_shape(&self) -> Arc<dyn Shape>;
    /// Expands a matched input form into a replacement expression.
    fn expand(&self, cx: &mut MacroCx<'_>, input: Expr, captures: Bindings) -> Result<Expr>;
}

/// Expansion context passed to macros: the evaluation context, current phase,
/// environment, expansion budget, and the active macro stack.
pub struct MacroCx<'a> {
    pub(crate) cx: &'a mut Cx,
    pub(crate) phase: Phase,
    env: Env,
    budget: MacroExpansionBudget,
    gensym_counter: u64,
    pub(crate) stack: Vec<Symbol>,
}

/// Bounds that stop runaway macro expansion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MacroExpansionLimits {
    /// Maximum recursive expansion depth.
    pub max_depth: usize,
    /// Maximum number of expansion steps charged against the budget.
    pub max_steps: usize,
}

impl Default for MacroExpansionLimits {
    fn default() -> Self {
        Self {
            max_depth: MAX_MACRO_EXPANSION_DEPTH,
            max_steps: MAX_MACRO_EXPANSION_STEPS,
        }
    }
}

#[derive(Clone, Debug)]
struct MacroExpansionBudget {
    limits: MacroExpansionLimits,
    steps: usize,
}

impl MacroExpansionBudget {
    fn new(limits: MacroExpansionLimits) -> Self {
        Self { limits, steps: 0 }
    }
}

impl<'a> MacroCx<'a> {
    /// Creates an expansion context with the default expansion limits.
    pub fn new(cx: &'a mut Cx, phase: Phase) -> Self {
        Self::with_limits(cx, phase, MacroExpansionLimits::default())
    }

    /// Creates an expansion context with explicit expansion limits.
    pub fn with_limits(cx: &'a mut Cx, phase: Phase, limits: MacroExpansionLimits) -> Self {
        let env = cx.env().clone();
        Self {
            cx,
            phase,
            env,
            budget: MacroExpansionBudget::new(limits),
            gensym_counter: 0,
            stack: Vec::new(),
        }
    }

    /// Returns the phase the macro is being expanded in.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Returns the environment captured for this expansion.
    pub fn env(&self) -> &Env {
        &self.env
    }

    /// Returns the object factory of the underlying context.
    pub fn factory(&self) -> &dyn Factory {
        self.cx.factory()
    }

    /// Quotes an expression into a value through the factory.
    pub fn quote_expr(&self, expr: Expr) -> Result<Value> {
        self.cx.factory().expr(expr)
    }

    /// Generates a fresh hygienic symbol scoped to the active macro.
    pub fn hygienic_symbol(&mut self, prefix: impl AsRef<str>) -> Symbol {
        self.gensym_counter += 1;
        let macro_name = self
            .stack
            .last()
            .map(Symbol::as_qualified_str)
            .unwrap_or_else(|| "anonymous".to_owned());
        Symbol::qualified(
            format!("macro/{macro_name}"),
            format!("{}${}", prefix.as_ref(), self.gensym_counter),
        )
    }

    pub(crate) fn charge(&mut self, amount: usize) -> Result<()> {
        self.budget.steps = self
            .budget
            .steps
            .checked_add(amount)
            .ok_or_else(|| self.budget_error("macro expansion step counter overflowed"))?;
        if self.budget.steps > self.budget.limits.max_steps {
            return Err(self.budget_error(format!(
                "macro expansion exceeded step limit of {}",
                self.budget.limits.max_steps
            )));
        }
        Ok(())
    }

    pub(crate) fn max_depth(&self) -> usize {
        self.budget.limits.max_depth
    }

    pub(crate) fn budget_error(&self, message: impl Into<String>) -> Error {
        Error::Eval(format!("{}{}", message.into(), self.stack_suffix()))
    }

    fn stack_suffix(&self) -> String {
        if self.stack.is_empty() {
            return String::new();
        }
        let skip = self.stack.len().saturating_sub(MAX_MACRO_STACK_DISPLAY);
        let mut names = self
            .stack
            .iter()
            .skip(skip)
            .map(Symbol::to_string)
            .collect::<Vec<_>>();
        if skip > 0 {
            names.insert(0, "...".to_owned());
        }
        format!(" while expanding {}", names.join(" -> "))
    }
}
