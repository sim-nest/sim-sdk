use std::sync::Arc;

use sim_kernel::{
    Args, Callable, Class, ClassId, ClassRef, Cx, DefaultFactory, Factory, Object, ObjectEncode,
    ObjectEncoding, ReadConstructorRef, Result, ShapeRef, Symbol, TableRef, Value,
};
use sim_shape::{ObjectExpr, OneOfShape, Shape};

use crate::{
    functions::{FunctionObject, empty_member_table, empty_shape_ref},
    shapes::shape_value,
};

/// Callable that reads one field out of an instance of a native class.
#[derive(Clone)]
pub struct MemberFunction {
    /// Symbol of the class this member belongs to.
    pub class_symbol: Symbol,
    /// Qualified symbol under which the member accessor is registered.
    pub symbol: Symbol,
    /// Field name the accessor reads from an instance.
    pub field: Symbol,
}

impl MemberFunction {
    /// Creates a member accessor for `field` on the class named `class_symbol`.
    pub fn new(class_symbol: Symbol, field: Symbol) -> Self {
        Self {
            symbol: Symbol::qualified(class_symbol.to_string(), field.name.clone()),
            class_symbol,
            field,
        }
    }
}

impl Object for MemberFunction {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok(format!("#<member {} {}>", self.class_symbol, self.field))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for MemberFunction {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&Symbol::qualified("core", "Function"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            sim_kernel::CORE_FUNCTION_CLASS_ID,
            Symbol::qualified("core", "Function"),
        )
    }
    fn as_expr(&self, _cx: &mut Cx) -> Result<sim_kernel::Expr> {
        Ok(sim_kernel::Expr::Symbol(self.symbol.clone()))
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
}

impl Callable for MemberFunction {
    fn call(&self, cx: &mut Cx, args: Args) -> Result<Value> {
        let args = args.into_vec();
        let [instance] = args.as_slice() else {
            return Err(sim_kernel::Error::Eval(format!(
                "member {} expects exactly one instance argument",
                self.symbol
            )));
        };

        let expr = instance.object().as_expr(cx)?;
        let entries = if let Some(object) = ObjectExpr::parse(&expr) {
            object.fields
        } else if let sim_kernel::Expr::Map(entries) = expr {
            entries
                .into_iter()
                .filter_map(|(key, value)| match key {
                    sim_kernel::Expr::Symbol(symbol) => Some((symbol, value)),
                    _ => None,
                })
                .collect()
        } else {
            return Err(sim_kernel::Error::TypeMismatch {
                expected: "map/object",
                found: "non-map",
            });
        };

        entries
            .into_iter()
            .find_map(|(key, value)| (key == self.field).then_some(value))
            .map(|expr| cx.factory().expr(expr))
            .transpose()?
            .ok_or_else(|| sim_kernel::Error::UnknownSymbol {
                symbol: self.field.clone(),
            })
    }
}

/// Host-defined class: a constructor, optional shapes, parents, and members
/// implementing the kernel `Class` and `Callable` contracts.
#[derive(Clone)]
pub struct NativeClass {
    /// Stable id assigned to the class.
    pub id: ClassId,
    /// Symbol the class is registered under.
    pub symbol: Symbol,
    /// Constructor invoked when the class is called.
    pub constructor: FunctionObject,
    /// Constructor used for read-construct literals, if distinct from the call
    /// constructor.
    pub read_constructor: Option<FunctionObject>,
    /// Shape that instances of the class are expected to satisfy.
    pub instance_shape: Option<Arc<dyn Shape>>,
    /// Symbols of the class's parent classes.
    pub parent_symbols: Vec<Symbol>,
    /// Member accessors exposed by the class.
    pub members: Vec<MemberFunction>,
}

impl NativeClass {
    /// Creates a class with the given id, symbol, constructor, instance shape,
    /// and member fields.
    pub fn new(
        id: ClassId,
        symbol: Symbol,
        constructor: FunctionObject,
        instance_shape: Option<Arc<dyn Shape>>,
        member_fields: Vec<Symbol>,
    ) -> Self {
        let members = member_fields
            .into_iter()
            .map(|field| MemberFunction::new(symbol.clone(), field))
            .collect();
        Self {
            id,
            symbol,
            read_constructor: Some(constructor.clone()),
            constructor,
            instance_shape,
            parent_symbols: Vec::new(),
            members,
        }
    }

    /// Sets the read-construct constructor and returns the updated class.
    pub fn with_read_constructor(mut self, read_constructor: Option<FunctionObject>) -> Self {
        self.read_constructor = read_constructor;
        self
    }

    /// Sets the parent symbols and returns the updated class.
    pub fn with_parents(mut self, parent_symbols: Vec<Symbol>) -> Self {
        self.parent_symbols = parent_symbols;
        self
    }

    /// Returns the class's call constructor.
    pub fn constructor(&self) -> &FunctionObject {
        &self.constructor
    }

    /// Iterates over the field names of the class's members.
    pub fn member_names(&self) -> impl Iterator<Item = &Symbol> {
        self.members.iter().map(|member| &member.field)
    }

    /// Returns the class's member accessors.
    pub fn member_functions(&self) -> &[MemberFunction] {
        &self.members
    }

    /// Looks up a member accessor by field name.
    pub fn member_function(&self, field: &Symbol) -> Option<&MemberFunction> {
        self.members.iter().find(|member| &member.field == field)
    }

    /// Builds the combined argument shape covering all constructor cases.
    pub fn constructor_shape_arc(&self) -> Option<Arc<dyn Shape>> {
        match self.constructor.cases.as_slice() {
            [] => None,
            [one] => Some(one.args.clone()),
            many => Some(Arc::new(OneOfShape::new(
                many.iter().map(|case| case.args.clone()).collect(),
            ))),
        }
    }
}

impl Object for NativeClass {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok(format!("#<class {}>", self.symbol))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for NativeClass {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        if let Some(value) = cx
            .registry()
            .class_by_symbol(&Symbol::qualified("core", "Class"))
        {
            return Ok(value.clone());
        }
        cx.factory().class_stub(
            sim_kernel::CORE_CLASS_CLASS_ID,
            Symbol::qualified("core", "Class"),
        )
    }
    fn as_expr(&self, _cx: &mut Cx) -> Result<sim_kernel::Expr> {
        Ok(sim_kernel::Expr::Symbol(self.symbol.clone()))
    }
    fn as_table(&self, cx: &mut Cx) -> Result<Value> {
        let mut entries = vec![
            (
                Symbol::new("symbol"),
                cx.factory().string(self.symbol.to_string())?,
            ),
            (
                Symbol::new("constructor"),
                cx.factory().string(self.constructor.symbol.to_string())?,
            ),
            (
                Symbol::new("member-count"),
                cx.factory().number_literal(
                    Symbol::qualified("numbers", "f64"),
                    self.members.len().to_string(),
                )?,
            ),
        ];
        if let Some(shape) = self.constructor_shape_arc() {
            let doc = shape.describe(cx)?;
            entries.push((
                Symbol::new("constructor-shape"),
                cx.factory().string(doc.name)?,
            ));
        }
        if let Some(shape) = &self.instance_shape {
            let doc = shape.describe(cx)?;
            entries.push((
                Symbol::new("instance-shape"),
                cx.factory().string(doc.name)?,
            ));
        }
        for member in &self.members {
            entries.push((
                Symbol::qualified("member", member.field.name.clone()),
                cx.factory().string(member.symbol.to_string())?,
            ));
        }
        cx.factory().table(entries)
    }
    fn as_callable(&self) -> Option<&dyn Callable> {
        Some(self)
    }
    fn as_class(&self) -> Option<&dyn Class> {
        Some(self)
    }
}

impl Callable for NativeClass {
    fn call(&self, cx: &mut Cx, args: Args) -> Result<Value> {
        self.constructor.call(cx, args)
    }

    fn browse_args_shape(&self, cx: &mut Cx) -> Result<Option<ShapeRef>> {
        shape_option(self.constructor_shape(cx)?)
    }

    fn browse_result_shape(&self, cx: &mut Cx) -> Result<Option<ShapeRef>> {
        shape_option(self.instance_shape(cx)?)
    }
}

fn shape_option(value: ShapeRef) -> Result<Option<ShapeRef>> {
    if value.object().as_shape().is_some() {
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

impl Class for NativeClass {
    fn id(&self) -> ClassId {
        self.id
    }

    fn symbol(&self) -> Symbol {
        self.symbol.clone()
    }

    fn parents(&self, cx: &mut Cx) -> Result<Vec<ClassRef>> {
        Ok(self
            .parent_symbols
            .iter()
            .filter_map(|symbol| cx.registry().class_by_symbol(symbol).cloned())
            .collect())
    }

    fn constructor_shape(&self, cx: &mut Cx) -> Result<ShapeRef> {
        match self.constructor_shape_arc() {
            Some(shape) => Ok(shape_value(
                Symbol::qualified(self.symbol.to_string(), "constructor-shape"),
                shape,
            )),
            None => empty_shape_ref(cx),
        }
    }

    fn instance_shape(&self, cx: &mut Cx) -> Result<ShapeRef> {
        match &self.instance_shape {
            Some(shape) => Ok(shape_value(
                Symbol::qualified(self.symbol.to_string(), "instance-shape"),
                shape.clone(),
            )),
            None => empty_shape_ref(cx),
        }
    }

    fn read_constructor(&self, _cx: &mut Cx) -> Result<Option<ReadConstructorRef>> {
        Ok(self.read_constructor.as_ref().map(|read| {
            DefaultFactory
                .opaque(Arc::new(read.clone()))
                .expect("read constructor should be boxable")
        }))
    }

    fn members(&self, cx: &mut Cx) -> Result<TableRef> {
        if self.members.is_empty() {
            return empty_member_table(cx);
        }
        cx.factory().table(
            self.members
                .iter()
                .map(|member| {
                    (
                        member.field.clone(),
                        DefaultFactory
                            .opaque(Arc::new(member.clone()))
                            .expect("member function should be boxable"),
                    )
                })
                .collect(),
        )
    }
}

/// Instance of a native class: its class symbol, the constructor arguments it
/// was built from, and its field values.
#[derive(Clone)]
pub struct ClassInstance {
    /// Symbol of the class this is an instance of.
    pub class_symbol: Symbol,
    /// Expressions the instance was constructed from, used for re-encoding.
    pub constructor_args: Vec<sim_kernel::Expr>,
    /// Field values held by the instance.
    pub fields: Vec<(Symbol, Value)>,
}

impl ClassInstance {
    /// Creates an instance from its class symbol, constructor arguments, and
    /// field values.
    pub fn new(
        class_symbol: Symbol,
        constructor_args: Vec<sim_kernel::Expr>,
        fields: Vec<(Symbol, Value)>,
    ) -> Self {
        Self {
            class_symbol,
            constructor_args,
            fields,
        }
    }
}

impl Object for ClassInstance {
    fn display(&self, _cx: &mut Cx) -> Result<String> {
        Ok(format!("#<instance {}>", self.class_symbol))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl sim_kernel::ObjectCompat for ClassInstance {
    fn class(&self, cx: &mut Cx) -> Result<ClassRef> {
        if let Some(value) = cx.registry().class_by_symbol(&self.class_symbol) {
            return Ok(value.clone());
        }
        cx.factory()
            .class_stub(ClassId(0), self.class_symbol.clone())
    }
    fn as_expr(&self, cx: &mut Cx) -> Result<sim_kernel::Expr> {
        Ok(ObjectExpr {
            class: self.class_symbol.clone(),
            fields: self
                .fields
                .iter()
                .map(|(key, value)| Ok((key.clone(), value.object().as_expr(cx)?)))
                .collect::<Result<Vec<_>>>()?,
        }
        .to_expr())
    }
    fn as_table(&self, cx: &mut Cx) -> Result<Value> {
        cx.factory().table(self.fields.clone())
    }
    fn as_object_encoder(&self) -> Option<&dyn ObjectEncode> {
        Some(self)
    }
}

impl ObjectEncode for ClassInstance {
    fn object_encoding(&self, _cx: &mut Cx) -> Result<ObjectEncoding> {
        Ok(ObjectEncoding::Constructor {
            class: self.class_symbol.clone(),
            args: self.constructor_args.clone(),
        })
    }
}

/// Borrows the constructor function of a native class.
pub fn constructor_function(class: &NativeClass) -> &FunctionObject {
    &class.constructor
}
