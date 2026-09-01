use sim::relation::{core::{DomainCatalog, DomainSpec}, plan::{admit_mutation, Mutation}, schema::SchemaBuilder};
fn main() { let _owners = (std::any::TypeId::of::<DomainCatalog>(), std::any::TypeId::of::<DomainSpec>(), std::any::TypeId::of::<Mutation>(), std::any::TypeId::of::<SchemaBuilder>()); let _ = admit_mutation; }
