//! SampleCurator — triaje rápido de librerías de samples.
//!
//! Reparto de responsabilidades (ver docs/ARCHITECTURE.md):
//!   `domain`  tipos puros, sin dependencias · fuente del contrato con el frontend
//!   `codec`   decodificar, remuestrear y describir audio
//!   `db`      SQLite: el índice, que es una caché reconstruible
//!   `scan`    recorrer el disco y analizar en segundo plano
//!   `audio`   motor de tiempo real
//!   `fileops` mover, rechazar y deshacer sin perder nada
//!   `ipc`     capa fina hacia el WebView

pub mod audio;
pub mod codec;
pub mod db;
pub mod domain;
pub mod error;
pub mod fileops;
pub mod ipc;
pub mod music;
pub mod paths;
pub mod scan;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // Al cerrar bien, las decisiones se vuelcan a <destino>/library.json. El índice se
        // puede reconstruir escaneando; lo que el usuario decidió, no.
        .on_window_event(|ventana, evento| {
            if !matches!(evento, WindowEvent::CloseRequested { .. }) {
                return;
            }
            let estado = ventana.state::<ipc::Estado>();
            estado
                .cancelar_analisis
                .store(true, std::sync::atomic::Ordering::Relaxed);

            if let Ok(Some(proyecto)) = estado.db.read(db::triage::last_project) {
                match fileops::export::exportar(&estado.db, proyecto.id) {
                    Ok(ruta) => eprintln!("[cierre] decisiones guardadas en {}", ruta.display()),
                    Err(e) => eprintln!("[cierre] no se pudo guardar library.json: {e}"),
                }
            }
        })
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .expect("el sistema no expone una carpeta de datos para la aplicación");
            let db = Arc::new(
                db::Db::open(&dir.join("library.db")).expect("no se pudo abrir el índice"),
            );

            // Antes que nada: cerrar lo que quedó a medias en la sesión anterior.
            match fileops::reparar(&db) {
                Ok(0) => {}
                Ok(n) => eprintln!("[arranque] {n} operaciones a medias reparadas"),
                Err(e) => eprintln!("[arranque] no se pudo reparar el journal: {e}"),
            }

            // El motor se abre una vez y no se cierra. Si falla, la app sigue viva sin audio:
            // poder mirar y ordenar la biblioteca es mejor que una ventana que no abre.
            let (audio, audio_error) = match audio::arrancar() {
                Ok(h) => (Some(h), None),
                Err(e) => {
                    eprintln!("[arranque] sin audio: {e}");
                    (None, Some(e.to_string()))
                }
            };

            let cancelar = Arc::new(AtomicBool::new(false));
            app.manage(ipc::Estado {
                db: Arc::clone(&db),
                audio,
                audio_error,
                cancelar_analisis: Arc::clone(&cancelar),
            });

            // Lo que quedó sin analizar de la última sesión se retoma solo, en segundo plano.
            std::thread::Builder::new()
                .name("analisis-arranque".into())
                .spawn(move || {
                    if let Err(e) = scan::analyzer::analizar_pendientes(&db, &cancelar, |_| {}) {
                        eprintln!("[arranque] análisis pendiente: {e}");
                    }
                })
                .ok();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::settings::app_info,
            ipc::settings::settings_get,
            ipc::settings::settings_set,
            ipc::labels::labels_extract_all,
            ipc::labels::labels_stats,
            ipc::labels::labels_of,
            ipc::labels::labels_set,
            ipc::labels::labels_clear,
            ipc::labels::labels_sampling,
            ipc::meta::tags_of,
            ipc::meta::tags_add,
            ipc::meta::tags_remove,
            ipc::meta::tags_all,
            ipc::meta::notes_set,
            ipc::triage::trash_list,
            ipc::triage::trash_restore,
            ipc::library::library_sources,
            ipc::library::library_add_source,
            ipc::library::library_rescan,
            ipc::library::library_remove_source,
            ipc::library::library_page,
            ipc::library::library_index_of,
            ipc::library::library_stats,
            ipc::library::library_detail,
            ipc::library::library_peaks,
            ipc::library::library_set_rating,
            ipc::library::library_analysis_pending,
            ipc::player::player_play,
            ipc::player::player_stop,
            ipc::player::player_seek,
            ipc::player::player_gain,
            ipc::player::player_set_loop,
            ipc::player::player_prefetch,
            ipc::player::player_info,
            ipc::triage::triage_projects,
            ipc::triage::triage_last_project,
            ipc::triage::triage_create_project,
            ipc::triage::triage_open_project,
            ipc::triage::triage_delete_project,
            ipc::triage::triage_set_mode,
            ipc::triage::triage_destinations,
            ipc::triage::triage_create_destination,
            ipc::triage::triage_rename_destination,
            ipc::triage::triage_delete_destination,
            ipc::triage::triage_suggest_destinations,
            ipc::triage::triage_send,
            ipc::triage::triage_reject,
            ipc::triage::triage_keep,
            ipc::triage::triage_rename,
            ipc::triage::triage_export,
            ipc::triage::triage_undo,
            ipc::triage::triage_redo,
            ipc::triage::triage_progress,
            ipc::triage::triage_remember,
            ipc::triage::triage_last_sample,
            ipc::triage::triage_trash_summary,
            ipc::triage::triage_empty_trash,
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo arrancar SampleCurator");
}
