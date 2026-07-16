#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use rusqlite::{params, Connection};
use tauri::{Manager, State};

struct ClientStore(Mutex<Connection>);

impl ClientStore {
    fn open(app: &tauri::App) -> Result<Self, String> {
        let data_dir = client_data_dir(app)?;
        fs::create_dir_all(&data_dir).map_err(|error| error.to_string())?;
        Self::open_at(&data_dir.join("client-state.sqlite3"))
    }

    fn open_at(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 CREATE TABLE IF NOT EXISTS client_kv (
                     key TEXT PRIMARY KEY NOT NULL,
                     value TEXT NOT NULL,
                     updated_at_ms INTEGER NOT NULL
                 );",
            )
            .map_err(|error| error.to_string())?;
        Ok(Self(Mutex::new(connection)))
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.0
            .lock()
            .map_err(|_| "client storage lock is poisoned".to_owned())
    }

    fn load(&self) -> Result<HashMap<String, String>, String> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare("SELECT key, value FROM client_kv ORDER BY key")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<HashMap<_, _>, _>>()
            .map_err(|error| error.to_string())
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        validate_client_key(key)?;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_millis() as i64;
        self.lock()?
            .execute(
                "INSERT INTO client_kv (key, value, updated_at_ms) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at_ms = excluded.updated_at_ms",
                params![key, value, now_ms],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn remove(&self, key: &str) -> Result<(), String> {
        validate_client_key(key)?;
        self.lock()?
            .execute("DELETE FROM client_kv WHERE key = ?1", [key])
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

fn client_data_dir(app: &tauri::App) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("MPGS_CLIENT_DATA_DIR") {
        if path.is_empty() {
            return Err("MPGS_CLIENT_DATA_DIR must not be empty".to_owned());
        }
        return Ok(PathBuf::from(path));
    }
    app.path().app_data_dir().map_err(|error| error.to_string())
}

fn validate_client_key(key: &str) -> Result<(), String> {
    if key.starts_with("mpgs.") {
        Ok(())
    } else {
        Err("client storage key must use the mpgs namespace".to_owned())
    }
}

#[tauri::command]
fn client_store_load(store: State<'_, ClientStore>) -> Result<HashMap<String, String>, String> {
    store.load()
}

#[tauri::command]
fn client_store_set(
    key: String,
    value: String,
    store: State<'_, ClientStore>,
) -> Result<(), String> {
    store.set(&key, &value)
}

#[tauri::command]
fn client_store_remove(key: String, store: State<'_, ClientStore>) -> Result<(), String> {
    store.remove(&key)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(ClientStore::open(app)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            client_store_load,
            client_store_set,
            client_store_remove
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_sqlite_persists_across_reopen_and_enforces_namespace() {
        let unique = format!(
            "mpgs-client-store-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let directory = std::env::temp_dir().join(unique);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("client-state.sqlite3");

        {
            let store = ClientStore::open_at(&path).unwrap();
            assert!(store.set("other.key", "rejected").is_err());
            store.set("mpgs.cache.feed", "snapshot").unwrap();
            assert_eq!(
                store.load().unwrap().get("mpgs.cache.feed").map(String::as_str),
                Some("snapshot")
            );
        }
        {
            let reopened = ClientStore::open_at(&path).unwrap();
            assert_eq!(
                reopened
                    .load()
                    .unwrap()
                    .get("mpgs.cache.feed")
                    .map(String::as_str),
                Some("snapshot")
            );
            reopened.remove("mpgs.cache.feed").unwrap();
            assert!(reopened.load().unwrap().is_empty());
        }

        fs::remove_dir_all(directory).unwrap();
    }
}
