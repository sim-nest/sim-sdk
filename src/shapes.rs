use std::sync::Arc;

use sim_kernel::{Cx, Error, Result, ShapeId, ShapeRef, Symbol, Value};
use sim_shape::{Shape, ShapeDoc, ShapeMatch};

/// Name of the core class under which shapes are registered.
pub const CORE_SHAPE_CLASS: &str = "Shape";

pub use sim_shape::{shape_value, shape_value_with_encoding};

/// Shape wrapper that overrides only the documentation of an inner shape,
/// delegating all matching and binding behavior unchanged.
pub struct DocumentedShape {
    name: String,
    details: Vec<String>,
    inner: Arc<dyn Shape>,
}

impl DocumentedShape {
    /// Wraps `inner` with a display `name` and zero or more detail lines.
    pub fn new(
        name: impl Into<String>,
        details: impl IntoIterator<Item = impl Into<String>>,
        inner: Arc<dyn Shape>,
    ) -> Self {
        Self {
            name: name.into(),
            details: details.into_iter().map(Into::into).collect(),
            inner,
        }
    }
}

impl Shape for DocumentedShape {
    fn is_total(&self) -> bool {
        self.inner.is_total()
    }

    fn parents(&self, cx: &mut Cx) -> Result<Vec<ShapeRef>> {
        self.inner.parents(cx)
    }

    fn is_subshape_of(&self, cx: &mut Cx, parent: &dyn Shape) -> Result<Option<bool>> {
        self.inner.is_subshape_of(cx, parent)
    }

    fn check_value(&self, cx: &mut Cx, value: Value) -> Result<sim_shape::ShapeMatch> {
        self.inner.check_value(cx, value)
    }

    fn check_expr(&self, cx: &mut Cx, expr: &sim_kernel::Expr) -> Result<sim_shape::ShapeMatch> {
        self.inner.check_expr(cx, expr)
    }

    fn describe(&self, _cx: &mut Cx) -> Result<ShapeDoc> {
        let mut doc = ShapeDoc::new(self.name.clone());
        for detail in &self.details {
            doc = doc.with_detail(detail.clone());
        }
        Ok(doc)
    }
}

/// Registers `shape` under `symbol` and returns its assigned shape id.
pub fn shape_id(cx: &mut Cx, symbol: Symbol, shape: Arc<dyn Shape>) -> sim_kernel::ShapeId {
    cx.registry_mut()
        .register_shape_value(symbol.clone(), shape_value(symbol, shape))
        .expect("shape registration should not duplicate symbols")
}

/// Borrows the shape backing a value, or errors if it is not a shape.
pub fn value_as_shape(value: &Value) -> Result<&dyn Shape> {
    value.object().as_shape().ok_or(Error::TypeMismatch {
        expected: "shape",
        found: "non-shape",
    })
}

/// Borrows the shape behind a shape reference, or errors if it is not a shape.
pub fn shape_ref_as_shape(shape: &ShapeRef) -> Result<&dyn Shape> {
    value_as_shape(shape)
}

#[derive(Clone)]
struct ValueBackedShape {
    value: ShapeRef,
}

impl Shape for ValueBackedShape {
    fn id(&self) -> Option<ShapeId> {
        self.value.object().as_shape().and_then(|shape| shape.id())
    }

    fn symbol(&self) -> Option<Symbol> {
        self.value
            .object()
            .as_shape()
            .and_then(|shape| shape.symbol())
    }

    fn parents(&self, cx: &mut Cx) -> Result<Vec<ShapeRef>> {
        shape_ref_as_shape(&self.value)?.parents(cx)
    }

    fn is_effectful(&self) -> bool {
        self.value
            .object()
            .as_shape()
            .map(Shape::is_effectful)
            .unwrap_or(false)
    }

    fn is_total(&self) -> bool {
        self.value
            .object()
            .as_shape()
            .map(Shape::is_total)
            .unwrap_or(false)
    }

    fn is_subshape_of(&self, cx: &mut Cx, parent: &dyn Shape) -> Result<Option<bool>> {
        shape_ref_as_shape(&self.value)?.is_subshape_of(cx, parent)
    }

    fn check_value(&self, cx: &mut Cx, value: Value) -> Result<ShapeMatch> {
        shape_ref_as_shape(&self.value)?.check_value(cx, value)
    }

    fn check_expr(&self, cx: &mut Cx, expr: &sim_kernel::Expr) -> Result<ShapeMatch> {
        shape_ref_as_shape(&self.value)?.check_expr(cx, expr)
    }

    fn describe(&self, cx: &mut Cx) -> Result<ShapeDoc> {
        shape_ref_as_shape(&self.value)?.describe(cx)
    }
}

/// Wraps a shape reference in a reusable `Arc<dyn Shape>` handle.
pub fn shape_ref_arc(shape: &ShapeRef) -> Result<Arc<dyn Shape>> {
    let _ = shape_ref_as_shape(shape)?;
    Ok(Arc::new(ValueBackedShape {
        value: shape.clone(),
    }))
}

/// Returns the shape id behind a shape reference, or `ShapeId(0)` if none.
pub fn shape_ref_id(shape: &ShapeRef) -> ShapeId {
    shape_ref_as_shape(shape)
        .ok()
        .and_then(|shape| shape.id())
        .unwrap_or(ShapeId(0))
}

/// Checks a value against the shape behind a shape reference.
pub fn check_shape_value(cx: &mut Cx, shape: &ShapeRef, value: Value) -> Result<ShapeMatch> {
    shape_ref_as_shape(shape)?.check_value(cx, value)
}

/// Checks an expression against the shape behind a shape reference.
pub fn check_shape_expr(
    cx: &mut Cx,
    shape: &ShapeRef,
    expr: &sim_kernel::Expr,
) -> Result<ShapeMatch> {
    shape_ref_as_shape(shape)?.check_expr(cx, expr)
}
