//! Output layer — macOS notifications and writing notes back to Mac Notes.
//!
//! All functions shell out to `osascript`. A `dry_run` flag logs the
//! command instead of executing (for tests and CI).
//!
//! `AppleScript` gotcha this module encodes: Notes' `whose`-filter
//! lookups (`count of folders whose name is "X"`) throw -1728
//! ("Can't get 3 whose name…") against the application-level folder
//! list. Every folder/account reference here is therefore scoped to an
//! explicit account and uses `exists folder` — the pattern verified
//! against a live Notes app.

use std::fmt::Write;
use std::process::Command;

/// The folder the daemon writes sermons into (and excludes from
/// ingress, so it never dreams its own output).
pub const SERMON_FOLDER: &str = "Wunicorm";

/// Output errors.
#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("osascript failed: {0}")]
    Osascript(String),
    #[error("no usable Notes account: {0}")]
    NoAccount(String),
}

/// Resolve the Notes account that receives sermons. An explicit config
/// value wins unchecked (a wrong name then fails loudly downstream);
/// otherwise the first account holding folders, else the first account.
pub fn resolve_notes_account(explicit: Option<&str>, dry_run: bool) -> Result<String, OutputError> {
    if let Some(account) = explicit {
        return Ok(account.to_string());
    }
    if dry_run {
        return Ok("dry-run-account".to_string());
    }
    let script = r#"tell application "Notes"
    repeat with a in every account
        if (count of folders of a) > 0 then return name of a
    end repeat
    return name of account 1
end tell"#;
    let stdout = run_osascript_capture(script)?;
    let name = stdout.lines().next().unwrap_or("").trim().to_string();
    if name.is_empty() {
        return Err(OutputError::NoAccount(
            "Notes reported no accounts".to_string(),
        ));
    }
    Ok(name)
}

/// Send a macOS notification.
///
/// `dry_run = true` logs the command without executing.
pub fn notify(title: &str, body: &str, dry_run: bool) -> Result<(), OutputError> {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        escape_applescript(body),
        escape_applescript(title),
    );
    if dry_run {
        eprintln!("[dry-run] notify: {script}");
        return Ok(());
    }
    run_osascript(&script)
}

/// Ensure a folder exists in the given Mac Notes account. Creates it if
/// missing. Idempotent: `exists folder` never uses the broken
/// application-level `whose` lookup.
pub fn ensure_notes_folder(account: &str, name: &str, dry_run: bool) -> Result<(), OutputError> {
    let script = format!(
        r#"tell application "Notes"
    tell account "{}"
        if not (exists folder "{}") then
            make new folder with properties {{name:"{}"}}
        end if
    end tell
end tell"#,
        escape_applescript(account),
        escape_applescript(name),
        escape_applescript(name),
    );
    if dry_run {
        eprintln!("[dry-run] ensure_folder: {script}");
        return Ok(());
    }
    run_osascript(&script)
}

/// Write a note into a Mac Notes folder, scoped to the account.
///
/// The body should be HTML (the Notes app renders it).
pub fn write_note(
    account: &str,
    title: &str,
    html_body: &str,
    folder: &str,
    dry_run: bool,
) -> Result<(), OutputError> {
    let script = format!(
        r#"tell application "Notes"
    tell account "{}"
        make new note at folder "{}" with properties {{name:"{}", body:"{}"}}
    end tell
end tell"#,
        escape_applescript(account),
        escape_applescript(folder),
        escape_applescript(title),
        escape_applescript(html_body),
    );
    if dry_run {
        eprintln!(
            "[dry-run] write_note: title={title}, folder={folder}, body_len={}",
            html_body.len()
        );
        return Ok(());
    }
    run_osascript(&script)
}

/// The delivery read-back: is a note with this exact title present in
/// the account's folder? Nothing but the live check counts as green.
pub fn note_present(
    account: &str,
    folder: &str,
    title: &str,
    dry_run: bool,
) -> Result<bool, OutputError> {
    if dry_run {
        eprintln!("[dry-run] note_present: title={title}, folder={folder}");
        return Ok(true);
    }
    // The names are joined inside the script, one per line: osascript
    // renders a returned AppleScript list as one comma-joined line, and
    // titles may themselves contain commas. Newlines cannot appear in
    // a note title, so line membership is title membership.
    let script = format!(
        r#"tell application "Notes"
    tell account "{account_esc}"
        set out to ""
        repeat with n in (every note of folder "{folder_esc}")
            set out to out & (name of n) & linefeed
        end repeat
        return out
    end tell
end tell"#,
        account_esc = escape_applescript(account),
        folder_esc = escape_applescript(folder),
    );
    let stdout = run_osascript_capture(&script)?;
    Ok(stdout.lines().map(str::trim).any(|l| l == title))
}

/// Convert a plain-text sermon body to minimal HTML for the Notes app.
pub fn sermon_to_html(text: &str) -> String {
    let mut html = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            html.push_str("<div><br></div>\n");
        } else {
            // Escape HTML entities in the text.
            let escaped = line
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            let _ = writeln!(html, "<div>{escaped}</div>");
        }
    }
    html
}

/// Escape a string for embedding in an `AppleScript` string literal.
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Run an osascript and check for success.
fn run_osascript(script: &str) -> Result<(), OutputError> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| OutputError::Osascript(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OutputError::Osascript(stderr.to_string()));
    }
    Ok(())
}

/// Run an osascript and return its stdout, checking for success.
fn run_osascript_capture(script: &str) -> Result<String, OutputError> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| OutputError::Osascript(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OutputError::Osascript(stderr.to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_applescript_quotes() {
        assert_eq!(escape_applescript(r#"say "hello""#), r#"say \"hello\""#);
    }

    #[test]
    fn escape_applescript_newlines() {
        assert_eq!(escape_applescript("line1\nline2"), r"line1\nline2");
    }

    #[test]
    fn escape_applescript_backslash() {
        assert_eq!(escape_applescript(r"path\to"), r"path\\to");
    }

    #[test]
    fn sermon_to_html_basic() {
        let html = sermon_to_html("Hello\nWorld");
        assert!(html.contains("<div>Hello</div>"));
        assert!(html.contains("<div>World</div>"));
    }

    #[test]
    fn sermon_to_html_blank_lines() {
        let html = sermon_to_html("Hello\n\nWorld");
        assert!(html.contains("<div><br></div>"));
    }

    #[test]
    fn sermon_to_html_escapes_entities() {
        let html = sermon_to_html("a < b & c > d");
        assert!(html.contains("a &lt; b &amp; c &gt; d"));
    }

    #[test]
    fn notify_dry_run() {
        // Should not panic or error.
        notify("Test", "Body", true).unwrap();
    }

    #[test]
    fn ensure_folder_dry_run() {
        ensure_notes_folder("Acct", "Wunicorm", true).unwrap();
    }

    #[test]
    fn write_note_dry_run() {
        write_note("Acct", "Title", "<div>body</div>", "Wunicorm", true).unwrap();
    }

    #[test]
    fn note_present_dry_run_trusts() {
        assert!(note_present("Acct", "Wunicorm", "Title", true).unwrap());
    }

    #[test]
    fn explicit_account_wins_unchecked() {
        assert_eq!(
            resolve_notes_account(Some("iCloud"), true).unwrap(),
            "iCloud"
        );
    }

    #[test]
    fn resolve_account_dry_run() {
        assert_eq!(
            resolve_notes_account(None, true).unwrap(),
            "dry-run-account"
        );
    }

    /// The only proof that counts for the `AppleScript` paths: a live
    /// round-trip against the real Notes app. `#[ignore]`d so the
    /// hermetic suite never touches the machine; run manually with
    /// `cargo test -- --ignored` on a Mac with Notes. Creates a probe
    /// note in the sermon folder and deletes it again.
    #[test]
    #[ignore = "needs a live Notes app and automation permission; run manually"]
    fn live_delivery_roundtrip_against_real_notes() {
        let account = resolve_notes_account(None, false).unwrap();
        ensure_notes_folder(&account, SERMON_FOLDER, false).unwrap();
        // Idempotent: a second call must not fail or duplicate.
        ensure_notes_folder(&account, SERMON_FOLDER, false).unwrap();

        let title = "Wunicorm delivery probe — safe to delete";
        write_note(&account, title, "<div>probe</div>", SERMON_FOLDER, false).unwrap();
        assert!(note_present(&account, SERMON_FOLDER, title, false).unwrap());
        assert!(!note_present(&account, SERMON_FOLDER, "no such note", false).unwrap());

        // Clean up: delete the probe note, leave the folder. Iterate by
        // descending index — deleting while looping a live list shifts
        // positions, and no `whose` is used even in cleanup.
        let script = format!(
            r#"tell application "Notes"
    tell account "{}"
        tell folder "{}"
            repeat with i from (count of notes) to 1 by -1
                if name of note i is "{}" then delete note i
            end repeat
        end tell
    end tell
end tell"#,
            escape_applescript(&account),
            escape_applescript(SERMON_FOLDER),
            escape_applescript(title),
        );
        run_osascript(&script).expect("probe cleanup failed — delete it by hand");
    }
}
