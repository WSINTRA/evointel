use memory_proto::schema::EMBEDDING_DIM;
use memory_proto::vector::init_vector_indexes;
use memory_proto::MemoryDb;

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
