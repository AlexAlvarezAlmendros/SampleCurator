//! Comandos de biblioteca: fuentes, escaneo, consulta de la lista y picos de onda.

use super::Estado;
use crate::db::queries;
use crate::domain::*;
use crate::error::{AppError, Result};
use crate::scan;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::State;

#[tauri::command]
pub async fn library_sources(estado: State<'_, Estado>) -> Result<Vec<SourceInfo>> {
    estado.db.read(queries::sources)
}

/// Añade una carpeta y arranca escaneo + análisis en segundo plano.
///
/// Devuelve en cuanto la fuente está registrada: la lista se puede usar mientras entra el
/// resto. El progreso viaja por el canal con throttle, nunca un mensaje por archivo.
#[tauri::command]
pub async fn library_add_source(
    estado: State<'_, Estado>,
    path: String,
    progreso: Channel<ScanProgress>,
) -> Result<SourceInfo> {
    let raiz = PathBuf::from(&path);
    if !raiz.is_dir() {
        return Err(AppError::PathUnavailable(raiz));
    }
    let id = estado.db.read(|c| queries::add_source(c, &path))?;
    lanzar_escaneo(&estado, id, raiz, progreso);
    estado.db.read(|c| {
        queries::sources(c)?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| AppError::NotFound("fuente recién creada".into()))
    })
}

#[tauri::command]
pub async fn library_rescan(
    estado: State<'_, Estado>,
    source_id: i64,
    progreso: Channel<ScanProgress>,
) -> Result<()> {
    let fuentes = estado.db.read(queries::sources)?;
    let fuente = fuentes
        .into_iter()
        .find(|s| s.id == source_id)
        .ok_or_else(|| AppError::NotFound(format!("fuente {source_id}")))?;
    lanzar_escaneo(&estado, source_id, PathBuf::from(fuente.path), progreso);
    Ok(())
}

fn lanzar_escaneo(
    estado: &State<'_, Estado>,
    source_id: i64,
    raiz: PathBuf,
    progreso: Channel<ScanProgress>,
) {
    let db = Arc::clone(&estado.db);
    let cancelar = Arc::clone(&estado.cancelar_analisis);
    cancelar.store(false, Ordering::Relaxed);

    std::thread::Builder::new()
        .name("escaneo".into())
        .spawn(move || {
            let canal = progreso.clone();
            if let Err(e) = scan::escanear(&db, source_id, &raiz, move |p| {
                let _ = canal.send(p);
            }) {
                eprintln!("[scan] error escaneando {}: {e}", raiz.display());
            }
            if let Err(e) = scan::analyzer::analizar_pendientes(&db, &cancelar, move |p| {
                let _ = progreso.send(p);
            }) {
                eprintln!("[scan] error analizando: {e}");
            }
        })
        .ok();
}

#[tauri::command]
pub async fn library_remove_source(estado: State<'_, Estado>, source_id: i64) -> Result<()> {
    estado.db.read(|c| queries::remove_source(c, source_id))
}

#[tauri::command]
pub async fn library_page(estado: State<'_, Estado>, query: LibraryQuery) -> Result<LibraryPage> {
    estado.db.read(|c| queries::page(c, &query))
}

#[tauri::command]
pub async fn library_index_of(
    estado: State<'_, Estado>,
    query: LibraryQuery,
    sample_id: i64,
) -> Result<Option<i64>> {
    estado.db.read(|c| queries::index_of(c, &query, sample_id))
}

#[tauri::command]
pub async fn library_stats(
    estado: State<'_, Estado>,
    source_id: Option<i64>,
) -> Result<LibraryStats> {
    estado.db.read(|c| queries::stats(c, source_id))
}

#[tauri::command]
pub async fn library_detail(estado: State<'_, Estado>, sample_id: i64) -> Result<SampleDetail> {
    estado.db.read(|c| queries::detail(c, sample_id))
}

/// Los picos van como bytes crudos: 2 KB por sample en binario frente a ~12 KB en JSON,
/// y el canvas los pinta sin parsear nada.
#[tauri::command]
pub async fn library_peaks(
    estado: State<'_, Estado>,
    sample_id: i64,
) -> Result<tauri::ipc::Response> {
    let bytes = estado.db.read(|c| queries::peaks(c, sample_id))?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub async fn library_set_rating(
    estado: State<'_, Estado>,
    sample_id: i64,
    rating: i64,
) -> Result<()> {
    estado
        .db
        .read(|c| queries::set_rating(c, sample_id, rating))
}

#[tauri::command]
pub async fn library_analysis_pending(estado: State<'_, Estado>) -> Result<i64> {
    estado.db.read(queries::count_pending_analysis)
}
