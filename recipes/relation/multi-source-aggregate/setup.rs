use sim::relation::plan::{Aggregate, JoinKind, NamedAggregate, Rel};
fn main() { let _ = (std::any::TypeId::of::<Aggregate>(), std::any::TypeId::of::<JoinKind>(), std::any::TypeId::of::<NamedAggregate>(), std::any::TypeId::of::<Rel>()); }
