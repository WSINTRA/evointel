//! Task 19 — the live demo dream: the §8 story as real notes, through
//! the real judge, against the real servers on this machine.
//!
//! Re-running is the idempotency demo: the second dream re-judges the
//! same notes and the graph converges instead of duplicating. Notes
//! carry explicit dates so the calendar extraction fires.

use chrono::{DateTime, TimeZone, Utc};
use memory_proto::db::MemoryDb;
use memory_proto::dreamer::{DreamSpec, TimeWindows, run_dream};
use memory_proto::llm::{LlmClient, LlmConfig, LlmJudge, LlmSermonWriter, UreqClient};
use memory_proto::nodes::Note;
use memory_proto::schema::{self, EMBEDDING_DIM};
use memory_proto::{extensions, vector};

fn ts(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 9, 0, 0)
        .single()
        .expect("valid demo date")
}

fn note(id: &str, content: &str, at: DateTime<Utc>) -> Note {
    Note {
        id: id.to_string(),
        content: content.to_string(),
        content_embedding: None,
        created_at: at,
        updated_at: at,
        layer: 0,
        context: "demo".to_string(),
    }
}

fn main() {
    let root = std::path::Path::new("demo-data");
    let outbox = root.join("outbox");
    std::fs::create_dir_all(&outbox).expect("create demo dirs");
    let db_path = root.join("dream.db");
    let is_new = !db_path.exists();

    let db = MemoryDb::open(&db_path, EMBEDDING_DIM).expect("open dream db");
    let conn = db.connect().expect("connect");
    extensions::load_extensions(&conn).expect("load extensions");
    if is_new {
        schema::init(&conn).expect("init schema");
        vector::init_vector_indexes(&conn).expect("init vector indexes");
    }

    // The §8 story, dated so the calendar fires.
    let mut queue = vec![
        note(
            "demo-n-a1",
            "2026-11-03: applied to five companies today, feeling anxious about the job search.",
            ts(2026, 11, 3),
        ),
        note(
            "demo-n-a2",
            "2026-11-05: revised the CV again, still worried about the job search.",
            ts(2026, 11, 5),
        ),
        note(
            "demo-n-b1",
            "2027-02-10: scheduled interviews with three companies, feeling strategic about the job search.",
            ts(2027, 2, 10),
        ),
    ];

    // Meaning first: embeddings for the queue and the anchor, while
    // the client is still unowned.
    let client = UreqClient::new(LlmConfig::from_env());
    let contents: Vec<&str> = queue.iter().map(|n| n.content.as_str()).collect();
    let note_embeddings = client.embed(&contents).expect("embed note contents");
    let anchor = client
        .embed(&["job search"])
        .expect("embed anchor")
        .remove(0);
    for (n, emb) in queue.iter_mut().zip(note_embeddings) {
        n.content_embedding = Some(emb);
    }

    // The dream: window A is November 2026 (the anxious cluster),
    // window B is February 2027 (the strategic one).
    let windows: TimeWindows = (
        (
            Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0)
                .single()
                .expect("window A start"),
            Utc.with_ymd_and_hms(2026, 11, 30, 0, 0, 0)
                .single()
                .expect("window A end"),
        ),
        (
            Utc.with_ymd_and_hms(2027, 2, 1, 0, 0, 0)
                .single()
                .expect("window B start"),
            Utc.with_ymd_and_hms(2027, 2, 28, 0, 0, 0)
                .single()
                .expect("window B end"),
        ),
    );

    let judge = LlmJudge::new(Box::new(client));
    let sermon = LlmSermonWriter::new(Box::new(UreqClient::new(LlmConfig::from_env())));
    let spec = DreamSpec {
        run_id: "demo-1".to_string(),
        queue,
        anchor_embedding: anchor,
        drift_windows: Some(windows),
        outbox: outbox.clone(),
        dream_log: root.join("dream-log.jsonl"),
    };

    println!("dreaming (three judgments and one sermon, ~1-2 min each)...");
    let report = run_dream(&conn, &judge, &sermon, &spec);
    println!("\n=== dream report ===\n{report:#?}");

    if report.message_written {
        let sermon: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(outbox.join("demo-1-sermon.json")).expect("sermon file"),
        )
        .expect("sermon json");
        println!(
            "\n=== the morning message ===\n{}",
            sermon["body"].as_str().unwrap_or("(no body)")
        );
    } else {
        println!("\nno message written — a quiet day or a fault: see the report.");
    }
}
