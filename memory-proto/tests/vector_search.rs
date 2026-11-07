use chrono::DateTime;
use memory_proto::mutations::insert_entity;
use memory_proto::nodes::Entity;
use memory_proto::schema::EMBEDDING_DIM;
use memory_proto::vector::{init_vector_indexes, semantic_search};
use memory_proto::MemoryDb;

fn ts() -> DateTime<chrono::Utc> {
    DateTime::from_timestamp(1700000000, 0).unwrap()
}

fn entity_with_embedding(id: &str, embedding: Vec<f32>) -> Entity {
    Entity {
        id: id.to_string(),
        label: id.to_string(),
        label_resolved: id.to_string(),
        label_embedding: Some(embedding),
        learned_at: ts(),
        expire_at: None,
        created_at: ts(),
        updated_at: ts(),
        layer: 1,
        context: "test".to_string(),
    }
}

#[test]
fn vector_index_init_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::extensions::load_extensions(&conn).unwrap();
    memory_proto::schema::init(&conn).unwrap();

    init_vector_indexes(&conn).unwrap();
}

#[test]
fn vector_index_init_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::extensions::load_extensions(&conn).unwrap();
    memory_proto::schema::init(&conn).unwrap();

    init_vector_indexes(&conn).unwrap();
    init_vector_indexes(&conn).unwrap();
}

#[test]
fn semantic_search_returns_ranked_results() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::extensions::load_extensions(&conn).unwrap();
    memory_proto::schema::init(&conn).unwrap();
    init_vector_indexes(&conn).unwrap();

    // Create 518-dim embeddings with distinctive patterns
    let mut emb_a = vec![0.0f32; EMBEDDING_DIM];
    emb_a[0] = 1.0; // [1, 0, 0, ...]

    let mut emb_b = vec![0.0f32; EMBEDDING_DIM];
    emb_b[1] = 1.0; // [0, 1, 0, ...]

    let mut emb_c = vec![0.0f32; EMBEDDING_DIM];
    emb_c[0] = 1.0;
    emb_c[1] = 1.0; // [1, 1, 0, ...]

    insert_entity(&conn, &entity_with_embedding("ent-a", emb_a)).unwrap();
    insert_entity(&conn, &entity_with_embedding("ent-b", emb_b)).unwrap();
    insert_entity(&conn, &entity_with_embedding("ent-c", emb_c)).unwrap();

    // Search for [1, 0, 0, ...] — entity A should be closest (distance 0)
    let mut query = vec![0.0f32; EMBEDDING_DIM];
    query[0] = 1.0;

    let results = semantic_search(&conn, &query, 3, "Entity").unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].id, "ent-a");
    assert!(results[0].score < results[1].score);
}

#[test]
fn semantic_search_respects_top_k() {
    let dir = tempfile::tempdir().unwrap();
    let db = MemoryDb::open(&dir.path().join("test.db"), EMBEDDING_DIM).unwrap();
    let conn = db.connect().unwrap();
    memory_proto::extensions::load_extensions(&conn).unwrap();
    memory_proto::schema::init(&conn).unwrap();
    init_vector_indexes(&conn).unwrap();

    // Insert 5 entities
    for i in 0..5 {
        let mut emb = vec![0.0f32; EMBEDDING_DIM];
        emb[i] = 1.0;
        insert_entity(&conn, &entity_with_embedding(&format!("ent-{i}"), emb)).unwrap();
    }

    let mut query = vec![0.0f32; EMBEDDING_DIM];
    query[0] = 1.0;

    let results = semantic_search(&conn, &query, 2, "Entity").unwrap();
    assert_eq!(results.len(), 2);
}
