//! Comandos del conjunto de evaluación (Fase 8, gate 8.0).
//!
//! Esto no es todavía el clasificador: es la báscula con la que se pesará. Sin ella, cualquier
//! número que diera el clasificador sería una opinión.

use super::Estado;
use crate::db::labels;
use crate::domain::*;
use crate::error::{AppError, Result};
use crate::scan;
use std::sync::Arc;
use tauri::State;

/// Pasa el extractor de nombres por toda la biblioteca. Idempotente: se puede repetir cada vez
/// que mejore el extractor.
#[tauri::command]
pub async fn labels_extract_all(estado: State<'_, Estado>) -> Result<LabelReport> {
    let db = Arc::clone(&estado.db);
    // Recorrer 50.000 nombres no puede bloquear el hilo de los comandos.
    tauri::async_runtime::spawn_blocking(move || scan::labeling::extraer_de_nombres(&db))
        .await
        .map_err(|e| AppError::Io(format!("la extracción no terminó: {e}")))?
}

#[tauri::command]
pub async fn labels_stats(estado: State<'_, Estado>) -> Result<LabelStats> {
    let db = Arc::clone(&estado.db);
    tauri::async_runtime::spawn_blocking(move || scan::labeling::cobertura(&db))
        .await
        .map_err(|e| AppError::Io(format!("el cálculo no terminó: {e}")))?
}

/// Etiquetas conocidas de un sample. Cuando hay varias fuentes para el mismo campo manda la
/// del usuario: es la única que no se discute.
#[tauri::command]
pub async fn labels_of(estado: State<'_, Estado>, sample_id: i64) -> Result<SampleLabels> {
    let etiquetas = estado.db.read(|c| labels::de_sample(c, sample_id))?;

    let elegir = |campo: &str| -> Option<(String, LabelSource)> {
        let mut mejor: Option<(String, LabelSource)> = None;
        for e in etiquetas.iter().filter(|e| e.field == campo) {
            let prioridad = |s: LabelSource| match s {
                LabelSource::User => 3,
                LabelSource::Audio => 2,
                LabelSource::Filename => 1,
            };
            if mejor
                .as_ref()
                .is_none_or(|(_, s)| prioridad(e.source) > prioridad(*s))
            {
                mejor = Some((e.value.clone(), e.source));
            }
        }
        mejor
    };

    let kind = elegir("kind");
    let bpm = elegir("bpm");
    let key = elegir("key");

    Ok(SampleLabels {
        sample_id,
        kind: kind.as_ref().map(|(v, _)| SampleKind::parse(v)),
        kind_source: kind.as_ref().map(|(_, s)| *s),
        bpm: bpm.as_ref().and_then(|(v, _)| v.parse::<f64>().ok()),
        bpm_source: bpm.as_ref().map(|(_, s)| *s),
        key: key.as_ref().map(|(v, _)| v.clone()),
        key_source: key.as_ref().map(|(_, s)| *s),
    })
}

/// Corrección del usuario: la verdad de referencia.
#[tauri::command]
pub async fn labels_set(
    estado: State<'_, Estado>,
    sample_id: i64,
    field: String,
    value: String,
) -> Result<()> {
    if !labels::CAMPOS.contains(&field.as_str()) {
        return Err(AppError::InvalidInput(format!(
            "campo desconocido: {field}"
        )));
    }
    let limpio = value.trim();
    if limpio.is_empty() {
        return estado
            .db
            .read(|c| labels::borrar(c, sample_id, &field, LabelSource::User));
    }
    // Se valida el formato aquí para que la báscula no acabe llena de basura.
    match field.as_str() {
        "bpm" => {
            let n: f64 = limpio
                .parse()
                .map_err(|_| AppError::InvalidInput(format!("«{limpio}» no es un tempo")))?;
            if !(20.0..=400.0).contains(&n) {
                return Err(AppError::InvalidInput(
                    "el tempo está fuera de rango".into(),
                ));
            }
        }
        "key" => {
            KeyLabel::parse(limpio).ok_or_else(|| {
                AppError::InvalidInput(format!("«{limpio}» no es una tonalidad (usa «A:min»)"))
            })?;
        }
        "kind" if SampleKind::parse(limpio) == SampleKind::Unknown && limpio != "unknown" => {
            return Err(AppError::InvalidInput(format!(
                "tipo desconocido: {limpio}"
            )));
        }
        _ => {}
    }

    estado.db.write(|conn| {
        let tx = conn.transaction()?;
        labels::upsert(&tx, sample_id, &field, limpio, 1.0, LabelSource::User)?;
        tx.commit()?;
        Ok(())
    })
}

#[tauri::command]
pub async fn labels_clear(estado: State<'_, Estado>, sample_id: i64, field: String) -> Result<()> {
    estado
        .db
        .read(|c| labels::borrar(c, sample_id, &field, LabelSource::User))
}

/// Lista de samples a etiquetar, estratificada por tipo para que no salgan 150 kicks y ningún
/// tom: sin cubrir todas las clases no se puede medir el acierto por clase.
#[tauri::command]
pub async fn labels_sampling(estado: State<'_, Estado>, per_class: i64) -> Result<Vec<i64>> {
    estado
        .db
        .read(|c| labels::muestreo_estratificado(c, per_class.clamp(1, 200)))
}
