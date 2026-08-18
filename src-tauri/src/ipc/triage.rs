//! Comandos de triaje: proyectos, destinos, enviar/rechazar/conservar, deshacer y papelera.

use super::Estado;
use crate::db::triage as q;
use crate::domain::*;
use crate::error::{AppError, Result};
use crate::fileops;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn triage_projects(estado: State<'_, Estado>) -> Result<Vec<Project>> {
    estado.db.read(q::projects)
}

#[tauri::command]
pub async fn triage_last_project(estado: State<'_, Estado>) -> Result<Option<Project>> {
    estado.db.read(q::last_project)
}

#[tauri::command]
pub async fn triage_create_project(
    estado: State<'_, Estado>,
    name: String,
    dest_root: String,
    mode: TriageMode,
) -> Result<Project> {
    let raiz = PathBuf::from(&dest_root);
    if !raiz.is_dir() {
        return Err(AppError::PathUnavailable(raiz));
    }
    estado
        .db
        .read(|c| q::create_project(c, &name, &dest_root, mode))
}

#[tauri::command]
pub async fn triage_open_project(estado: State<'_, Estado>, project_id: i64) -> Result<Project> {
    estado.db.read(|c| {
        q::touch_project(c, project_id, None)?;
        q::project(c, project_id)
    })
}

#[tauri::command]
pub async fn triage_delete_project(estado: State<'_, Estado>, project_id: i64) -> Result<()> {
    estado.db.read(|c| q::delete_project(c, project_id))
}

#[tauri::command]
pub async fn triage_set_mode(
    estado: State<'_, Estado>,
    project_id: i64,
    mode: TriageMode,
) -> Result<()> {
    estado.db.read(|c| q::set_project_mode(c, project_id, mode))
}

#[tauri::command]
pub async fn triage_destinations(
    estado: State<'_, Estado>,
    project_id: i64,
) -> Result<Vec<Destination>> {
    estado.db.read(|c| q::destinations(c, project_id))
}

#[tauri::command]
pub async fn triage_create_destination(
    estado: State<'_, Estado>,
    project_id: i64,
    name: String,
    rel_path: String,
) -> Result<Destination> {
    let nombre = name.trim();
    if nombre.is_empty() {
        return Err(AppError::InvalidInput(
            "el destino necesita un nombre".into(),
        ));
    }
    let rel = if rel_path.trim().is_empty() {
        nombre.to_string()
    } else {
        rel_path.trim().to_string()
    };
    // Un destino nunca puede salirse de la carpeta raíz del proyecto.
    if rel.contains("..") || rel.starts_with('/') {
        return Err(AppError::InvalidInput(
            "la subcarpeta del destino debe quedar dentro de la carpeta de destino".into(),
        ));
    }
    estado
        .db
        .read(|c| q::create_destination(c, project_id, nombre, &rel))
}

#[tauri::command]
pub async fn triage_rename_destination(
    estado: State<'_, Estado>,
    dest_id: i64,
    name: String,
    rel_path: String,
) -> Result<()> {
    estado
        .db
        .read(|c| q::rename_destination(c, dest_id, &name, &rel_path))
}

#[tauri::command]
pub async fn triage_delete_destination(estado: State<'_, Estado>, dest_id: i64) -> Result<()> {
    estado.db.read(|c| q::delete_destination(c, dest_id))
}

/// Propone como destinos las subcarpetas que ya existen en la carpeta de destino: casi siempre
/// el usuario ya tiene su estructura montada y no quiere volver a escribirla.
#[tauri::command]
pub async fn triage_suggest_destinations(
    estado: State<'_, Estado>,
    project_id: i64,
) -> Result<Vec<String>> {
    let proyecto = estado.db.read(|c| q::project(c, project_id))?;
    let mut nombres = Vec::new();
    if let Ok(entradas) = std::fs::read_dir(&proyecto.dest_root) {
        for e in entradas.flatten() {
            let nombre = e.file_name().to_string_lossy().to_string();
            if nombre.starts_with('.') {
                continue;
            }
            if e.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                nombres.push(nombre);
            }
        }
    }
    nombres.sort_by_key(|n| n.to_lowercase());
    Ok(nombres)
}

// ─────────────────────────── decisiones ───────────────────────────

#[tauri::command]
pub async fn triage_send(
    estado: State<'_, Estado>,
    project_id: i64,
    dest_id: i64,
    sample_ids: Vec<i64>,
) -> Result<TriageResult> {
    let r = fileops::enviar(&estado.db, project_id, dest_id, &sample_ids)?;
    olvidar_movidos(&estado, &r.affected);
    Ok(r)
}

#[tauri::command]
pub async fn triage_reject(
    estado: State<'_, Estado>,
    project_id: i64,
    sample_ids: Vec<i64>,
) -> Result<TriageResult> {
    let r = fileops::rechazar(&estado.db, project_id, &sample_ids)?;
    olvidar_movidos(&estado, &r.affected);
    Ok(r)
}

#[tauri::command]
pub async fn triage_keep(
    estado: State<'_, Estado>,
    project_id: i64,
    sample_ids: Vec<i64>,
) -> Result<TriageResult> {
    fileops::conservar(&estado.db, project_id, &sample_ids)
}

#[tauri::command]
pub async fn triage_undo(estado: State<'_, Estado>) -> Result<UndoResult> {
    let r = fileops::deshacer(&estado.db)?;
    olvidar_movidos(&estado, &r.restored);
    Ok(r)
}

#[tauri::command]
pub async fn triage_redo(estado: State<'_, Estado>) -> Result<UndoResult> {
    let r = fileops::rehacer(&estado.db)?;
    olvidar_movidos(&estado, &r.restored);
    Ok(r)
}

/// Al mover un archivo, el buffer que hay en la caché apunta a una ruta que ya no existe.
fn olvidar_movidos(estado: &State<'_, Estado>, ids: &[i64]) {
    if let Some(audio) = estado.audio.as_ref() {
        audio.olvidar(ids.to_vec());
    }
}

#[tauri::command]
pub async fn triage_progress(
    estado: State<'_, Estado>,
    source_id: Option<i64>,
) -> Result<SessionProgress> {
    let (done, total) = estado.db.read(|c| q::session_progress(c, source_id))?;
    Ok(SessionProgress { done, total })
}

#[tauri::command]
pub async fn triage_remember(
    estado: State<'_, Estado>,
    project_id: i64,
    sample_id: i64,
) -> Result<()> {
    estado
        .db
        .read(|c| q::touch_project(c, project_id, Some(sample_id)))
}

#[tauri::command]
pub async fn triage_last_sample(estado: State<'_, Estado>, project_id: i64) -> Result<Option<i64>> {
    estado.db.read(|c| q::last_sample_of(c, project_id))
}

#[tauri::command]
pub async fn triage_rename(
    estado: State<'_, Estado>,
    sample_id: i64,
    name: String,
    project_id: Option<i64>,
) -> Result<String> {
    let nuevo = fileops::renombrar(&estado.db, project_id, sample_id, &name)?;
    olvidar_movidos(&estado, &[sample_id]);
    Ok(nuevo)
}

/// Vuelca las decisiones a `<destino>/library.json`. El índice se puede reconstruir; las
/// decisiones del usuario, no.
#[tauri::command]
pub async fn triage_export(estado: State<'_, Estado>, project_id: i64) -> Result<String> {
    let ruta = fileops::export::exportar(&estado.db, project_id)?;
    Ok(ruta.to_string_lossy().to_string())
}

// ─────────────────────────── papelera ───────────────────────────

#[tauri::command]
pub async fn triage_trash_summary(
    estado: State<'_, Estado>,
    project_id: i64,
) -> Result<TrashSummary> {
    let proyecto = estado.db.read(|c| q::project(c, project_id))?;
    let r = fileops::trash::resumen(&PathBuf::from(&proyecto.dest_root));
    Ok(TrashSummary {
        files: r.archivos,
        bytes: r.bytes,
    })
}

/// Lo que hay en la papelera, con lo que hace falta para decidir sin salir de la app.
#[tauri::command]
pub async fn trash_list(estado: State<'_, Estado>, project_id: i64) -> Result<Vec<TrashEntry>> {
    fileops::papelera(&estado.db, project_id)
}

/// Devuelve un archivo de la papelera a su carpeta original y lo pone otra vez en la cola.
#[tauri::command]
pub async fn trash_restore(
    estado: State<'_, Estado>,
    project_id: i64,
    trash_path: String,
) -> Result<i64> {
    let id = fileops::restaurar(&estado.db, project_id, &trash_path)?;
    if id > 0 {
        olvidar_movidos(&estado, &[id]);
    }
    Ok(id)
}

/// La única operación irreversible de la app. El frontend la confirma con un diálogo, y es el
/// único diálogo de confirmación que existe.
#[tauri::command]
pub async fn triage_empty_trash(estado: State<'_, Estado>, project_id: i64) -> Result<i64> {
    let proyecto = estado.db.read(|c| q::project(c, project_id))?;
    fileops::trash::vaciar(&PathBuf::from(&proyecto.dest_root))
}
