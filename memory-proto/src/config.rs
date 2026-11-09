//! Configuration — loaded from `~/.config/memory-proto/config.toml`.
//!
//! Created with defaults on first run. Environment variables override
//! LLM URLs (`WUNICORM_CHAT_URL`, `WUNICORM_CHAT_MODEL`,
//! `WUNICORM_EMBED_URL`, `WUNICORM_EMBED_MODEL`).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The daemon's configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Local time of day to trigger the dream, HH:MM.
    pub dream_time: String,
    /// Where the memory graph database lives.
    pub db_path: String,
    /// Where the small JSON state file lives.
    pub state_path: String,
    /// Mac Notes folders to skip when pulling notes.
    pub excluded_folders: Vec<String>,
    /// The Mac Notes account that receives the sermons. `None` resolves
    /// at runtime to the first account that holds folders.
    #[serde(default)]
    pub notes_account: Option<String>,
    /// LLM configuration. All fields are optional; env vars override.
    #[serde(default)]
    pub llm: LlmSection,
}

/// Optional LLM overrides in the config file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmSection {
    pub chat_base_url: Option<String>,
    pub chat_model: Option<String>,
    pub embed_base_url: Option<String>,
    pub embed_model: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dream_time: "22:00".to_string(),
            db_path: Self::default_db_path().to_string_lossy().to_string(),
            state_path: Self::default_state_path().to_string_lossy().to_string(),
            excluded_folders: vec!["Wunicorm".to_string(), "Recently Deleted".to_string()],
            notes_account: None,
            llm: LlmSection::default(),
        }
    }
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("invalid dream_time '{0}': expected HH:MM")]
    InvalidDreamTime(String),
}

impl Config {
    /// Load configuration from the default path. Creates the file
    /// with defaults if it doesn't exist.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::default_config_path();
        Self::load_from(&path)
    }

    /// Load configuration from a specific path.
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            config.validate()?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save_to(path)?;
            Ok(config)
        }
    }

    /// Save the configuration to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Parse `dream_time` into (hour, minute).
    pub fn dream_hour_minute(&self) -> Result<(u8, u8), ConfigError> {
        parse_hhmm(&self.dream_time)
    }

    /// Expand `~` and return the DB path as a `PathBuf`.
    pub fn resolved_db_path(&self) -> PathBuf {
        expand_tilde(&self.db_path)
    }

    /// Expand `~` and return the state file path as a `PathBuf`.
    pub fn resolved_state_path(&self) -> PathBuf {
        expand_tilde(&self.state_path)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        self.dream_hour_minute()?;
        Ok(())
    }

    fn default_config_path() -> PathBuf {
        config_dir().join("config.toml")
    }

    fn default_db_path() -> PathBuf {
        data_dir().join("memory.db")
    }

    fn default_state_path() -> PathBuf {
        data_dir().join("state.json")
    }
}

/// The XDG config directory for memory-proto.
fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("memory-proto")
}

/// The XDG data directory for memory-proto.
fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("memory-proto")
}

/// Expand a leading `~` to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

/// Parse "HH:MM" into (hour, minute). Rejects invalid values.
fn parse_hhmm(s: &str) -> Result<(u8, u8), ConfigError> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(ConfigError::InvalidDreamTime(s.to_string()));
    }
    let hour: u8 = parts[0]
        .parse()
        .map_err(|_| ConfigError::InvalidDreamTime(s.to_string()))?;
    let minute: u8 = parts[1]
        .parse()
        .map_err(|_| ConfigError::InvalidDreamTime(s.to_string()))?;
    if hour > 23 || minute > 59 {
        return Err(ConfigError::InvalidDreamTime(s.to_string()));
    }
    Ok((hour, minute))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_config_has_22_00() {
        let c = Config::default();
        assert_eq!(c.dream_time, "22:00");
        assert_eq!(c.excluded_folders, vec!["Wunicorm", "Recently Deleted"]);
    }

    #[test]
    fn parse_hhmm_valid() {
        assert_eq!(parse_hhmm("22:00").unwrap(), (22, 0));
        assert_eq!(parse_hhmm("03:30").unwrap(), (3, 30));
        assert_eq!(parse_hhmm("00:00").unwrap(), (0, 0));
        assert_eq!(parse_hhmm("23:59").unwrap(), (23, 59));
    }

    #[test]
    fn parse_hhmm_rejects_invalid() {
        assert!(parse_hhmm("24:00").is_err());
        assert!(parse_hhmm("22:60").is_err());
        assert!(parse_hhmm("bad").is_err());
        assert!(parse_hhmm("22").is_err());
        assert!(parse_hhmm("22:00:00").is_err());
    }

    #[test]
    fn load_creates_default_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        assert!(!path.exists());
        let c = Config::load_from(&path).unwrap();
        assert!(path.exists());
        assert_eq!(c.dream_time, "22:00");
    }

    #[test]
    fn load_reads_existing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
dream_time = "03:00"
db_path = "/tmp/test.db"
state_path = "/tmp/test-state.json"
excluded_folders = ["Wunicorm"]
"#,
        )
        .unwrap();
        let c = Config::load_from(&path).unwrap();
        assert_eq!(c.dream_time, "03:00");
        assert_eq!(c.db_path, "/tmp/test.db");
        assert_eq!(c.excluded_folders, vec!["Wunicorm"]);
        assert_eq!(c.notes_account, None);
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        let c = Config::default();
        c.save_to(&path).unwrap();
        let c2 = Config::load_from(&path).unwrap();
        assert_eq!(c.dream_time, c2.dream_time);
        assert_eq!(c.db_path, c2.db_path);
    }

    #[test]
    fn expand_tilde_works() {
        let expanded = expand_tilde("~/foo/bar");
        assert!(!expanded.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn expand_tilde_absolute_unchanged() {
        let p = expand_tilde("/absolute/path");
        assert_eq!(p, PathBuf::from("/absolute/path"));
    }
}
