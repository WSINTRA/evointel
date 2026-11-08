//! Scheduler + state persistence — when to dream, what we've seen.
//!
//! `DaemonState` is a small JSON file tracking which notes have been
//! ingested and when the last dream ran. `Scheduler` answers "is it
//! dream time?" with a once-per-day guard.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Local, NaiveDate, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::config::Config;

/// The daemon's persistent state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DaemonState {
    /// Note ID → last-seen modification date.
    pub last_seen: BTreeMap<String, DateTime<Utc>>,
    /// The date of the last completed dream (once-per-day guard).
    pub last_dream_date: Option<NaiveDate>,
}

/// State persistence errors.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl DaemonState {
    /// Load state from a JSON file. Returns defaults if the file
    /// doesn't exist.
    pub fn load(path: &Path) -> Result<Self, StateError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let state: DaemonState = serde_json::from_str(&content)?;
            Ok(state)
        } else {
            Ok(Self::default())
        }
    }

    /// Save state to a JSON file atomically (temp file + rename).
    pub fn save(&self, path: &Path) -> Result<(), StateError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, content)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

/// Decides when to trigger the dreamer.
#[derive(Debug, Clone)]
pub struct Scheduler {
    dream_hour: u8,
    dream_minute: u8,
}

impl Scheduler {
    /// Build from the parsed config.
    pub fn from_config(config: &Config) -> Self {
        let (hour, minute) = config.dream_hour_minute().unwrap_or((22, 0));
        Self {
            dream_hour: hour,
            dream_minute: minute,
        }
    }

    /// Build with explicit hour and minute.
    pub fn new(hour: u8, minute: u8) -> Self {
        Self {
            dream_hour: hour,
            dream_minute: minute,
        }
    }

    /// Is it time to dream?
    ///
    /// True when the current time is at or past the configured dream
    /// time AND we haven't already dreamed today.
    pub fn is_dream_time(&self, now: DateTime<Local>, last_dream_date: Option<NaiveDate>) -> bool {
        let today = now.date_naive();
        // Already dreamed today?
        if last_dream_date == Some(today) {
            return false;
        }
        // Past the configured time? (hour/minute are always 0-23 / 0-59)
        #[allow(clippy::cast_possible_truncation)]
        let (h, m) = (now.hour() as u8, now.minute() as u8);
        (h, m) >= (self.dream_hour, self.dream_minute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, mo, d, h, mi, 0).single().unwrap()
    }

    #[test]
    fn state_default_is_empty() {
        let s = DaemonState::default();
        assert!(s.last_seen.is_empty());
        assert!(s.last_dream_date.is_none());
    }

    #[test]
    fn state_save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("state.json");
        let mut state = DaemonState::default();
        state.last_seen.insert("note-1".to_string(), Utc::now());
        state.last_dream_date = Some(NaiveDate::from_ymd_opt(2026, 11, 8).unwrap());
        state.save(&path).unwrap();
        let loaded = DaemonState::load(&path).unwrap();
        assert_eq!(loaded.last_seen.len(), 1);
        assert!(loaded.last_seen.contains_key("note-1"));
        assert_eq!(
            loaded.last_dream_date,
            Some(NaiveDate::from_ymd_opt(2026, 11, 8).unwrap())
        );
    }

    #[test]
    fn state_load_missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.json");
        let state = DaemonState::load(&path).unwrap();
        assert!(state.last_seen.is_empty());
        assert!(state.last_dream_date.is_none());
    }

    #[test]
    fn state_save_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("sub").join("dir").join("state.json");
        let state = DaemonState::default();
        state.save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn scheduler_fires_at_exact_time() {
        let s = Scheduler::new(22, 0);
        let now = at(2026, 11, 8, 22, 0);
        assert!(s.is_dream_time(now, None));
    }

    #[test]
    fn scheduler_fires_after_time() {
        let s = Scheduler::new(22, 0);
        let now = at(2026, 11, 8, 23, 30);
        assert!(s.is_dream_time(now, None));
    }

    #[test]
    fn scheduler_does_not_fire_before_time() {
        let s = Scheduler::new(22, 0);
        let now = at(2026, 11, 8, 21, 59);
        assert!(!s.is_dream_time(now, None));
    }

    #[test]
    fn scheduler_does_not_fire_morning() {
        let s = Scheduler::new(22, 0);
        let now = at(2026, 11, 8, 8, 0);
        assert!(!s.is_dream_time(now, None));
    }

    #[test]
    fn scheduler_once_per_day_guard() {
        let s = Scheduler::new(22, 0);
        let now = at(2026, 11, 8, 22, 0);
        let today = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
        assert!(!s.is_dream_time(now, Some(today)));
    }

    #[test]
    fn scheduler_fires_next_day() {
        let s = Scheduler::new(22, 0);
        let now = at(2026, 11, 9, 22, 0);
        let yesterday = NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
        assert!(s.is_dream_time(now, Some(yesterday)));
    }

    #[test]
    fn scheduler_from_config() {
        let c = Config::default();
        let s = Scheduler::from_config(&c);
        // Default is 22:00.
        let now = at(2026, 11, 8, 22, 0);
        assert!(s.is_dream_time(now, None));
    }
}
