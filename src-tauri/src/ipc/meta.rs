//! Comandos de metadatos: etiquetas y notas.
//!
//! Todo esto vive en el índice. Los archivos de audio del usuario no se tocan nunca: escribir
//! etiquetas dentro del `.wav` significaría reescribir sus archivos, y los DAW mayormente las
//! ignoran en samples.

use super::Estado;
use crate::db::tags;
use crate::error::{AppError, Result};
use tauri::State;

const MAX_ETIQUETA: usize = 40;
const MAX_NOTAS: usize = 2000;

#[tauri::command]
pub async fn tags_of(estado: State<'_, Estado>, sample_id: i64) -> Result<Vec<String>> {
    estado.db.read(|c| tags::de_sample(c, sample_id))
}

#[tauri::command]
pub async fn tags_add(estado: State<'_, Estado>, sample_id: i64, name: String) -> Result<()> {
    let limpio = tags::normalizar(&name);
    if limpio.is_empty() {
        return Err(AppError::InvalidInput("la etiqueta está vacía".into()));
    }
    if limpio.chars().count() > MAX_ETIQUETA {
        return Err(AppError::InvalidInput(format!(
            "la etiqueta no puede pasar de {MAX_ETIQUETA} caracteres"
        )));
    }
    estado.db.read(|c| tags::poner(c, sample_id, &limpio))
}

#[tauri::command]
pub async fn tags_remove(estado: State<'_, Estado>, sample_id: i64, name: String) -> Result<()> {
    estado.db.read(|c| tags::quitar(c, sample_id, &name))
}

/// Catálogo de etiquetas con cuántos samples las llevan: alimenta el autocompletado y el filtro.
#[tauri::command]
pub async fn tags_all(estado: State<'_, Estado>) -> Result<Vec<(String, i64)>> {
    estado.db.read(tags::todas)
}

#[tauri::command]
pub async fn notes_set(estado: State<'_, Estado>, sample_id: i64, text: String) -> Result<()> {
    if text.chars().count() > MAX_NOTAS {
        return Err(AppError::InvalidInput(format!(
            "las notas no pueden pasar de {MAX_NOTAS} caracteres"
        )));
    }
    estado.db.read(|c| tags::set_notas(c, sample_id, &text))
}
