//! Wunicorm daemon — the entry point.
//!
//! Loads config, opens the database, starts the tray icon on the
//! main thread and the daemon loop on a worker thread. Handles
//! graceful shutdown on SIGINT/SIGTERM.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use memory_proto::config::Config;
use memory_proto::daemon;
use memory_proto::tray;

fn main() {
    eprintln!("wunicorm daemon starting...");

    // Load configuration (creates default file on first run).
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("config error: {e}");
            std::process::exit(1);
        }
    };
    eprintln!(
        "config: dream_time={}, db={}",
        config.dream_time, config.db_path
    );

    // Shared state between tray and daemon.
    let tray_state = Arc::new(AtomicU8::new(tray::ICON_IDLE));
    let shutdown = Arc::new(AtomicBool::new(false));

    // Signal handler: set the shutdown flag on SIGINT/SIGTERM.
    let shutdown_signal = shutdown.clone();
    ctrlc::set_handler(move || {
        eprintln!("\nshutting down...");
        shutdown_signal.store(true, Ordering::Relaxed);
        // Give the daemon a moment to finish, then exit.
        std::thread::sleep(std::time::Duration::from_secs(2));
        std::process::exit(0);
    })
    .expect("set signal handler");

    // Spawn the daemon worker thread.
    let daemon_config = config.clone();
    let daemon_state = tray_state.clone();
    let daemon_shutdown = shutdown.clone();
    let daemon_handle = std::thread::spawn(move || {
        daemon::run_daemon(&daemon_config, &daemon_state, &daemon_shutdown);
    });

    // Run the tray on the main thread (Cocoa event loop).
    // This blocks until the user clicks Quit or we receive a signal.
    tray::run_tray(&tray_state, &shutdown);

    // If we get here, the tray exited. Signal the daemon to stop.
    shutdown.store(true, Ordering::Relaxed);
    let _ = daemon_handle.join();

    eprintln!("wunicorm daemon stopped");
}
