pub mod edge_nodes;
pub mod entity_nodes;
pub mod meta;
pub mod migrations;
pub mod poly_relations;
pub mod relations;
pub mod self_nodes;
pub mod time_tree;

use lbug::Connection;

use crate::db::DbResult;

pub use entity_nodes::EMBEDDING_DIM;
pub use migrations::latest_version;

/// Creates or migrates the schema to [`latest_version`].
///
/// Idempotent: safe on every open, fresh DB or old — the version marker
/// decides what runs. Before Task 20 this was create-only and callers
/// guarded it with file-existence checks; an older DB was silently never
/// migrated. Now `init` *is* the migration.
pub fn init(conn: &Connection) -> DbResult<()> {
    migrations::migrate(conn)?;
    Ok(())
}
