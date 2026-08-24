use sim::relation::plan::{admit_query, FieldRef, Rel, Scalar};
fn main() { let _ = (admit_query, std::any::TypeId::of::<FieldRef>(), std::any::TypeId::of::<Rel>(), std::any::TypeId::of::<Scalar>()); }
