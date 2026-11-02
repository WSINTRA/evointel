use memory_proto::MemoryDb;

fn temp_db(dim: usize) -> (tempfile::TempDir, MemoryDb) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let db = MemoryDb::open(&path, dim).unwrap();
    (dir, db)
}

#[test]
fn open_db_creates_file() {
    let (dir, db) = temp_db(384);
    assert!(dir.path().join("test.db").exists());
    let _ = db;
}

#[test]
fn connect_returns_connection() {
    let (_dir, db) = temp_db(384);
    let conn = db.connect().unwrap();
    let _ = conn;
}
