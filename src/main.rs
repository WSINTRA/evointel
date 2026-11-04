fn main() {
    let db_path = std::path::Path::new("memory-proto.db");
    let is_new = !db_path.exists();

    let db = memory_proto::MemoryDb::open(db_path, memory_proto::schema::EMBEDDING_DIM)
        .expect("failed to open memory db");
    println!("opened memory db at {}", db.db_path.display());

    let conn = db.connect().expect("failed to connect");
    if is_new {
        memory_proto::schema::init(&conn).expect("failed to init schema");
        println!("schema initialized");
    } else {
        println!("existing db found, skipping schema init");
    }
}
