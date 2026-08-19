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
    pub notes: Option<String>,
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
    /// Solo los que aún no tienes valorados.
    pub unrated: bool,
    /// Solo los enviados a este destino.
    #[ts(type = "number | null")]
    pub dest_id: Option<i64>,
    /// Solo los que llevan esta etiqueta.
    pub tag: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct AudioInfo {
    /// Nombre del dispositivo de salida en uso.
    pub device: String,
    #[ts(type = "number")]
    pub sample_rate: i64,
    #[ts(type = "number")]
    pub channels: i64,
    #[ts(type = "number")]
    pub buffer_frames: i64,
    /// `false` cuando el backend no deja elegir el tamaño de buffer (WASAPI en Windows):
    /// entonces manda el del sistema y `buffer_frames` vale 0.
    pub buffer_fixed: bool,
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
    /// Cuántas veces ha habido que volver a abrir el dispositivo (cascos, Bluetooth, cambio
    /// de salida). Si sube sin que hayas tocado nada, algo va mal con el audio del sistema.
    #[ts(type = "number")]
    pub reconnections: i64,
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

// ─────────────────────────── clasificación (Fase 8) ───────────────────────────

/// Taxonomía cerrada de tipos de sample. Cerrada a propósito: una lista abierta convierte el
/// filtro en un cajón de sastre y hace imposible medir el acierto por clase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub enum SampleKind {
    Kick,
    Snare,
    Clap,
    Hat,
    Cymbal,
    Tom,
    Perc,
    Bass,
    Synth,
    Vocal,
    Fx,
    Loop,
    Unknown,
}

impl SampleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kick => "kick",
            Self::Snare => "snare",
            Self::Clap => "clap",
            Self::Hat => "hat",
            Self::Cymbal => "cymbal",
            Self::Tom => "tom",
            Self::Perc => "perc",
            Self::Bass => "bass",
            Self::Synth => "synth",
            Self::Vocal => "vocal",
            Self::Fx => "fx",
            Self::Loop => "loop",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "kick" => Self::Kick,
            "snare" => Self::Snare,
            "clap" => Self::Clap,
            "hat" => Self::Hat,
            "cymbal" => Self::Cymbal,
            "tom" => Self::Tom,
            "perc" => Self::Perc,
            "bass" => Self::Bass,
            "synth" => Self::Synth,
            "vocal" => Self::Vocal,
            "fx" => Self::Fx,
            "loop" => Self::Loop,
            _ => Self::Unknown,
        }
    }

    /// ¿Puede este tipo tener altura definida? Un hi-hat no tiene nota, y decírselo al
    /// estimador de tonalidad ahorra la mitad de los falsos positivos.
    pub fn puede_ser_tonal(self) -> bool {
        matches!(
            self,
            Self::Bass | Self::Synth | Self::Vocal | Self::Loop | Self::Fx | Self::Unknown
        )
    }

    /// ¿Tiene sentido preguntarle el tempo? Solo lo que es un bucle.
    pub fn puede_tener_bpm(self) -> bool {
        matches!(self, Self::Loop | Self::Unknown)
    }

    pub const TODOS: [SampleKind; 13] = [
        Self::Kick,
        Self::Snare,
        Self::Clap,
        Self::Hat,
        Self::Cymbal,
        Self::Tom,
        Self::Perc,
        Self::Bass,
        Self::Synth,
        Self::Vocal,
        Self::Fx,
        Self::Loop,
        Self::Unknown,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub enum KeyMode {
    Major,
    Minor,
}

/// Tonalidad: clase de altura (0 = Do) y modo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct KeyLabel {
    #[ts(type = "number")]
    pub root: i64,
    pub mode: KeyMode,
}

pub const NOTAS: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

impl KeyLabel {
    pub fn as_str(self) -> String {
        let nota = NOTAS
            .get(self.root.rem_euclid(12) as usize)
            .copied()
            .unwrap_or("C");
        match self.mode {
            KeyMode::Major => format!("{nota}:maj"),
            KeyMode::Minor => format!("{nota}:min"),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let (nota, modo) = s.split_once(':')?;
        let root = NOTAS.iter().position(|n| *n == nota)? as i64;
        let mode = match modo {
            "maj" => KeyMode::Major,
            "min" => KeyMode::Minor,
            _ => return None,
        };
        Some(Self { root, mode })
    }

    /// Relativo mayor/menor: Do mayor y La menor comparten notas, y confundirlos es el error
    /// clásico de cualquier estimador. Se cuenta aparte en la evaluación.
    pub fn es_relativo_de(self, otra: Self) -> bool {
        match (self.mode, otra.mode) {
            (KeyMode::Major, KeyMode::Minor) => (self.root - 3).rem_euclid(12) == otra.root,
            (KeyMode::Minor, KeyMode::Major) => (self.root + 3).rem_euclid(12) == otra.root,
            _ => false,
        }
    }

    pub fn es_paralelo_de(self, otra: Self) -> bool {
        self.root == otra.root && self.mode != otra.mode
    }

    pub fn es_quinta_de(self, otra: Self) -> bool {
        self.mode == otra.mode
            && ((self.root + 7).rem_euclid(12) == otra.root
                || (self.root - 7).rem_euclid(12) == otra.root)
    }
}

/// De dónde sale una etiqueta. `Filename` es la referencia barata y masiva; `User` es la única
/// verdad sin discusión.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub enum LabelSource {
    Filename,
    User,
    Audio,
}

impl LabelSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Filename => "filename",
            Self::User => "user",
            Self::Audio => "audio",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "user" => Self::User,
            "audio" => Self::Audio,
            _ => Self::Filename,
        }
    }
}

/// Etiquetas conocidas de un sample, con su procedencia.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct SampleLabels {
    #[ts(type = "number")]
    pub sample_id: i64,
    pub kind: Option<SampleKind>,
    pub kind_source: Option<LabelSource>,
    pub bpm: Option<f64>,
    pub bpm_source: Option<LabelSource>,
    pub key: Option<String>,
    pub key_source: Option<LabelSource>,
}

/// Resultado de pasar el extractor de nombres por toda la biblioteca.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct LabelReport {
    #[ts(type = "number")]
    pub processed: i64,
    #[ts(type = "number")]
    pub kind: i64,
    #[ts(type = "number")]
    pub bpm: i64,
    #[ts(type = "number")]
    pub key: i64,
    #[ts(type = "number")]
    pub pitch: i64,
    #[ts(type = "number")]
    pub millis: i64,
}

/// Cuánto coincide la referencia barata (nombres) con la verdad del usuario, campo a campo.
/// Es la medida que decide el gate de la Fase 8.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct FieldCoverage {
    pub field: String,
    #[ts(type = "number")]
    pub from_filename: i64,
    #[ts(type = "number")]
    pub from_user: i64,
    /// Etiquetados a mano que el nombre no supo describir: el material donde el clasificador
    /// tendrá que ganarse el sueldo de verdad.
    #[ts(type = "number")]
    pub only_user: i64,
    #[ts(type = "number")]
    pub pairs: i64,
    #[ts(type = "number")]
    pub exact: i64,
    #[ts(type = "number")]
    pub close: i64,
    #[ts(type = "number")]
    pub wrong: i64,
    pub accuracy: f64,
    pub mirex: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct LabelStats {
    pub fields: Vec<FieldCoverage>,
    #[ts(type = "number")]
    pub labeled_samples: i64,
    #[ts(type = "number")]
    pub target: i64,
}

/// Una entrada de la papelera. Puede haber archivos sin fila en el índice —si se quitó la
/// carpeta de origen— y por eso `sample_id` es opcional: se restauran igual.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/bindings.ts")]
pub struct TrashEntry {
    #[ts(type = "number | null")]
    pub sample_id: Option<i64>,
    pub filename: String,
    pub trash_path: String,
    pub original_path: String,
    #[ts(type = "number")]
    pub at: i64,
    #[ts(type = "number")]
    pub size: i64,
    #[ts(type = "number | null")]
    pub duration_ms: Option<i64>,
    /// `false` cuando el archivo está en la papelera pero ya no hay fila que actualizar.
    pub in_index: bool,
}
