use std::path::Path;

use rusqlite::{Connection, OpenFlags, backup::Backup};

use crate::db::{Database, apply_pragmas};
use crate::error::{StorageError, StorageResult};
use crate::migrate;

/// Online backup of the active database into `dest_path`.
pub fn backup_to_path(db: &Database, dest_path: impl AsRef<Path>) -> StorageResult<()> {
    let dest_path = dest_path.as_ref();
    if let Some(parent) = dest_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    if dest_path.exists() {
        std::fs::remove_file(dest_path)?;
    }

    db.with_conn(|src| {
        let mut dst = Connection::open(dest_path)?;
        {
            let backup = Backup::new(src, &mut dst)?;
            backup
                .run_to_completion(100, std::time::Duration::from_millis(5), None)
                .map_err(|e| StorageError::migration(format!("backup failed: {e}")))?;
        }
        apply_pragmas(&dst)?;
        Ok(())
    })
}

/// Restore a backup file into a new destination path and verify integrity/migrations.
pub fn restore_from_backup(
    backup_path: impl AsRef<Path>,
    dest_path: impl AsRef<Path>,
    now_ms: i64,
) -> StorageResult<Database> {
    let backup_path = backup_path.as_ref();
    let dest_path = dest_path.as_ref();
    if !backup_path.exists() {
        return Err(StorageError::not_found(format!(
            "backup {}",
            backup_path.display()
        )));
    }
    if let Some(parent) = dest_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    if dest_path.exists() {
        std::fs::remove_file(dest_path)?;
    }
    std::fs::copy(backup_path, dest_path)?;

    let db = Database::open(dest_path)?;
    // Ensure restored DB can still accept forward migrations.
    db.with_conn_mut(|conn| {
        apply_pragmas(conn)?;
        migrate::migrate_to_latest(conn, now_ms)?;
        Ok(())
    })?;
    db.assert_ready()?;
    Ok(db)
}

/// Open a read-only connection against a file for verification helpers.
pub fn open_readonly(path: impl AsRef<Path>) -> StorageResult<Connection> {
    let conn = Connection::open_with_flags(
        path.as_ref(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    Ok(conn)
}
