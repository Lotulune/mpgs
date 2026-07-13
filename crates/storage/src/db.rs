use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OpenFlags};

use crate::clock::{Clock, SystemClock};
use crate::error::{StorageError, StorageResult};
use crate::migrate::{self, latest_version};

/// SQLite access handle with enforced PRAGMA and single-writer coordination.
#[derive(Clone)]
pub struct Database {
    path: PathBuf,
    write: Arc<Mutex<Connection>>,
    clock: Arc<dyn Clock>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        Self::open_with_clock(path, Arc::new(SystemClock))
    }

    pub fn open_with_clock(path: impl AsRef<Path>, clock: Arc<dyn Clock>) -> StorageResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let conn = open_connection(&path)?;
        apply_pragmas(&conn)?;
        Ok(Self {
            path,
            write: Arc::new(Mutex::new(conn)),
            clock,
        })
    }

    pub fn open_in_memory() -> StorageResult<Self> {
        Self::open_in_memory_with_clock(Arc::new(SystemClock))
    }

    pub fn open_in_memory_with_clock(clock: Arc<dyn Clock>) -> StorageResult<Self> {
        let conn = Connection::open_in_memory()?;
        apply_pragmas(&conn)?;
        Ok(Self {
            path: PathBuf::from(":memory:"),
            write: Arc::new(Mutex::new(conn)),
            clock,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn clock(&self) -> &dyn Clock {
        self.clock.as_ref()
    }

    pub fn now_ms(&self) -> i64 {
        self.clock.now_ms()
    }

    pub fn migrate(&self) -> StorageResult<i64> {
        let mut guard = self
            .write
            .lock()
            .map_err(|_| StorageError::migration("write lock poisoned"))?;
        apply_pragmas(&guard)?;
        migrate::migrate_to_latest(&mut guard, self.now_ms())
    }

    pub fn schema_version(&self) -> StorageResult<i64> {
        let guard = self
            .write
            .lock()
            .map_err(|_| StorageError::migration("write lock poisoned"))?;
        migrate::current_version(&guard)
    }

    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> StorageResult<T>,
    ) -> StorageResult<T> {
        let guard = self
            .write
            .lock()
            .map_err(|_| StorageError::migration("write lock poisoned"))?;
        apply_pragmas(&guard)?;
        f(&guard)
    }

    pub fn with_conn_mut<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> StorageResult<T>,
    ) -> StorageResult<T> {
        let mut guard = self
            .write
            .lock()
            .map_err(|_| StorageError::migration("write lock poisoned"))?;
        apply_pragmas(&guard)?;
        f(&mut guard)
    }

    pub fn integrity_check(&self) -> StorageResult<Vec<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("PRAGMA integrity_check")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    pub fn assert_ready(&self) -> StorageResult<()> {
        let version = self.schema_version()?;
        if version != latest_version() {
            return Err(StorageError::migration(format!(
                "schema version {version} != expected {}",
                latest_version()
            )));
        }
        let check = self.integrity_check()?;
        if check != ["ok".to_owned()] {
            return Err(StorageError::migration(format!(
                "integrity_check failed: {check:?}"
            )));
        }
        Ok(())
    }
}

fn open_connection(path: &Path) -> StorageResult<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    Ok(conn)
}

pub fn apply_pragmas(conn: &Connection) -> StorageResult<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 5000i32)?;
    conn.pragma_update(None, "trusted_schema", "OFF")?;
    // journal_mode returns the mode string; verify WAL when on a real file.
    let mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    if mode.eq_ignore_ascii_case("wal") || mode.eq_ignore_ascii_case("memory") {
        // memory databases may not use WAL; accept both.
    } else {
        return Err(StorageError::migration(format!(
            "expected journal_mode WAL or memory, got {mode}"
        )));
    }
    conn.pragma_update(None, "synchronous", "FULL")?;
    Ok(())
}
