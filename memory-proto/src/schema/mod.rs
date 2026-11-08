pub mod edge_nodes;
pub mod entity_nodes;
pub mod poly_relations;
pub mod relations;
pub mod time_tree;

use lbug::Connection;

use crate::db::DbResult;

pub use entity_nodes::EMBEDDING_DIM;

pub fn init(conn: &Connection) -> DbResult<()> {
    // Entity tables carry the embedding column, so their width is
    // rendered from EMBEDDING_DIM; the rest are static.
    for ddl in entity_nodes::entity_node_ddls() {
        conn.query(&ddl)?;
    }
    for ddl in edge_nodes::ALL_EDGE_NODE_DDLS
        .iter()
        .chain(relations::ALL_RELATION_DDLS.iter())
        .chain(poly_relations::ALL_POLY_RELATIONS.iter())
        .chain(time_tree::ALL_TIME_TREE_DDLS.iter())
    {
        conn.query(ddl)?;
    }
    Ok(())
}
