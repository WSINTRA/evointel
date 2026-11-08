//! Migration tests — the schema updating itself in place.
//!
//! The three worlds every migration must survive:
//! 1. a fresh DB (nothing exists),
//! 2. a pre-migration DB (old tables, no marker — every DB opened before
//!    Task 20),
//! 3. a re-run (partial or complete — idempotence converges both).

use memory_proto::db::MemoryDb;
use memory_proto::schema::{self, EMBEDDING_DIM};

fn temp_db() -> (tempfile::TempDir, MemoryDb) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let db = MemoryDb::open(&path, EMBEDDING_DIM).unwrap();
    (dir, db)
}

/// lbug has no SHOW TABLES — existence is probed by query-and-error.
fn table_exists(conn: &lbug::Connection, label: &str) -> bool {
    conn.query(&format!("MATCH (n:{label}) RETURN count(n);"))
        .is_ok()
}

/// Rel tables are probed with a rel pattern instead of a node pattern.
fn rel_table_exists(conn: &lbug::Connection, label: &str) -> bool {
    conn.query(&format!("MATCH ()-[r:{label}]->() RETURN count(r);"))
        .is_ok()
}

fn count(conn: &lbug::Connection, cypher: &str) -> i64 {
    let result = conn.query(cypher).unwrap();
    for row in result {
        if let lbug::Value::Int64(n) = row[0] {
            return n;
        }
    }
    panic!("no count row for: {cypher}")
}

#[test]
fn fresh_init_reaches_latest_version() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    schema::init(&conn).unwrap();

    assert_eq!(
        schema::meta::schema_version(&conn).unwrap(),
        schema::latest_version()
    );
    // Spot-check the world the batch built: a content table, an
    // edge-node table, and the time-tree seeds.
    assert!(table_exists(&conn, "Note"));
    assert!(table_exists(&conn, "Memory"));
    assert!(table_exists(&conn, "Similar"));
    assert_eq!(
        count(&conn, "MATCH (t:Time) RETURN count(t);"),
        4,
        "time-tree seeds: year, month, day, week"
    );
}

#[test]
fn reinit_is_noop() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    schema::init(&conn).unwrap();
    schema::init(&conn).unwrap();

    assert_eq!(
        schema::meta::schema_version(&conn).unwrap(),
        schema::latest_version()
    );
    // MERGE seeds did not duplicate.
    assert_eq!(count(&conn, "MATCH (t:Time) RETURN count(t);"), 4);
    assert_eq!(count(&conn, "MATCH (t:AbstractTime) RETURN count(t);"), 2);
}

#[test]
fn pre_migration_db_converges_with_data_intact() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    // The old world: tables created before migrations existed, no marker.
    conn.query(
        "CREATE NODE TABLE Note(
            id STRING PRIMARY KEY, content STRING,
            content_embedding FLOAT[768], created_at TIMESTAMP,
            updated_at TIMESTAMP, layer INT16 DEFAULT 0, context STRING
        );",
    )
    .unwrap();
    conn.query(
        "CREATE (n:Note {id: 'old-1', content: 'written before migrations',
            created_at: timestamp('2026-01-01'), updated_at: timestamp('2026-01-01')});",
    )
    .unwrap();

    schema::init(&conn).unwrap();

    // The marker now exists at the latest version, the missing tables
    // appeared, and the old data survived the upgrade.
    assert_eq!(
        schema::meta::schema_version(&conn).unwrap(),
        schema::latest_version()
    );
    assert!(table_exists(&conn, "Memory"));
    assert!(table_exists(&conn, "Before"));
    assert!(rel_table_exists(&conn, "TO_Contains"));
    assert_eq!(
        count(&conn, "MATCH (n:Note {id: 'old-1'}) RETURN count(n);"),
        1
    );
    assert_eq!(
        count(&conn, "MATCH (n:Note) RETURN count(n);"),
        1,
        "no duplication of existing rows"
    );
}

#[test]
fn partial_schema_converges() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    // A batch half-applied (crash mid-run, marker never written): some
    // tables exist, the rest don't.
    conn.query("CREATE NODE TABLE Entity(id STRING PRIMARY KEY);")
        .unwrap();
    conn.query("CREATE NODE TABLE Fact(id STRING PRIMARY KEY);")
        .unwrap();
    assert!(!table_exists(&conn, "Note"));

    schema::init(&conn).unwrap();

    assert!(table_exists(&conn, "Note"));
    assert!(table_exists(&conn, "Memory"));
    assert_eq!(
        schema::meta::schema_version(&conn).unwrap(),
        schema::latest_version()
    );
}

/// Pinned empirically (Task 20) against lbug 0.22.1, both halves:
///
/// 1. DDL **is** transactional in the small: `BEGIN` + CREATE + `ROLLBACK`
///    leaves no table behind.
/// 2. But a real migration batch — tens of CREATEs plus MERGE seeds —
///    run inside **one** transaction dies partway: some later seed can
///    no longer bind a table the batch created, and ROLLBACK confirms
///    nothing from the batch persisted.
///
/// Conclusion, now evidence-backed: batches run auto-committed, and
/// batch atomicity comes from idempotent DDL plus the after-batch
/// version marker. If lbug ever makes half 2 pass, wrapping batches in
/// transactions becomes an option, not a requirement.
#[test]
fn transactional_batches_are_unreliable_so_batches_auto_commit() {
    let (_dir, db) = temp_db();
    let conn = db.connect().unwrap();

    // Half 1: the small case works — rollback really removes DDL.
    conn.query("BEGIN TRANSACTION").unwrap();
    conn.query("CREATE NODE TABLE ProbeTx(id STRING PRIMARY KEY);")
        .unwrap();
    conn.query("ROLLBACK").unwrap();
    assert!(!table_exists(&conn, "ProbeTx"));

    // Half 2: the real batch cannot complete inside one transaction.
    conn.query("BEGIN TRANSACTION").unwrap();
    let mut failed = false;
    for (_, ddls) in memory_proto::schema::migrations::batches() {
        for ddl in &ddls {
            if conn.query(ddl).is_err() {
                failed = true;
                break;
            }
        }
        if failed {
            break;
        }
    }
    let _ = conn.query("ROLLBACK");
    assert!(
        failed,
        "the full batch survived one transaction — lbug changed; \
          wrapping migrate() batches becomes an option"
    );
    // And the failed batch left nothing behind.
    assert!(!table_exists(&conn, "Note"));
    assert!(!table_exists(&conn, "AbstractTime"));
}
