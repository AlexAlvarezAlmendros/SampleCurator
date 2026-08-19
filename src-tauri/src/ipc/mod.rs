//! Capa IPC: parsear, delegar y traducir. Aquí no vive lógica de negocio.
//!
//! Cada comando llama al módulo que corresponde (`db`, `scan`, `audio`, `fileops`) y devuelve
//! `Result<T, AppError>`, que el frontend recibe como `{ kind, message }`.

pub mod labels;
pub mod library;
pub mod meta;
pub mod player;
pub mod settings;
pub mod triage;
pub mod updater;

use crate::audio::AudioHandle;
use crate::db::Db;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Estado global de la app, compartido por todos los comandos.
pub struct Estado {
    pub db: Arc<Db>,
    pub audio: Option<AudioHandle>,
    pub audio_error: Option<String>,
    /// Permite cortar el análisis en marcha al cambiar de biblioteca o al cerrar.
    pub cancelar_analisis: Arc<AtomicBool>,
}

impl Estado {
    pub fn audio(&self) -> crate::error::Result<&AudioHandle> {
        self.audio.as_ref().ok_or_else(|| {
            crate::error::AppError::Audio(
                self.audio_error
                    .clone()
                    .unwrap_or_else(|| "el motor de audio no está disponible".into()),
            )
        })
    }
}
