use sim::relation::{mount::{MountKind, MountedDir}, table::{TableOp, TablePath}, table_relation::{RelationDir, RelationView}};
fn main() { let _ = (std::any::TypeId::of::<MountKind>(), std::any::TypeId::of::<MountedDir>(), std::any::TypeId::of::<TableOp>(), std::any::TypeId::of::<TablePath>(), std::any::TypeId::of::<RelationDir>(), std::any::TypeId::of::<RelationView>()); }
