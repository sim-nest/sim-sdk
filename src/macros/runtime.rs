use std::sync::Arc;

use sim_kernel::{
    CORE_MACRO_CLASS_ID, ClassRef, Cx, Expr, Factory, Object, Result, Symbol, TableRef, Value,
};
use sim_shape::{Bindings, Shape};

use crate::macros::{LispMacro, MacroCx, NativeMacroImpl};

const CORE_MACRO_CLASS: &str = "Macro";

/// Macro implemented by a native Rust expansion function.
#[derive(Clone)]
pub struct NativeExprMacro {
    symbol: Symbol,
    syntax_shape: Arc<dyn Shape>,
    implementation: NativeMacroImpl,
}

impl NativeExprMacro {
    /// Creates a native macro from its symbol, syntax shape, and expansion
    /// function.
    pub fn new(
        symbol: Symbol,
        syntax_shape: Arc<dyn Shape>,
        implementation: NativeMacroImpl,
    ) -> Self {
        Self {
            symbol,
            syntax_shape,
            implementation,
        }
    }
}

impl LispMacro for NativeExprMacro {
    fn symbol(&self) -> Symbol {
        self.symbol.clone()
    }

    fn syntax_shape(&self) -> Arc<dyn Shape> {
        self.syntax_shape.clone()
    }

    fn expand(&self, cx: &mut MacroCx<'_>, input: Expr, captures: Bindings) -> Result<Expr> {
        (self.implementation)(cx, input, captures)
    }
}

/// Macro defined in source by a template expression instantiated from captures.
#[derive(Clone)]
pub struct SourceTemplateMacro {
    symbol: Symbol,
    syntax_shape: Arc<dyn Shape>,
    template: Expr,
}

impl SourceTemplateMacro {
    /// Creates a template macro from its symbol, syntax shape, and template.
    pub fn new(symbol: Symbol, syntax_shape: Arc<dyn Shape>, template: Expr) -> Self {
        Self {
            symbol,
            syntax_shape,
            template,
        }
    }
}

impl LispMacro for SourceTemplateMacro {
    fn symbol(&self) -> Symbol {
        self.symbol.clone()
    }

    fn syntax_shape(&self) -> Arc<dyn Shape> {
        self.syntax_shape.clone()
    }

    fn expand(&self, _cx: &mut MacroCx<'_>, _input: Expr, captures: Bindings) -> Result<Expr> {
        super::template::instantiate_macro_template(&self.template, &captures)
    }
}

/// Runtime object wrapping a macro and whether its parser output is trusted.
#[derive(Clone)]
pub struct MacroObject {
    inner: Arc<dyn LispMacro>,
    parser_trusted: bool,
}

impl MacroObject {
    /// Wraps a macro with its parser-trust flag.
    pub fn new(inner: Arc<dyn LispMacro>, parser_trusted: bool) -> Self {
        Self {
            inner,
            parser_trusted,
        }
    }

    /// Borrows the wrapped macro.
    pub fn macro_ref(&self) -> &dyn LispMacro {
        self.inner.as_ref()
    }

    /// Returns the wrapped macro's syntax shape.
    pub fn syntax_shape(&self) -> Arc<dyn Shape> {
        self.inner.syntax_shape()
    }

    /// Returns whether the macro's parser output is trusted.
    pub fn parser_trusted(&self) -> bool {
        self.parser_trusted
    }
}

impl Object for MacroObject {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok(format!("#<macro {}>", self.inner.symbol()))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for MacroObject {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&Symbol::qualified("core", CORE_MACRO_CLASS))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            CORE_MACRO_CLASS_ID,
            Symbol::qualified("core", CORE_MACRO_CLASS),
        )
    }
    fn as_expr(&self, _cx: &mut Cx) -> Result<Expr> {
        Ok(Expr::Symbol(self.inner.symbol()))
    }
    fn as_table(&self, cx: &mut Cx) -> Result<TableRef> {
        let shape = self.inner.syntax_shape();
        let doc = shape.describe(cx)?;
        let mut entries = vec![
            (
                Symbol::new("symbol"),
                cx.factory().string(self.inner.symbol().to_string())?,
            ),
            (Symbol::new("syntax-shape"), cx.factory().string(doc.name)?),
            (
                Symbol::new("parser-trusted"),
                cx.factory().bool(self.parser_trusted)?,
            ),
        ];
        for (index, detail) in doc.details.into_iter().enumerate() {
            entries.push((
                Symbol::qualified("syntax-detail", index.to_string()),
                cx.factory().string(detail)?,
            ));
        }
        cx.factory().table(entries)
    }
}

/// Boxes a macro into a value, trusting its parser output.
pub fn macro_value(mac: Arc<dyn LispMacro>) -> Value {
    macro_value_with_parser_trust(mac, true)
}

/// Boxes a macro into a value with an explicit parser-trust flag.
pub fn macro_value_with_parser_trust(mac: Arc<dyn LispMacro>, parser_trusted: bool) -> Value {
    sim_kernel::DefaultFactory
        .opaque(Arc::new(MacroObject::new(mac, parser_trusted)))
        .expect("macro object should always be boxable")
}
