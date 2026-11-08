//! Output layer — macOS notifications and writing notes back to Mac Notes.
//!
//! All functions shell out to `osascript`. A `dry_run` flag logs the
//! command instead of executing (for tests and CI).

use std::fmt::Write;
use std::process::Command;

/// Output errors.
#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("osascript failed: {0}")]
    Osascript(String),
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

/// Ensure a folder exists in Mac Notes. Creates it if missing.
pub fn ensure_notes_folder(name: &str, dry_run: bool) -> Result<(), OutputError> {
    let script = format!(
        r#"tell application "Notes"
    if (count of folders whose name is "{}") = 0 then
        make new folder with properties {{name:"{}"}}
    end if
end tell"#,
        escape_applescript(name),
        escape_applescript(name),
    );
    if dry_run {
        eprintln!("[dry-run] ensure_folder: {script}");
        return Ok(());
    }
    run_osascript(&script)
}

/// Write a note into a Mac Notes folder.
///
/// The body should be HTML (the Notes app renders it).
pub fn write_note(
    title: &str,
    html_body: &str,
    folder: &str,
    dry_run: bool,
) -> Result<(), OutputError> {
    let script = format!(
        r#"tell application "Notes"
    make new note at folder "{}" with properties {{name:"{}", body:"{}"}}
end tell"#,
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
        ensure_notes_folder("Wunicorm", true).unwrap();
    }

    #[test]
    fn write_note_dry_run() {
        write_note("Title", "<div>body</div>", "Wunicorm", true).unwrap();
    }
}
