//! SQLite persistence for the calculator: calculation history and settings.
//! Schema changes are applied by `rusqlite_migration`, tracked in `PRAGMA user_version`.

use std::fmt;
use std::path::Path;

use rusqlite::{Connection, params};
use rusqlite_migration::{M, Migrations};

use crate::viewmodel::{HISTORY_LIMIT, HistoryEntry};

#[derive(Debug)]
pub enum StorageError {
    Sql(rusqlite::Error),
    Io(std::io::Error),
    Migration(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sql(e) => write!(f, "erro no banco de dados: {e}"),
            Self::Io(e) => write!(f, "erro de arquivo: {e}"),
            Self::Migration(msg) => write!(f, "falha ao atualizar o banco: {msg}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("../migrations/0001_history.sql")),
        M::up(include_str!("../migrations/0002_settings.sql")),
    ])
}

pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Opens (or creates) the database, creating its parent directory if needed.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::from_connection(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self, StorageError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut conn: Connection) -> Result<Self, StorageError> {
        migrations()
            .to_latest(&mut conn)
            .map_err(|e| StorageError::Migration(e.to_string()))?;
        Ok(Self { conn })
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        match self.conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0)) {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Most recent first, at most `HISTORY_LIMIT` entries.
    pub fn load_history(&self) -> Result<Vec<HistoryEntry>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT expression, result FROM history ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map([HISTORY_LIMIT as i64], |row| {
            Ok(HistoryEntry { expression: row.get(0)?, result: row.get(1)? })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Appends one calculation and drops the oldest entries beyond the limit.
    pub fn push_history(&self, entry: &HistoryEntry) -> Result<(), StorageError> {
        self.conn.execute(
            "INSERT INTO history (expression, result) VALUES (?1, ?2)",
            params![entry.expression, entry.result],
        )?;
        self.conn.execute(
            "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT ?1)",
            [HISTORY_LIMIT as i64],
        )?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<(), StorageError> {
        self.conn.execute("DELETE FROM history", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(i: usize) -> HistoryEntry {
        HistoryEntry { expression: format!("{i}+1"), result: format!("{}", i + 1) }
    }

    #[test]
    fn migrations_are_valid() {
        assert!(migrations().validate().is_ok());
    }

    #[test]
    fn history_round_trip_keeps_newest_first() {
        let storage = Storage::in_memory().unwrap();
        storage.push_history(&entry(1)).unwrap();
        storage.push_history(&entry(2)).unwrap();
        let loaded = storage.load_history().unwrap();
        assert_eq!(loaded, vec![entry(2), entry(1)]);
    }

    #[test]
    fn history_is_capped_at_the_limit() {
        let storage = Storage::in_memory().unwrap();
        for i in 0..(HISTORY_LIMIT + 7) {
            storage.push_history(&entry(i)).unwrap();
        }
        let loaded = storage.load_history().unwrap();
        assert_eq!(loaded.len(), HISTORY_LIMIT);
        assert_eq!(loaded[0], entry(HISTORY_LIMIT + 6));
        assert_eq!(loaded[HISTORY_LIMIT - 1], entry(7));
    }

    #[test]
    fn clear_empties_history() {
        let storage = Storage::in_memory().unwrap();
        storage.push_history(&entry(1)).unwrap();
        storage.clear_history().unwrap();
        assert!(storage.load_history().unwrap().is_empty());
    }

    #[test]
    fn settings_round_trip() {
        let storage = Storage::in_memory().unwrap();
        assert_eq!(storage.get_setting("theme").unwrap(), None);
        storage.set_setting("theme", "a").unwrap();
        storage.set_setting("theme", "b").unwrap();
        assert_eq!(storage.get_setting("theme").unwrap().as_deref(), Some("b"));
    }
}
