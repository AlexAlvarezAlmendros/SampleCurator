//! Información de la app y preferencias sueltas.

use super::Estado;
use crate::domain::AppInfo;
use crate::error::Result;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub async fn app_info(estado: State<'_, Estado>) -> Result<AppInfo> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_path: estado.db.path().to_string_lossy().to_string(),
        audio: estado.audio.as_ref().map(|a| a.info()),
        audio_error: estado.audio_error.clone(),
    })
}

#[tauri::command]
pub async fn settings_get(estado: State<'_, Estado>, key: String) -> Result<Option<String>> {
    estado.db.read(|c| {
        let mut st = c.prepare_cached("SELECT value FROM settings WHERE key = ?1")?;
        let v: Option<String> =
            st.query_row(params![key], |r| r.get(0))
                .map(Some)
                .or_else(|e| match e {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    otro => Err(otro),
                })?;
        Ok(v)
    })
}

#[tauri::command]
pub async fn settings_set(estado: State<'_, Estado>, key: String, value: String) -> Result<()> {
    estado.db.read(|c| {
        c.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    })
}
