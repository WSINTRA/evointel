fn main() {
    let db_path = std::path::Path::new("memory-proto.db");
    let db = memory_proto::MemoryDb::open(db_path, memory_proto::schema::EMBEDDING_DIM)
        .expect("failed to open memory db");
    println!("opened memory db at {}", db.db_path.display());

    let conn = db.connect().expect("failed to connect");
    memory_proto::schema::init(&conn).expect("failed to init schema");
    println!("schema initialized");
}
