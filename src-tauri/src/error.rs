//! Un único tipo de error para toda la app. Los comandos IPC lo serializan como
//! `{ kind, message }`, nunca como un String suelto: el frontend necesita poder distinguir
//! un "carpeta no encontrada" de un "disco lleno" sin parsear texto.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("entrada no válida: {0}")]
    InvalidInput(String),

    #[error("no encontrado: {0}")]
    NotFound(String),

    #[error("la ruta no existe o no es accesible: {0}")]
    PathUnavailable(PathBuf),

    #[error("no se pudo leer el audio de {path}: {reason}")]
    Decode { path: PathBuf, reason: String },

    #[error("error de audio: {0}")]
    Audio(String),

    #[error("error de base de datos: {0}")]
    Db(String),

    #[error("error de disco: {0}")]
    Io(String),

    #[error("la operación dejaría datos en peligro: {0}")]
    Unsafe(String),

    #[error("no hay nada que deshacer")]
    NothingToUndo,

    #[error("no se pudo actualizar: {0}")]
    Update(String),
}

impl AppError {
    /// Categoría estable que el frontend puede usar para decidir qué hacer.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::NotFound(_) => "not_found",
            Self::PathUnavailable(_) => "path_unavailable",
            Self::Decode { .. } => "decode",
            Self::Audio(_) => "audio",
            Self::Db(_) => "db",
            Self::Io(_) => "io",
            Self::Unsafe(_) => "unsafe",
            Self::NothingToUndo => "nothing_to_undo",
            Self::Update(_) => "update",
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
