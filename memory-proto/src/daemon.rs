//! The daemon loop — the worker thread that runs the pipeline.
//!
//! Sleep → check schedule → pull notes → detect changes → embed →
//! dream → output (notification + Notes writer). Errors are logged
//! and the loop continues.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Local, Timelike, Utc};

use crate::config::Config;
use crate::db::MemoryDb;
use crate::dreamer::{run_dream, DreamSpec, TimeWindows};
use crate::ingest;
use crate::llm::{LlmClient, LlmConfig, LlmJudge, LlmSermonWriter, UreqClient};
use crate::nodes::FunctionAnchor;
use crate::output;
use crate::scheduler::{DaemonState, Scheduler};
use crate::schema::{self, EMBEDDING_DIM};
use crate::tray;
use crate::{extensions, vector};

/// How often the daemon checks the clock (seconds).
const TICK_SECS: u64 = 30;

/// Run the daemon loop on the current thread. Blocks until shutdown.
pub fn run_daemon(config: &Config, tray_state: &Arc<AtomicU8>, shutdown: &Arc<AtomicBool>) {
    eprintln!("[daemon] starting");

    // Open the database and run migrations.
    let db_path = config.resolved_db_path();
    let db = match MemoryDb::open(&db_path, EMBEDDING_DIM) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[daemon] failed to open database: {e}");
            return;
        }
    };
    let conn = match db.connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[daemon] failed to connect: {e}");
            return;
        }
    };
    if let Err(e) = extensions::load_extensions(&conn) {
        eprintln!("[daemon] extension load: {e}");
    }
    if let Err(e) = schema::init(&conn) {
        eprintln!("[daemon] schema init: {e}");
        return;
    }
    if let Err(e) = vector::init_vector_indexes(&conn) {
        eprintln!("[daemon] vector init: {e}");
        return;
    }

    // Seed function anchors if not already seeded.
    seed_anchors_if_needed(&conn);

    let scheduler = Scheduler::from_config(config);
    let state_path = config.resolved_state_path();
    let outbox = db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("outbox");
    let dream_log = db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("dream-log.jsonl");
    let _ = std::fs::create_dir_all(&outbox);

    eprintln!(
        "[daemon] ready. dream time: {}, db: {}",
        config.dream_time,
        db_path.display()
    );

    loop {
        if shutdown.load(Ordering::Relaxed) {
            eprintln!("[daemon] shutdown signal received");
            break;
        }
        tray_state.store(tray::ICON_IDLE, Ordering::Relaxed);

        // Sleep in small increments so we can check the shutdown flag.
        for _ in 0..TICK_SECS {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }

        let now = Local::now();
        let mut state = DaemonState::load(&state_path).unwrap_or_default();

        if !scheduler.is_dream_time(now, state.last_dream_date) {
            continue;
        }

        eprintln!("[daemon] dream time — starting dream cycle");
        tray_state.store(tray::ICON_DREAMING, Ordering::Relaxed);

        match dream_cycle(&conn, config, &mut state, &outbox, &dream_log) {
            Ok(sermon_written) => {
                if sermon_written {
                    tray_state.store(tray::ICON_NOTIFY, Ordering::Relaxed);
                } else {
                    tray_state.store(tray::ICON_IDLE, Ordering::Relaxed);
                }
            }
            Err(e) => {
                eprintln!("[daemon] dream cycle error: {e}");
                tray_state.store(tray::ICON_IDLE, Ordering::Relaxed);
            }
        }

        // Mark today as dreamed, save state.
        state.last_dream_date = Some(now.date_naive());
        if let Err(e) = state.save(&state_path) {
            eprintln!("[daemon] state save error: {e}");
        }
    }

    eprintln!("[daemon] stopped");
}

/// Run one dream cycle: pull → detect → embed → dream → output.
///
/// Returns `true` if a sermon was written to the outbox.
fn dream_cycle(
    conn: &lbug::Connection,
    config: &Config,
    state: &mut DaemonState,
    outbox: &Path,
    dream_log: &Path,
) -> Result<bool, String> {
    // 1. Pull notes from Mac Notes.
    let raw_notes =
        ingest::pull_notes(&config.excluded_folders).map_err(|e| format!("pull notes: {e}"))?;
    eprintln!("[dream] pulled {} notes from Mac Notes", raw_notes.len());

    // 2. Detect changes.
    let queue = ingest::detect_changes(&raw_notes, &state.last_seen);
    eprintln!("[dream] {} new/changed notes", queue.len());

    if queue.is_empty() {
        eprintln!("[dream] no changes — quiet night");
        ingest::update_last_seen(&mut state.last_seen, &raw_notes);
        return Ok(false);
    }

    // 3. Embed the queue contents.
    let llm_config = build_llm_config(config);
    let client = UreqClient::new(llm_config.clone());
    let contents: Vec<&str> = queue.iter().map(|n| n.content.as_str()).collect();
    let embeddings = client
        .embed(&contents)
        .map_err(|e| format!("embed notes: {e}"))?;
    let anchor = client
        .embed(&["general"])
        .map_err(|e| format!("embed anchor: {e}"))?
        .remove(0);

    let mut queue = queue;
    for (n, emb) in queue.iter_mut().zip(embeddings) {
        n.content_embedding = Some(emb);
    }

    // 4. Build drift windows (current month vs previous month).
    let windows = build_drift_windows();

    // 5. Run the dream.
    let judge = LlmJudge::new(Box::new(client));
    let sermon = LlmSermonWriter::new(Box::new(UreqClient::new(llm_config)));
    let run_id = format!("dream-{}", Utc::now().format("%Y%m%d-%H%M%S"));
    let spec = DreamSpec {
        run_id: run_id.clone(),
        queue,
        anchor_embedding: anchor,
        drift_windows: Some(windows),
        outbox: outbox.to_path_buf(),
        dream_log: dream_log.to_path_buf(),
    };

    eprintln!("[dream] running dream cycle (run_id={run_id})...");
    let report = run_dream(conn, &judge, &sermon, &spec);
    eprintln!(
        "[dream] complete: {} nodes written, message={}, shelf_ok={}",
        report.nodes_written, report.message_written, report.shelf_ok
    );

    // 6. Output: notification + write to Notes.
    if report.message_written {
        let sermon_path = outbox.join(format!("{run_id}-sermon.json"));
        if let Ok(content) = std::fs::read_to_string(&sermon_path)
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
        {
            let body = json["body"].as_str().unwrap_or("(no body)");
            let preview: String = body.chars().take(80).collect();
            let _ = output::notify("Wunicorm", &preview, false);
            let html = output::sermon_to_html(body);
            let title = format!("Sermon — {}", Utc::now().format("%Y-%m-%d"));
            let _ = output::ensure_notes_folder("Wunicorm", false);
            let _ = output::write_note(&title, &html, "Wunicorm", false);
            eprintln!("[dream] sermon written to Notes + notification sent");
        }
    }

    // 7. Update last_seen.
    ingest::update_last_seen(&mut state.last_seen, &raw_notes);
    Ok(report.message_written)
}

/// Build the LLM config, applying config-file overrides on top of
/// environment defaults.
fn build_llm_config(config: &Config) -> LlmConfig {
    let mut llm = LlmConfig::from_env();
    if let Some(ref url) = config.llm.chat_base_url {
        llm.chat_base_url.clone_from(url);
    }
    if let Some(ref model) = config.llm.chat_model {
        llm.chat_model.clone_from(model);
    }
    if let Some(ref url) = config.llm.embed_base_url {
        llm.embed_base_url.clone_from(url);
    }
    if let Some(ref model) = config.llm.embed_model {
        llm.embed_model.clone_from(model);
    }
    llm
}

/// Build drift windows: current month vs previous month.
fn build_drift_windows() -> TimeWindows {
    let now = Utc::now();
    let current_start = now
        .with_day(1)
        .unwrap_or(now)
        .with_hour(0)
        .and_then(|d| d.with_minute(0))
        .and_then(|d| d.with_second(0))
        .unwrap_or(now);
    let current_end = now;
    // Previous month: 30 days back.
    let prev_end = current_start;
    let prev_start = current_start - chrono::Duration::days(30);
    ((prev_start, prev_end), (current_start, current_end))
}

/// Seed function anchors if the table is empty.
fn seed_anchors_if_needed(conn: &lbug::Connection) {
    // Check if anchors are already seeded.
    let result = conn.query("MATCH (fa:FunctionAnchor) RETURN count(fa)");
    if let Ok(mut qr) = result
        && let Some(row) = qr.next()
        && let Some(count) = row.first()
        && let lbug::Value::Int64(n) = count
        && *n > 0
    {
        eprintln!("[daemon] anchors already seeded ({n} rows)");
        return;
    }

    // Seed from the corpus.
    eprintln!("[daemon] seeding function anchors...");
    let llm_config = LlmConfig::from_env();
    let client = UreqClient::new(llm_config);
    let corpus = crate::axes::corpus();
    let corpus_texts: Vec<&str> = corpus.iter().map(|e| e.text.as_str()).collect();
    match client.embed(&corpus_texts) {
        Ok(embeddings) => {
            let seeded_at = Utc::now();
            let mut per_function: std::collections::BTreeMap<crate::axes::Function, usize> =
                std::collections::BTreeMap::new();
            let anchors: Vec<FunctionAnchor> = corpus
                .into_iter()
                .zip(embeddings)
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
                        context: "daemon".to_string(),
                    }
                })
                .collect();
            if let Err(e) = crate::mutations::seed_function_anchors(conn, &anchors) {
                eprintln!("[daemon] anchor seed error: {e}");
            } else {
                eprintln!("[daemon] seeded {} function anchors", anchors.len());
            }
        }
        Err(e) => {
            eprintln!("[daemon] anchor embed error (self layer disabled): {e}");
        }
    }
}
