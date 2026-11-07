use std::path::{Path, PathBuf};

use lbug::{Connection, Database, SystemConfig};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("lbug error: {0}")]
    Lbug(#[from] lbug::Error),
}

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug)]
pub struct MemoryDb {
    pub db: Database,
    pub db_path: PathBuf,
    pub embedding_dim: usize,
}

impl MemoryDb {
    pub fn open(path: &Path, embedding_dim: usize) -> DbResult<Self> {
        let db = Database::new(path, SystemConfig::default())?;
        Ok(Self {
            db_path: path.to_path_buf(),
            embedding_dim,
            db,
        })
    }

    pub fn connect(&self) -> DbResult<Connection<'_>> {
        Ok(Connection::new(&self.db)?)
    }
}
