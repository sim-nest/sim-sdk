mod lib_support;
mod model;
#[cfg(test)]
mod tests;

pub use lib_support::NativeClassLib;
pub use model::{ClassInstance, MemberFunction, NativeClass, constructor_function};
