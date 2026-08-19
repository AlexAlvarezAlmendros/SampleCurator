//! Actualizador: comprobar si hay versión nueva e instalarla sin salir de la app.
//!
//! Capa fina sobre `tauri_plugin_updater`. Toda la confianza viene de la firma: el paquete se
//! verifica contra la clave pública que va compilada en la app, así que un endpoint tomado por
//! alguien más no basta para colar nada.

use crate::domain::{UpdateInfo, UpdateProgress};
use crate::error::{AppError, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

/// Un mensaje de progreso cada 100 ms como mucho: la descarga son cientos de trozos y un
/// mensaje por trozo saturaría el puente IPC sin que se note en pantalla.
const CADA: Duration = Duration::from_millis(100);

/// ¿Puede esta instalación reemplazarse a sí misma?
///
/// En Linux el actualizador solo sabe con AppImage —reescribe el propio fichero—, y la variable
/// `APPIMAGE` es justo lo que dice si venimos de uno. Desde un `.deb` mandan `apt` y compañía:
/// tocar ahí los archivos por nuestra cuenta dejaría el sistema de paquetes mintiendo.
fn puede_instalarse() -> bool {
    if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE").is_some()
    } else {
        true
    }
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<Option<UpdateInfo>> {
    let updater = app
        .updater()
        .map_err(|e| AppError::Update(format!("no se pudo preparar el actualizador: {e}")))?;

    let encontrada = updater
        .check()
        .await
        .map_err(|e| AppError::Update(format!("no se pudo comprobar si hay versión nueva: {e}")))?;

    match &encontrada {
        Some(u) => eprintln!(
            "[actualizador] hay {} disponible (tienes la {})",
            u.version, u.current_version
        ),
        None => eprintln!("[actualizador] no hay versión nueva"),
    }

    Ok(encontrada.map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone(),
        can_install: puede_instalarse(),
    }))
}

/// Descarga e instala. Al terminar la app se reinicia sola: es la única forma de que el binario
/// nuevo esté corriendo, y hacerlo aquí evita dejar al usuario con una versión a medias.
#[tauri::command]
pub async fn update_install(app: AppHandle, progreso: Channel<UpdateProgress>) -> Result<()> {
    if !puede_instalarse() {
        return Err(AppError::Update(
            "esta instalación viene de un paquete del sistema y se actualiza desde ahí".into(),
        ));
    }

    let updater = app
        .updater()
        .map_err(|e| AppError::Update(format!("no se pudo preparar el actualizador: {e}")))?;
    let Some(update) = updater
        .check()
        .await
        .map_err(|e| AppError::Update(format!("no se pudo comprobar si hay versión nueva: {e}")))?
    else {
        return Err(AppError::Update("ya estás en la última versión".into()));
    };

    let llevamos = AtomicU64::new(0);
    let ultimo = Mutex::new(Instant::now() - CADA);
    let canal = progreso.clone();

    update
        .download_and_install(
            move |trozo, total| {
                let hechos = llevamos.fetch_add(trozo as u64, Ordering::Relaxed) + trozo as u64;
                // El throttle no puede tragarse el final: si ya está todo, se manda igual.
                let completo = total.is_some_and(|t| hechos >= t);
                let toca = match ultimo.lock() {
                    Ok(mut u) if u.elapsed() >= CADA || completo => {
                        *u = Instant::now();
                        true
                    }
                    _ => false,
                };
                if toca {
                    let _ = canal.send(UpdateProgress {
                        downloaded: hechos as i64,
                        total: total.unwrap_or(0) as i64,
                        done: false,
                    });
                }
            },
            || {},
        )
        .await
        .map_err(|e| AppError::Update(format!("no se pudo instalar la actualización: {e}")))?;

    let _ = progreso.send(UpdateProgress {
        downloaded: 0,
        total: 0,
        done: true,
    });

    app.restart();
}
