/// Open domains, typed cells, rows, and relational identities.
pub use sim_relation_core as core;
/// Checked adoption and migration programs.
pub use sim_relation_migrate as migrate;
/// Raw relational algebra, admission, and read-only checked-plan views.
pub use sim_relation_plan as plan;
/// Logical and normalized physical schemas plus their raw builders.
pub use sim_relation_schema as schema;
/// Runtime Shapes for cells, rows, and relational records.
pub use sim_relation_shape as shapes;
/// Provider-neutral placement, sessions, bounded effects, and receipts.
pub use sim_relation_site as site;
/// Standard Table/Dir paths and operation protocol.
pub use sim_table_core as table;
/// Mounted Table/Dir namespace composition.
pub use sim_table_mount as mount;
/// Relation-backed Table and Dir adapters.
pub use sim_table_relation as table_relation;

/// The sole SQL provider capsule; its prepared SQL remains private to it.
#[cfg(feature = "relation-sqlite")]
pub use sim_platform_sqlite as sqlite;
