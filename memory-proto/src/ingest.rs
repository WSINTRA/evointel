//! Notes ingress — pull notes from Mac Notes via osascript.
//!
//! The `AppleScript` iterates all folders (except excluded), emitting
//! each note as a `<<<NOTE>>>…<<<ENDNOTE>>>` record. The body is HTML;
//! we strip it to text for the dreamer. Change detection compares
//! modification timestamps against a `last_seen` map.

use std::collections::BTreeMap;
use std::process::Command;

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};

use crate::nodes::Note;

const NOTE_START: &str = "<<<NOTE>>>";
const NOTE_END: &str = "<<<ENDNOTE>>>";

/// A note as pulled from Mac Notes before any processing.
#[derive(Debug, Clone)]
pub struct RawNote {
    /// Core Data URI — stable across renames.
    pub id: String,
    /// The note's display title.
    pub name: String,
    /// When the note was last modified.
    pub modified: DateTime<Utc>,
    /// Raw HTML body from the Notes app.
    pub body_html: String,
}

/// Errors from the ingress layer.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("osascript failed: {0}")]
    Osascript(String),
    #[error("parse error: {0}")]
    Parse(String),
}

/// Pull all notes from Mac Notes, skipping the given folder names.
///
/// Runs a single `AppleScript` that iterates every folder and every
/// note, returning structured records. The body is HTML from the
/// Notes app.
pub fn pull_notes(excluded_folders: &[String]) -> Result<Vec<RawNote>, IngestError> {
    let script = build_pull_script(excluded_folders);
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| IngestError::Osascript(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(IngestError::Osascript(stderr.to_string()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_notes(&stdout)
}

/// Build the `AppleScript` to pull all notes.
fn build_pull_script(excluded: &[String]) -> String {
    let exclusions: Vec<String> = excluded
        .iter()
        .map(|f| format!("name is not \"{f}\""))
        .collect();
    let where_clause = if exclusions.is_empty() {
        String::new()
    } else {
        format!(" whose {}", exclusions.join(" and "))
    };
    format!(
        r#"tell application "Notes"
    set output to ""
    repeat with f in (every folder{where_clause})
        set theNotes to notes of f
        repeat with i from 1 to (count of theNotes)
            set n to item i of theNotes
            set d to modification date of n
            set output to output & "{NOTE_START}" & linefeed
            set output to output & (id of n) & linefeed
            set output to output & (name of n) & linefeed
            set output to output & (year of d) & " " & (month of d as integer) & " " & (day of d) & " " & (hours of d) & " " & (minutes of d) & " " & (seconds of d) & linefeed
            set output to output & (body of n) & linefeed
            set output to output & "{NOTE_END}" & linefeed
        end repeat
    end repeat
    return output
end tell"#
    )
}

/// Parse the structured osascript output into `RawNote`s.
fn parse_notes(input: &str) -> Result<Vec<RawNote>, IngestError> {
    let mut notes = Vec::new();
    let mut rest = input;
    while let Some(start) = rest.find(NOTE_START) {
        let after_start = &rest[start + NOTE_START.len()..];
        let end = after_start
            .find(NOTE_END)
            .ok_or_else(|| IngestError::Parse("missing <<<ENDNOTE>>>".to_string()))?;
        let record = &after_start[..end];
        let note = parse_record(record)?;
        notes.push(note);
        rest = &after_start[end + NOTE_END.len()..];
    }
    Ok(notes)
}

/// Parse one `<<<NOTE>>>…<<<ENDNOTE>>>` record.
///
/// Format:
/// ```text
/// <id>
/// <name>
/// <year> <month> <day> <hour> <min> <sec>
/// <body_html>
/// ```
fn parse_record(record: &str) -> Result<RawNote, IngestError> {
    // The record starts with a newline (after <<<NOTE>>>); trim it.
    let record = record.trim_start_matches('\n');
    let mut lines = record.lines();
    let id = lines
        .next()
        .ok_or_else(|| IngestError::Parse("missing id".to_string()))?
        .trim()
        .to_string();
    let name = lines
        .next()
        .ok_or_else(|| IngestError::Parse("missing name".to_string()))?
        .trim()
        .to_string();
    let date_line = lines
        .next()
        .ok_or_else(|| IngestError::Parse("missing date".to_string()))?
        .trim()
        .to_string();
    // Body is everything remaining (may contain newlines).
    let body_html = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    let modified = parse_applescript_date(&date_line)?;
    Ok(RawNote {
        id,
        name,
        modified,
        body_html,
    })
}

/// Parse the numeric date format: "2026 11 4 13 25 49".
///
/// `AppleScript` outputs: `year month day hour minute second` — all
/// integers, space-separated. Locale-independent.
fn parse_applescript_date(s: &str) -> Result<DateTime<Utc>, IngestError> {
    let parts: Vec<u32> = s
        .split_whitespace()
        .map(|p| {
            p.parse::<u32>()
                .map_err(|_| IngestError::Parse(format!("bad date component: {p}")))
        })
        .collect::<Result<_, _>>()?;
    if parts.len() != 6 {
        return Err(IngestError::Parse(format!(
            "expected 6 date components, got {}",
            parts.len()
        )));
    }
    let (y, mo, d, h, mi, s) = (
        parts[0].cast_signed(),
        parts[1],
        parts[2],
        parts[3],
        parts[4],
        parts[5],
    );
    let date = NaiveDate::from_ymd_opt(y, mo, d)
        .ok_or_else(|| IngestError::Parse(format!("invalid date: {y}-{mo}-{d}")))?;
    let time = NaiveTime::from_hms_opt(h, mi, s)
        .ok_or_else(|| IngestError::Parse(format!("invalid time: {h}:{mi}:{s}")))?;
    Ok(Utc.from_utc_datetime(&date.and_time(time)))
}

/// Strip HTML tags and decode common entities.
///
/// Block-level elements (div, p, br, li, h1-h6) are converted to
/// newlines before stripping, so text structure is preserved.
pub fn html_to_text(html: &str) -> String {
    // Replace block-level closing/opening tags with newlines.
    let block_re = regex::Regex::new(r"(?i)</(div|p|li|h[1-6])>|<br\s*/?>").unwrap();
    let with_breaks = block_re.replace_all(html, "\n");
    // Strip remaining tags.
    let tag_re = regex::Regex::new(r"<[^>]+>").unwrap();
    let stripped = tag_re.replace_all(&with_breaks, "");
    let mut text = stripped.to_string();
    // Decode entities (&amp; must be last).
    text = text.replace("&lt;", "<");
    text = text.replace("&gt;", ">");
    text = text.replace("&quot;", "\"");
    text = text.replace("&#39;", "'");
    text = text.replace("&apos;", "'");
    text = text.replace("&nbsp;", " ");
    text = text.replace("&amp;", "&");
    // Collapse consecutive newlines into one.
    let blank_re = regex::Regex::new(r"\n{2,}").unwrap();
    text = blank_re.replace_all(&text, "\n").to_string();
    text.trim().to_string()
}

/// Detect which notes are new or changed since the last dream.
///
/// Returns `Vec<Note>` ready for the dreamer queue. Notes whose
/// modification date hasn't changed are skipped.
pub fn detect_changes(raw: &[RawNote], last_seen: &BTreeMap<String, DateTime<Utc>>) -> Vec<Note> {
    let mut queue = Vec::new();
    for r in raw {
        let changed = match last_seen.get(&r.id) {
            None => true,                     // new note
            Some(&prev) => r.modified > prev, // changed
        };
        if changed {
            queue.push(Note {
                id: r.id.clone(),
                content: html_to_text(&r.body_html),
                content_embedding: None,
                created_at: r.modified,
                updated_at: r.modified,
                layer: 0,
                context: "mac-notes".to_string(),
            });
        }
    }
    queue
}

/// Update the `last_seen` map with the current modification dates.
pub fn update_last_seen(map: &mut BTreeMap<String, DateTime<Utc>>, raw: &[RawNote]) {
    for r in raw {
        map.insert(r.id.clone(), r.modified);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    fn sample_osascript_output() -> String {
        format!(
            "{NOTE_START}\n\
             x-coredata://ABC/ICNote/p1\n\
             My First Note\n\
             2026 11 4 13 25 49\n\
             <div>Hello world</div>\n\
             {NOTE_END}\n\
             {NOTE_START}\n\
             x-coredata://ABC/ICNote/p2\n\
             Second Note\n\
             2026 11 5 9 0 0\n\
             <div>Line 1</div>\n\
             <div>Line 2</div>\n\
             {NOTE_END}\n"
        )
    }

    #[test]
    fn parse_notes_basic() {
        let output = sample_osascript_output();
        let notes = parse_notes(&output).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].id, "x-coredata://ABC/ICNote/p1");
        assert_eq!(notes[0].name, "My First Note");
        assert_eq!(notes[1].name, "Second Note");
    }

    #[test]
    fn parse_notes_date_components() {
        let output = sample_osascript_output();
        let notes = parse_notes(&output).unwrap();
        assert_eq!(notes[0].modified.year(), 2026);
        assert_eq!(notes[0].modified.month(), 11);
        assert_eq!(notes[0].modified.day(), 4);
        assert_eq!(notes[0].modified.hour(), 13);
        assert_eq!(notes[0].modified.minute(), 25);
        assert_eq!(notes[0].modified.second(), 49);
    }

    #[test]
    fn parse_notes_body_preserves_content() {
        let output = sample_osascript_output();
        let notes = parse_notes(&output).unwrap();
        assert!(notes[0].body_html.contains("Hello world"));
        assert!(notes[1].body_html.contains("Line 1"));
        assert!(notes[1].body_html.contains("Line 2"));
    }

    #[test]
    fn parse_notes_empty_input() {
        let notes = parse_notes("").unwrap();
        assert!(notes.is_empty());
    }

    #[test]
    fn parse_applescript_date_valid() {
        let dt = parse_applescript_date("2026 11 4 13 25 49").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 11);
        assert_eq!(dt.day(), 4);
        assert_eq!(dt.hour(), 13);
    }

    #[test]
    fn parse_applescript_date_invalid() {
        assert!(parse_applescript_date("bad").is_err());
        assert!(parse_applescript_date("2026 13 1 0 0 0").is_err()); // month 13
        assert!(parse_applescript_date("2026 11 4 25 0 0").is_err()); // hour 25
    }

    #[test]
    fn html_to_text_strips_tags() {
        let html = "<div><h1>Title</h1></div><div>Body text</div>";
        assert_eq!(html_to_text(html), "Title\nBody text");
    }

    #[test]
    fn html_to_text_decodes_entities() {
        assert_eq!(html_to_text("&amp;"), "&");
        assert_eq!(html_to_text("&lt;b&gt;"), "<b>");
        assert_eq!(html_to_text("a&nbsp;b"), "a b");
        assert_eq!(html_to_text("&quot;hi&quot;"), "\"hi\"");
    }

    #[test]
    fn html_to_text_handles_lists() {
        let html = "<ul><li>One</li><li>Two</li></ul>";
        let text = html_to_text(html);
        assert!(text.contains("One"));
        assert!(text.contains("Two"));
    }

    #[test]
    fn detect_changes_new_notes() {
        let raw = vec![RawNote {
            id: "note-1".to_string(),
            name: "Test".to_string(),
            modified: Utc::now(),
            body_html: "<div>test</div>".to_string(),
        }];
        let last_seen = BTreeMap::new();
        let queue = detect_changes(&raw, &last_seen);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].id, "note-1");
        assert_eq!(queue[0].content, "test");
        assert_eq!(queue[0].context, "mac-notes");
    }

    #[test]
    fn detect_changes_unchanged_skipped() {
        let now = Utc::now();
        let raw = vec![RawNote {
            id: "note-1".to_string(),
            name: "Test".to_string(),
            modified: now,
            body_html: "<div>test</div>".to_string(),
        }];
        let mut last_seen = BTreeMap::new();
        last_seen.insert("note-1".to_string(), now);
        let queue = detect_changes(&raw, &last_seen);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn detect_changes_modified_included() {
        let old = Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap();
        let new = Utc.with_ymd_and_hms(2026, 11, 5, 0, 0, 0).unwrap();
        let raw = vec![RawNote {
            id: "note-1".to_string(),
            name: "Test".to_string(),
            modified: new,
            body_html: "<div>updated</div>".to_string(),
        }];
        let mut last_seen = BTreeMap::new();
        last_seen.insert("note-1".to_string(), old);
        let queue = detect_changes(&raw, &last_seen);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].content, "updated");
    }

    #[test]
    fn update_last_seen_inserts_and_overwrites() {
        let now = Utc::now();
        let raw = vec![
            RawNote {
                id: "note-1".to_string(),
                name: "A".to_string(),
                modified: now,
                body_html: String::new(),
            },
            RawNote {
                id: "note-2".to_string(),
                name: "B".to_string(),
                modified: now,
                body_html: String::new(),
            },
        ];
        let mut map = BTreeMap::new();
        update_last_seen(&mut map, &raw);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("note-1"));
        assert!(map.contains_key("note-2"));
    }
}
