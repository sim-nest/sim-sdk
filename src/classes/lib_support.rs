use std::sync::Arc;

use sim_kernel::{
    AbiVersion, Dependency, Export, Factory, Lib, LibManifest, LibTarget, Linker, LoadCx, Result,
    Symbol, Value, Version,
};

use crate::{classes::model::NativeClass, shapes::shape_value};

/// Lib that registers a host-defined [`NativeClass`], its constructor, optional
/// constructor and instance shapes, and its member functions.
pub struct NativeClassLib {
    manifest: LibManifest,
    class_symbol: Symbol,
    class_value: Value,
    constructor_symbol: Symbol,
    constructor_value: Value,
    constructor_shape_symbol: Option<Symbol>,
    constructor_shape_value: Option<Value>,
    instance_shape_symbol: Option<Symbol>,
    instance_shape_value: Option<Value>,
    member_values: Vec<(Symbol, Value)>,
}

impl NativeClassLib {
    /// Builds the lib from a native class, lib symbol, and version string.
    pub fn from_class(lib_symbol: Symbol, class: &NativeClass, version: impl Into<String>) -> Self {
        let version = Version(version.into());
        let constructor_shape_symbol = Some(Symbol::qualified(
            class.symbol.to_string(),
            "constructor-shape",
        ));
        let constructor_shape_value = class.constructor_shape_arc().map(|shape| {
            shape_value(
                Symbol::qualified(class.symbol.to_string(), "constructor-shape"),
                shape,
            )
        });
        let instance_shape_symbol = class
            .instance_shape
            .as_ref()
            .map(|_| Symbol::qualified(class.symbol.to_string(), "instance-shape"));
        let instance_shape_value = class.instance_shape.as_ref().map(|shape| {
            shape_value(
                Symbol::qualified(class.symbol.to_string(), "instance-shape"),
                shape.clone(),
            )
        });

        let mut exports = vec![
            Export::Class {
                symbol: class.symbol.clone(),
                class_id: None,
            },
            Export::Function {
                symbol: class.constructor.symbol.clone(),
                function_id: None,
            },
        ];
        if let Some(symbol) = &constructor_shape_symbol {
            exports.push(Export::Shape {
                symbol: symbol.clone(),
                shape_id: None,
            });
        }
        if let Some(symbol) = &instance_shape_symbol {
            exports.push(Export::Shape {
                symbol: symbol.clone(),
                shape_id: None,
            });
        }
        exports.extend(class.members.iter().map(|member| Export::Function {
            symbol: member.symbol.clone(),
            function_id: None,
        }));

        Self {
            manifest: LibManifest {
                id: lib_symbol,
                version,
                abi: AbiVersion { major: 0, minor: 1 },
                target: LibTarget::HostRegistered,
                requires: Vec::<Dependency>::new(),
                capabilities: Vec::new(),
                exports,
            },
            class_symbol: class.symbol.clone(),
            class_value: sim_kernel::DefaultFactory
                .opaque(Arc::new(class.clone()))
                .expect("class should be boxable"),
            constructor_symbol: class.constructor.symbol.clone(),
            constructor_value: sim_kernel::DefaultFactory
                .opaque(Arc::new(class.constructor.clone()))
                .expect("constructor should be boxable"),
            constructor_shape_symbol,
            constructor_shape_value,
            instance_shape_symbol,
            instance_shape_value,
            member_values: class
                .members
                .iter()
                .map(|member| {
                    (
                        member.symbol.clone(),
                        sim_kernel::DefaultFactory
                            .opaque(Arc::new(member.clone()))
                            .expect("member function should be boxable"),
                    )
                })
                .collect(),
        }
    }
}

impl Lib for NativeClassLib {
    fn manifest(&self) -> LibManifest {
        self.manifest.clone()
    }

    fn load(&self, _cx: &mut LoadCx, linker: &mut Linker) -> Result<()> {
        linker.class_value(self.class_symbol.clone(), self.class_value.clone())?;
        linker.function_value(
            self.constructor_symbol.clone(),
            self.constructor_value.clone(),
        )?;
        if let (Some(symbol), Some(value)) = (
            &self.constructor_shape_symbol,
            &self.constructor_shape_value,
        ) {
            linker.shape_value(symbol.clone(), value.clone())?;
        }
        if let (Some(symbol), Some(value)) =
            (&self.instance_shape_symbol, &self.instance_shape_value)
        {
            linker.shape_value(symbol.clone(), value.clone())?;
        }
        for (symbol, value) in &self.member_values {
            linker.function_value(symbol.clone(), value.clone())?;
        }
        Ok(())
    }
}
