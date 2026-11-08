# Wunicorm v1 — Work Breakdown

## Where we are

The prototype build-out (schema, mutations, queries, vector search, drift,
snapshot, the dreamer, the LLM layer, the function space, self pass, sermon
voice, full eval) is **complete**. Its record lives in git history. The memory
layer is proven: LadybugDB, the four pillars, the Jungian function space, the
two-point model, the dream cycle.

**The daemon shell is now implemented.** Mac Notes ingress, daily scheduling,
notifications, Notes output, the tao menu-bar tray, and local app/DMG packaging
are in place. The tray was visually accepted, the DMG and signature were
verified, and all 177 tests pass with Clippy clean.

**Current phase: live end-to-end acceptance.** Task 9 remains pending: a full
scheduled dream against real Mac Notes and the local LLM servers has not yet
been verified. Tasks 1-8 and 10-13 are complete to the verification limits
recorded below. Task CTAs, launch at login, continuous ingestion, Developer ID
signing/notarization, and universal builds are deferred.

## What this list is: the daemon shell

The daemon shell wraps the proven memory core:

```
[ Mac Notes app ]
       |
       |  osascript (on schedule)
       v
[ INGRESS ]      -- pulls notes, detects changes, builds queue
       |
       v
[ SCHEDULER ]    -- tick every 30s, is it dream time?
       |
       v
[ MEMORY ]       -- existing dreamer + judge + graph (unchanged)
       |
       v
[ OUTPUT ]       -- macOS notification + write to "Wunicorm" Notes folder

[ TRAY ]         -- menu-bar icon on main thread, shows state
```

Single process. Main thread = tray (tao/Cocoa event loop). Worker thread = daemon
loop. State shared via `Arc<AtomicU8>`.

## Design principles

- **Memory layer is untouched.** Every existing module (axes, db, dreamer,
  edges, llm, nodes, queries, schema, vector, message, mutations,
  observability, extensions) stays exactly as-is. The daemon is a new consumer.
- **One process, one binary.** No Swift, no IPC, no separate apps. `tray-icon`
  on the main thread, the dream loop on a worker thread.
- **osascript is the Notes bridge.** Read notes, write notes, send
  notifications — all through `std::process::Command` invoking `osascript`.
- **Config is a TOML file.** `~/Library/Application Support/memory-proto/config.toml`. Created
  with defaults on first run. LLM URLs still overridable by env vars.
- **State is a small JSON file.** `~/Library/Application Support/memory-proto/state.json`
  holds `last_seen` (note ID → modification date) and `last_dream_date`.
- **The tray is the only UI.** Menu-bar icon with three states: idle,
  dreaming, notification. Menu: status, open folder, quit.
- **Failures are recorded, not fatal.** The daemon loop catches errors, logs
  them, and continues. The daemon never dies for one bad night.

## Key decisions

| Decision | Choice | Rationale |
|---|---|---|
| Dream time default | 22:00, configurable | Overnight pass |
| Folders to watch | All except "Wunicorm" and "Recently Deleted" | Auto-discover; user doesn't configure |
| Tray icon | Generated monochrome PNG | Placeholder until a real glyph |
| Directory name | `memory-proto/` (unchanged) | Don't touch git history paths |
| Package name | `memory-proto` (unchanged) | Same reason |
| Note identity | Core Data URI (`id of note`) | Stable across renames |
| Change detection | Modification date comparison | Cheap; no content hashing |
| Note body | HTML → strip to text | LLM wants text, not markup |
| DB location | `~/Library/Application Support/memory-proto/memory.db` | macOS platform data directory |
| Threading | Main = tray; worker = daemon | `tray-icon` needs main thread on macOS |
| Shutdown | SIGINT/SIGTERM or tray "Quit" → daemon thread exits | Graceful |

---

## Tasks

### Task 1: Restructure — move demo to examples, stub main.rs

**What:** Clear the main.rs slot for the daemon. The existing demo (§8 story)
becomes an example binary.

**Details:**
- Move `src/main.rs` → `examples/demo.rs` (unchanged, still runs the §8 story)
- New `src/main.rs`: prints a version banner and exits. No daemon yet.
- `cargo build` + `cargo test` green.

**Files:** `src/main.rs`, `examples/demo.rs`

**Commit:** `refactor: move demo to examples, stub daemon main`

**Depends on:** nothing

---

### Task 2: Config layer

**What:** A `Config` struct read from `~/Library/Application Support/memory-proto/config.toml`.
Creates the file with defaults on first run.

**Details:**
- `src/config.rs`: `Config` struct + `Config::load() -> Result<Self>`.
- Fields:
  - `dream_time: String` — default `"22:00"`, parsed to `(u8, u8)` (hour, min)
  - `db_path: String` — default `~/Library/Application Support/memory-proto/memory.db`
  - `state_path: String` — default `~/Library/Application Support/memory-proto/state.json`
  - `excluded_folders: Vec<String>` — default `["Wunicorm", "Recently Deleted"]`
  - `[llm]` block: `chat_base_url`, `chat_model`, `embed_base_url`,
    `embed_model` — all optional, env vars override
- `Config::load()` creates parent dirs, writes defaults if the file is missing.
- Env var overrides: `WUNICORM_CHAT_URL`, `WUNICORM_CHAT_MODEL`,
  `WUNICORM_EMBED_URL`, `WUNICORM_EMBED_MODEL` (existing `LlmConfig::from_env`
  already does this).

**Files:** `src/config.rs` (new), `src/lib.rs`

**Tests:** default creation, parse valid TOML, env override.

**Commit:** `feat: add config layer (TOML, defaults, env overrides)`

**Depends on:** Task 1

---

### Task 3: Notes Ingress — osascript bridge

**What:** Read all notes from Mac Notes via osascript, detect which are new
or changed, return a queue ready for the dreamer.

**Details:**
- `src/ingest.rs`:
  - `RawNote { id: String, name: String, modified: DateTime<Utc>, body_html: String }`
  - `pull_notes(folders: &[String]) -> Result<Vec<RawNote>>`:
    Builds and runs one AppleScript that iterates all folders except the
    excluded list, for each note emits tab-delimited:
    `<id>\t<name>\t<modification_date>\t<body_html>`
    Parses stdout into `Vec<RawNote>`.
  - `html_to_text(html: &str) -> String`: strip tags (regex `<[^>]+>`),
    unescape `&amp; &lt; &gt; &quot; &#39; &nbsp;`.
  - `detect_changes(raw: &[RawNote], last_seen: &BTreeMap<String, DateTime<Utc>>) -> Vec<Note>`:
    New notes (id not in map) + changed notes (modified > stored) → `Vec<Note>`.
    Note.id = Core Data URI. Note.content = stripped text.
  - `update_last_seen(map, raw)` — update the map after a successful dream.
- `last_seen` is a `BTreeMap<String, DateTime<Utc>>` persisted as JSON in the
  state file (Task 4 owns the file).

**AppleScript:**
```applescript
tell application "Notes"
    set output to ""
    repeat with f in (every folder whose name is not "Wunicorm" and name is not "Recently Deleted")
        set theNotes to notes of f
        repeat with i from 1 to (count of theNotes)
            set n to item i of theNotes
            set output to output & (id of n) & tab & (name of n) & tab & (modification date of n) & tab & (body of n) & linefeed
        end repeat
    end repeat
    return output
end tell
```

**Files:** `src/ingest.rs` (new), `src/lib.rs`, `Cargo.toml` (add `regex`)

**Tests:** fixture output parsing, HTML stripping, change detection
(new/changed/unchanged), tab-in-body handling.

**Commit:** `feat: add notes ingress (osascript bridge, change detection)`

**Depends on:** Task 1

---

### Task 4: State persistence + Scheduler

**What:** A small JSON state file that tracks what the daemon has seen and
when it last dreamed. Plus the scheduler that decides "is it dream time?"

**Details:**
- `src/scheduler.rs`:
  - `DaemonState { last_seen: BTreeMap<String, DateTime<Utc>>, last_dream_date: Option<NaiveDate> }`
  - `DaemonState::load(path) -> Self` — read or default.
  - `DaemonState::save(path)` — write JSON atomically (temp + rename).
  - `Scheduler { dream_hour: u8, dream_minute: u8 }`
  - `Scheduler::from_config(config) -> Self`
  - `is_dream_time(&self, now: DateTime<Local>, last_dream_date: Option<NaiveDate>) -> bool`
    — true when `now.hour() >= dream_hour && now.minute() >= dream_minute`
    AND `last_dream_date != Some(today)`.

**Files:** `src/scheduler.rs` (new), `src/lib.rs`

**Tests:** scheduler fires at the right time, once-per-day guard, state
save/load roundtrip, missing file → defaults.

**Commit:** `feat: add scheduler and state persistence`

**Depends on:** Tasks 1, 2, 3

---

### Task 5: Output — notifications + Notes writer

**What:** Send a macOS notification and write a note back into Mac Notes.

**Details:**
- `src/output.rs`:
  - `notify(title: &str, body: &str) -> Result<()>` — `osascript -e
    'display notification "..." with title "..."'`. Escape double quotes.
  - `ensure_notes_folder(name: &str) -> Result<()>` — create if missing.
  - `write_note(title: &str, html_body: &str, folder: &str) -> Result<()>` —
    `make new note at folder ... with properties {name:..., body:...}`.
  - `sermon_to_html(text: &str) -> String` — wrap paragraphs in `<div>`.
- All output functions take a `dry_run: bool` flag — when true, log the
  command instead of executing (for CI/tests).

**Files:** `src/output.rs` (new), `src/lib.rs`

**Tests:** escaping (quotes in title/body), dry-run produces correct command
strings.

**Commit:** `feat: add output layer (macOS notifications, Notes writer)`

**Depends on:** Task 1

---

### Task 6: Tray icon

**What:** The menu-bar icon. Three visual states. A simple menu.

**Details:**
- Generate three 22x22 monochrome PNGs (a small horn/spiral glyph):
  - `assets/tray-idle.png` — normal
  - `assets/tray-dreaming.png` — with a filled dot indicator
  - `assets/tray-notify.png` — with a badge dot
- `src/tray.rs`:
  - `TrayState` enum: Idle, Dreaming, Notification
  - `run_tray(state: Arc<AtomicU8>) -> !` — runs on the main thread.
    Creates the `tray-icon`, menu items, enters the event loop.
    Polls the atomic state every ~1s, swaps the icon on change.
  - Menu items:
    - "Wunicorm — Running" (disabled)
    - "Open Wunicorm Folder" (osascript to open Notes folder)
    - Separator
    - "Quit" (sets a shutdown flag, exits the process)
- `IconState` constants: `IDLE = 0`, `DREAMING = 1`, `NOTIFY = 2`.

**Files:** `src/tray.rs` (new), `src/lib.rs`, `assets/tray-*.png` (new),
`Cargo.toml` (add `tray-icon`)

**Tests:** none (UI; manual verification in Task 9).

**Commit:** `feat: add tray icon with state indicator`

**Depends on:** Task 1

---

### Task 7: Daemon loop

**What:** The worker thread. The main loop: sleep, check time, dream, output.

**Details:**
- `src/daemon.rs`:
  - `run_daemon(config: Config, state: Arc<AtomicU8>, shutdown: Arc<AtomicBool>)`
  - Loop:
    1. Set tray state to Idle.
    2. Sleep 30 seconds (check shutdown flag every 1s).
    3. Load `DaemonState` from disk.
    4. `if scheduler.is_dream_time(now, state.last_dream_date)`:
       a. Set tray state to Dreaming.
       b. `ingest::pull_notes(&config.excluded_folders)`
       c. `ingest::detect_changes(&raw, &state.last_seen)` → queue
       d. If queue is non-empty:
          - Embed note contents via the LLM client.
          - Build `DreamSpec` (run_id = timestamp, drift_windows from config,
            outbox = data dir / "outbox", dream_log = data dir / "dream-log.jsonl")
          - `run_dream(&conn, &judge, &sermon, &spec)`
          - If `report.message_written`: read the sermon from the outbox,
            `output::notify("Sermon ready", ...)`,
            `output::write_note(...)` to the "Wunicorm" folder,
            set tray state to Notification.
       e. `state.update_last_seen(&raw)`, `state.last_dream_date = Some(today)`,
          `state.save(...)`.
       f. Set tray state to Idle (or Notification if a message was written).
    5. Catch all errors, log to stderr, continue the loop.
  - The DB connection is opened once at the start and held for the daemon's
    lifetime.

**Files:** `src/daemon.rs` (new), `src/lib.rs`

**Tests:** none (integration; manual verification in Task 9).

**Commit:** `feat: add daemon loop (sleep, check, dream, output)`

**Depends on:** Tasks 2, 3, 4, 5

---

### Task 8: Wire in main.rs

**What:** The real entry point. Loads config, opens DB, seeds anchors, starts
tray and daemon, handles shutdown.

**Details:**
- `src/main.rs`:
  1. `Config::load()`
  2. `MemoryDb::open(&config.db_path, EMBEDDING_DIM)`
  3. `schema::init(&conn)` (migrates in place)
  4. `vector::init_vector_indexes(&conn)`
  5. Seed function anchors (idempotent — check if already seeded)
  6. Create `Arc<AtomicU8>` (tray state) and `Arc<AtomicBool>` (shutdown)
  7. Spawn daemon thread with config + state + db + shutdown flag
  8. Call `tray::run_tray(state, shutdown)` on the main thread (blocks)
  9. On tray exit: set shutdown, join daemon thread
- Signal handling: `ctrlc` crate or `signal-hook` to set the shutdown flag
  on SIGINT/SIGTERM.

**Files:** `src/main.rs`, `Cargo.toml` (add `ctrlc` or `signal-hook`)

**Tests:** `cargo build` green.

**Commit:** `feat: wire daemon entry point (config, db, tray, worker)`

**Depends on:** Tasks 6, 7

---

### Task 9: End-to-end verify — PENDING

**What:** Run the daemon, confirm the full cycle works. Component verification
and the demo do not complete this task; it requires a real scheduled dream.

**Details:**
- Build release: `cargo build --release`
- Run: `WUNICORM_CHAT_URL=... ./target/release/memory-proto`
- Confirm:
  - Tray icon appears in the menu bar
  - Set dream_time to 1 minute ahead in config
  - Wait for the dream to trigger
  - Confirm notes are pulled (check stderr logs)
  - Confirm dreamer runs (watch the dream log)
  - Confirm notification fires
  - Confirm a note appears in the "Wunicorm" folder in Mac Notes
  - Re-run same day: no re-dream (once-per-day guard)
  - Re-run next day: dreams again with new/changed notes only
- `cargo clippy --all-targets -- -D warnings` clean.

**Files:** none (verification only)

**Commit:** `chore: verify end-to-end daemon cycle`

**Depends on:** Task 8

---

### Task 10: Fix the tray runtime

**What:** Replace the sleep loop with a real macOS application event loop.

**Details:**
- Add `tao`; use the Accessory activation policy and no Dock presence.
- Create the tray on `StartCause::Init`, forward menu events through
  `EventLoopProxy`, wake the main Core Foundation run loop, and poll the shared
  icon state with `ControlFlow::WaitUntil`.
- Quit returns through `run_return` so `main` can join the daemon thread.

**Files:** `src/tray.rs`, `Cargo.toml`, `Cargo.lock`

**Verification:** build and Clippy clean; the packaged process stays running;
the menu-bar icon was visually confirmed by the user.

**Commit:** `fix: run tray in a macOS application event loop`

**Depends on:** Task 8 (developed while Task 9 live acceptance remains pending)

---

### Task 11: Add the macOS app bundle definition

**What:** Metadata and a generated application icon for a menu-bar-only app.

**Details:**
- `packaging/macos/Info.plist`: bundle ID `com.wunicorm.app`, executable
  `wunicorm`, `LSUIElement=true`, and a Notes automation usage description.
- `packaging/macos/AppIcon.png`: generated 1024×1024 placeholder spiral.

**Files:** `packaging/macos/Info.plist`, `packaging/macos/AppIcon.png`

**Verification:** `plutil -lint` passes; the generated image is inspected.

**Commit:** `feat: add macOS app bundle metadata`

**Depends on:** Task 10

---

### Task 12: Add local DMG packaging

**What:** One reproducible script builds the release app and a compressed,
drag-to-install DMG for the current architecture.

**Details:**
- `scripts/package-macos.sh` builds release, assembles `Wunicorm.app`, generates
  `AppIcon.icns`, injects the Cargo version, and creates an Applications symlink.
- Vendor LadybugDB's `libssl`/`libcrypto`, rewrite the executable and dylib load
  paths to `@rpath`, include the OpenSSL license, and reject remaining
  machine-local dependencies.
- Ad-hoc sign, verify the signature, create the DMG, and run `hdiutil verify`.
  All generated artifacts stay under `target/packaging/`.

**Files:** `scripts/package-macos.sh`

**Verification:** produced `Wunicorm-0.1.0-arm64.dmg`; bundle and image checks
pass.

**Commit:** `feat: add local macOS DMG packaging script`

**Depends on:** Task 11

---

### Task 13: Verify installation and document it

**What:** Verify the packaged artifact and record how to build and install it.

**Details:**
- All 177 Rust tests pass; Clippy is clean.
- Mount the DMG read-only; verify its executable, Applications symlink, plist,
  and app signature.
- Launch the packaged app and verify that its process remains running. The
  menu-bar icon was subsequently visually confirmed by the user.
- README documents daemon/demo commands, packaging, installation, permissions,
  data preservation, and the local-signing limitation.

**Files:** `README.md`, `AGILE.md`

**Commit:** `docs: document macOS DMG installation`

**Depends on:** Task 12

---

## Commit sequence

```
Task 1 → refactor: move demo to examples, stub daemon main
Task 2 → feat: add config layer (TOML, defaults, env overrides)
Task 3 → feat: add notes ingress (osascript bridge, change detection)
Task 4 → feat: add scheduler and state persistence
Task 5 → feat: add output layer (macOS notifications, Notes writer)
Task 6 → feat: add tray icon with state indicator
Task 7 → feat: add daemon loop (sleep, check, dream, output)
Task 8 → feat: wire daemon entry point (config, db, tray, worker)
Task 9 (pending) → chore: verify end-to-end daemon cycle
Task 10 → fix: run tray in a macOS application event loop
Task 11 → feat: add macOS app bundle metadata
Task 12 → feat: add local macOS DMG packaging script
Task 13 → docs: document macOS DMG installation
```

Each commit: `cargo clippy --all-targets -- -D warnings` clean, `cargo tst`
green (alias for `test -- --test-threads=1`).

---

## Resolved decisions

| Question | Decision |
|---|---|
| Web app | Removed. Mac Notes is the input surface. |
| Tailscale | Removed. Local-only. |
| Notes DB | Mac Notes (the app's own storage). No separate notes store. |
| Note ID | Core Data URI from AppleScript (`id of note`) |
| Change detection | Modification date comparison (not content hashing) |
| HTML handling | Strip to text for the LLM; keep HTML for writing back to Notes |
| Threading | Main = tray (Cocoa); worker = daemon loop |
| State file | `~/Library/Application Support/memory-proto/state.json` on macOS (last_seen + last_dream_date) |
| DB path | `~/Library/Application Support/memory-proto/memory.db` on macOS |
| Config path | `~/Library/Application Support/memory-proto/config.toml` on macOS |
| Tray icon format | 22x22 monochrome PNG (macOS template) |
| Tray runtime | `tao` main-thread event loop; tray created on `StartCause::Init`; Accessory activation policy |
| App bundle | `Wunicorm.app`, `com.wunicorm.app`, `LSUIElement=true` |
| DMG architecture | Current Mac only for v1 |
| DMG signing | Local ad-hoc signature; Developer ID/notarization deferred |
| Native dependencies | Vendor and re-sign OpenSSL dylibs; rewrite load paths to `@rpath`; include license |
| Live acceptance | Pending: a complete scheduled dream against real Mac Notes and local LLM servers |
| Task CTAs / task engine | Deferred |
| Launch at login | Deferred; no LaunchAgent installed |
| Distribution | Current architecture and ad-hoc signing only; universal builds and notarization deferred |
| Shutdown | SIGINT/SIGTERM or tray Quit → graceful join |
| Error handling | Log to stderr, continue the loop. Never die for one bad night. |
