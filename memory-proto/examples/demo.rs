//! Task 19 — the live demo dream: the §8 story as real notes, through
//! the real judge, against the real servers on this machine.
//!
//! Re-running is the idempotency demo: the second dream re-judges the
//! same notes and the graph converges instead of duplicating. Notes
//! carry explicit dates so the calendar extraction fires.

use chrono::{DateTime, TimeZone, Utc};
use memory_proto::db::MemoryDb;
use memory_proto::dreamer::{DreamSpec, NoDelivery, TimeWindows, run_dream};
use memory_proto::llm::{LlmClient, LlmConfig, LlmJudge, LlmSermonWriter, UreqClient};
use memory_proto::nodes::{FunctionAnchor, Note};
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

    let db = MemoryDb::open(&db_path, EMBEDDING_DIM).expect("open dream db");
    let conn = db.connect().expect("connect");
    extensions::load_extensions(&conn).expect("load extensions");
    // Migrates in place: an old dream.db upgrades instead of being
    // left behind (Task 20) — init is idempotent on every open.
    schema::init(&conn).expect("init or migrate schema");
    vector::init_vector_indexes(&conn).expect("init vector indexes");

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

    // The axes: embed the anchor corpus and seed it into the graph.
    // The self layer is opt-in — this seeding is what wakes it, and the
    // gate (`tests/anchor_eval.rs`, live) is what vouches for it.
    let corpus = memory_proto::axes::corpus();
    assert_eq!(corpus.len(), 40, "four poles of ten");
    let corpus_texts: Vec<&str> = corpus.iter().map(|e| e.text.as_str()).collect();
    let corpus_embeddings = client
        .embed(&corpus_texts)
        .expect("embed the anchor corpus");
    let seeded_at = Utc::now();
    let mut per_function: std::collections::BTreeMap<memory_proto::axes::Function, usize> =
        std::collections::BTreeMap::new();
    let anchors: Vec<FunctionAnchor> = corpus
        .into_iter()
        .zip(corpus_embeddings)
        .map(|(e, embedding)| {
            let n = per_function.entry(e.function).or_insert(0);
            *n += 1;
            FunctionAnchor {
                id: format!("fa-{}-{n}", e.function.as_str()),
                function: e.function.as_str().to_string(),
                axis: e.function.axis().as_str().to_string(),
                exemplar: e.text.clone(),
                embedding: Some(embedding),
                active: true,
                created_at: seeded_at,
                updated_at: seeded_at,
                layer: 0,
                context: "demo".to_string(),
            }
        })
        .collect();
    memory_proto::mutations::seed_function_anchors(&conn, &anchors)
        .expect("seed the function anchors");

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
    let report = run_dream(&conn, &judge, &sermon, &NoDelivery, &spec);
    println!("\n=== dream report ===\n{report:#?}");

    // The self reading — the ledger's window. Numbers live here; the
    // sermon only ever speaks them as movement.
    if let Some(sp) = &report.self_pass {
        println!("\n=== the self, after this dream ===");
        match sp.user_position {
            Some((j, p)) => println!(
                "  you:        judgment {j:+.3}  perception {p:+.3}  (positive: thinking / sensing)"
            ),
            None => println!("  you:        unmeasurable this window"),
        }
        println!(
            "  wunicorm:   core ({:+.3}, {:+.3})  comp ({:+.3}, {:+.3})  position ({:+.3}, {:+.3})",
            sp.wunicorm_core.0,
            sp.wunicorm_core.1,
            sp.wunicorm_comp.0,
            sp.wunicorm_comp.1,
            sp.wunicorm_position.0,
            sp.wunicorm_position.1,
        );
        println!(
            "  rigidity:   judgment {:.2}  perception {:.2}  (compensation gated at 0.70)",
            sp.rigidity.0, sp.rigidity.1,
        );
        match sp.drift {
            Some((j, p)) => {
                println!("  drift:      judgment {j:+.3}  perception {p:+.3}");
            }
            None => println!("  drift:      first window — nothing to compare yet"),
        }
        println!(
            "  evidence:   {} kept, {} let go",
            sp.evidence_kept, sp.evidence_dropped
        );
    } else {
        println!("\nno self reading this run — anchors unseeded or a quiet day");
    }

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
