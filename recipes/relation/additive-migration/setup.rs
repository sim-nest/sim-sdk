use sim::relation::migrate::{admit, derive_lossless, CheckedProgram, MigrationProgram, Operation};
fn main() { let _ = (admit, derive_lossless, std::any::TypeId::of::<CheckedProgram>(), std::any::TypeId::of::<MigrationProgram>(), std::any::TypeId::of::<Operation>()); }
