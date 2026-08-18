//! Tipos puros del dominio. No dependen de tauri, ni de rusqlite, ni de cpal.
//! Son la única fuente de verdad del contrato con el frontend: `cargo test` los exporta
//! a `src/bindings.ts` con ts-rs.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Los `i64` llevan `#[ts(type = "number")]` a propósito: por defecto ts-rs los mapea a
// `bigint`, pero el puente IPC de Tauri serializa a JSON, donde llegan como `number`.
// Ningún valor de este dominio (ids, tamaños, milisegundos) se acerca a 2^53.

pub const TS_OUT: &str = "../../src/bindings.ts";

// ─────────────────────────── biblioteca ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub enum SampleStatus {
    Pending,
    Kept,
    Rejected,
    Moved,
}

impl SampleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Kept => "kept",
            Self::Rejected => "rejected",
            Self::Moved => "moved",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "kept" => Self::Kept,
            "rejected" => Self::Rejected,
            "moved" => Self::Moved,
            _ => Self::Pending,
        }
    }
}

/// Lo que la lista virtualizada necesita para pintar una fila. Nada más: cada campo de más
/// se multiplica por 100.000 filas.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct SampleRow {
    #[ts(type = "number")]
    pub id: i64,
    pub filename: String,
    pub rel_path: String,
    pub ext: String,
    #[ts(type = "number")]
    pub size: i64,
    #[ts(type = "number | null")]
    pub duration_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub sample_rate: Option<i64>,
    #[ts(type = "number | null")]
    pub channels: Option<i64>,
    pub analyzed: bool,
    pub status: SampleStatus,
    #[ts(type = "number")]
    pub rating: i64,
    pub duplicate: bool,
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct SampleDetail {
    pub row: SampleRow,
    pub abs_path: String,
    pub loudness_db: Option<f64>,
    #[ts(type = "number | null")]
    pub bit_depth: Option<i64>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub enum StatusFilter {
    All,
    Pending,
    Decided,
    Kept,
    Rejected,
    Duplicates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub enum SortBy {
    Path,
    Filename,
    Duration,
    Size,
    Loudness,
    Recent,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct LibraryQuery {
    #[ts(type = "number | null")]
    pub source_id: Option<i64>,
    pub search: Option<String>,
    pub status: StatusFilter,
    pub sort: SortBy,
    /// Filtro de duración en ms. Separa los one-shots de los loops, que es la primera
    /// pregunta que se hace cualquiera al ordenar una librería.
    #[ts(type = "number | null")]
    pub min_duration_ms: Option<i64>,
    #[ts(type = "number | null")]
    pub max_duration_ms: Option<i64>,
    /// Valoración mínima (0 = sin filtrar).
    #[ts(type = "number")]
    pub min_rating: i64,
    #[ts(type = "number")]
    pub offset: i64,
    #[ts(type = "number")]
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct LibraryPage {
    pub rows: Vec<SampleRow>,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct SourceInfo {
    #[ts(type = "number")]
    pub id: i64,
    pub path: String,
    #[ts(type = "number")]
    pub added_at: i64,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub analyzed: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct LibraryStats {
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub pending: i64,
    #[ts(type = "number")]
    pub kept: i64,
    #[ts(type = "number")]
    pub rejected: i64,
    #[ts(type = "number")]
    pub moved: i64,
    #[ts(type = "number")]
    pub analyzed: i64,
    #[ts(type = "number")]
    pub duplicates: i64,
}

/// Progreso del escaneo. Se emite por `Channel` con throttle: nunca uno por archivo.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct ScanProgress {
    #[ts(type = "number")]
    pub found: i64,
    #[ts(type = "number")]
    pub indexed: i64,
    #[ts(type = "number")]
    pub analyzed: i64,
    #[ts(type = "number")]
    pub pending_analysis: i64,
    pub done: bool,
}

// ─────────────────────────── triaje ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub enum TriageMode {
    Move,
    Copy,
}

impl TriageMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Copy => "copy",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "copy" {
            Self::Copy
        } else {
            Self::Move
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct Project {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub dest_root: String,
    pub mode: TriageMode,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct Destination {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub project_id: i64,
    pub name: String,
    pub rel_path: String,
    pub hotkey: Option<String>,
    pub color: String,
    #[ts(type = "number")]
    pub sort_order: i64,
    #[ts(type = "number")]
    pub count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub enum ActionKind {
    Move,
    Copy,
    Reject,
    Rename,
    /// Conservar en su sitio: no toca el disco, pero sí deja rastro para poder deshacerlo.
    Keep,
}

impl ActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Copy => "copy",
            Self::Reject => "reject",
            Self::Rename => "rename",
            Self::Keep => "keep",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "copy" => Self::Copy,
            "reject" => Self::Reject,
            "rename" => Self::Rename,
            "keep" => Self::Keep,
            _ => Self::Move,
        }
    }
}

/// Resultado de una operación de triaje: lo que el frontend necesita para actualizar la UI
/// sin volver a pedir la página entera.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct TriageResult {
    pub batch_id: String,
    #[ts(type = "Array<number>")]
    pub affected: Vec<i64>,
    #[ts(type = "number | null")]
    pub destination_id: Option<i64>,
    #[ts(type = "number | null")]
    pub destination_count: Option<i64>,
    pub kind: ActionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct UndoResult {
    pub batch_id: String,
    #[ts(type = "Array<number>")]
    pub restored: Vec<i64>,
    #[ts(type = "number | null")]
    pub focus_sample_id: Option<i64>,
    pub kind: ActionKind,
    #[ts(type = "number | null")]
    pub destination_id: Option<i64>,
    #[ts(type = "number | null")]
    pub destination_count: Option<i64>,
}

// ─────────────────────────── reproductor ───────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct PlaybackStarted {
    #[ts(type = "number")]
    pub sample_id: i64,
    /// Milisegundos desde el arranque del proceso: el front interpola el cabezal desde aquí
    /// con requestAnimationFrame, sin un solo evento IPC por frame.
    pub started_at_ms: f64,
    pub duration_ms: f64,
    pub start_offset_ms: f64,
    pub looping: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct AudioInfo {
    #[ts(type = "number")]
    pub sample_rate: i64,
    #[ts(type = "number")]
    pub channels: i64,
    #[ts(type = "number")]
    pub buffer_frames: i64,
    #[ts(type = "number")]
    pub cache_bytes: i64,
    #[ts(type = "number")]
    pub cache_limit_bytes: i64,
    #[ts(type = "number")]
    pub cache_entries: i64,
    pub latency_p50_ms: f64,
    pub latency_p95_ms: f64,
    #[ts(type = "number")]
    pub shots: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
    pub audio: Option<AudioInfo>,
    pub audio_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct TrashSummary {
    #[ts(type = "number")]
    pub files: i64,
    #[ts(type = "number")]
    pub bytes: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct SessionProgress {
    #[ts(type = "number")]
    pub done: i64,
    #[ts(type = "number")]
    pub total: i64,
}
