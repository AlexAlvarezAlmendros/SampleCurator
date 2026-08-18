//! Consultas del conjunto de evaluación (Fase 8).
//!
//! El punto de todo esto es poder responder a una pregunta con números: **¿cuánto acierta lo
//! que decimos de cada sample?** Para eso hacen falta dos referencias que se puedan comparar,
//! y por eso las etiquetas se guardan por origen en vez de machacar un único campo.

use crate::db::ahora_ms;
use crate::domain::LabelSource;
use crate::error::Result;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

pub const CAMPOS: [&str; 4] = ["kind", "bpm", "key", "pitch"];

#[derive(Debug, Clone)]
pub struct Etiqueta {
    pub field: String,
    pub value: String,
    pub confidence: f64,
    pub source: LabelSource,
}

pub fn upsert(
    tx: &Transaction<'_>,
    sample_id: i64,
    field: &str,
    value: &str,
    confidence: f64,
    source: LabelSource,
) -> Result<()> {
    tx.prepare_cached(
        "INSERT INTO labels (sample_id, field, value, confidence, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(sample_id, field, source) DO UPDATE SET
            value = excluded.value,
            confidence = excluded.confidence,
            created_at = excluded.created_at",
    )?
    .execute(params![
        sample_id,
        field,
        value,
        confidence,
        source.as_str(),
        ahora_ms()
    ])?;
    Ok(())
}

pub fn de_sample(conn: &Connection, sample_id: i64) -> Result<Vec<Etiqueta>> {
    let mut st = conn.prepare_cached(
        "SELECT field, value, confidence, source FROM labels WHERE sample_id = ?1",
    )?;
    let filas = st.query_map(params![sample_id], |r| {
        Ok(Etiqueta {
            field: r.get(0)?,
            value: r.get(1)?,
            confidence: r.get(2)?,
            source: LabelSource::parse(&r.get::<_, String>(3)?),
        })
    })?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn borrar(conn: &Connection, sample_id: i64, field: &str, source: LabelSource) -> Result<()> {
    conn.execute(
        "DELETE FROM labels WHERE sample_id = ?1 AND field = ?2 AND source = ?3",
        params![sample_id, field, source.as_str()],
    )?;
    Ok(())
}

pub fn contar(conn: &Connection, field: &str, source: LabelSource) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM labels WHERE field = ?1 AND source = ?2",
        params![field, source.as_str()],
        |r| r.get(0),
    )?)
}

/// Pares (referencia débil, verdad del usuario) de un campo. Es exactamente lo que hay que
/// comparar para saber cuánto mienten los nombres de archivo.
pub fn pares_para_evaluar(conn: &Connection, field: &str) -> Result<Vec<(i64, String, String)>> {
    let mut st = conn.prepare(
        "SELECT u.sample_id, f.value, u.value
         FROM labels u
         JOIN labels f ON f.sample_id = u.sample_id AND f.field = u.field AND f.source = 'filename'
         WHERE u.field = ?1 AND u.source = 'user'
         ORDER BY u.sample_id",
    )?;
    let filas = st.query_map(params![field], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Cuántos samples tienen verdad del usuario pero NO referencia débil: el material que el
/// nombre no sabía describir, que es justo donde el clasificador tendrá que ganarse el sueldo.
pub fn solo_usuario(conn: &Connection, field: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM labels u
         WHERE u.field = ?1 AND u.source = 'user'
           AND NOT EXISTS (
             SELECT 1 FROM labels f
             WHERE f.sample_id = u.sample_id AND f.field = u.field AND f.source = 'filename')",
        params![field],
        |r| r.get(0),
    )?)
}

/// Muestreo estratificado para la sesión de etiquetado.
///
/// Coge `por_clase` samples de cada tipo según la referencia débil, más un puñado de los que
/// no tienen ninguna pista. Sin esto, un muestreo al azar sobre una librería llena de kicks
/// daría 150 kicks y ningún tom, y no se podría medir nada por clase.
pub fn muestreo_estratificado(conn: &Connection, por_clase: i64) -> Result<Vec<i64>> {
    let mut ids = Vec::new();

    let mut st = conn.prepare(
        "SELECT s.id FROM samples s
         JOIN labels l ON l.sample_id = s.id AND l.field = 'kind' AND l.source = 'filename'
         WHERE l.value = ?1
           AND NOT EXISTS (SELECT 1 FROM labels u
                           WHERE u.sample_id = s.id AND u.field = 'kind' AND u.source = 'user')
         ORDER BY s.content_hash, s.id
         LIMIT ?2",
    )?;
    for kind in crate::domain::SampleKind::TODOS {
        let filas = st.query_map(params![kind.as_str(), por_clase], |r| r.get::<_, i64>(0))?;
        for id in filas {
            ids.push(id?);
        }
    }

    // Y material sin ninguna pista en el nombre: es donde más falta hace saber la verdad.
    let mut st = conn.prepare(
        "SELECT s.id FROM samples s
         WHERE NOT EXISTS (SELECT 1 FROM labels l WHERE l.sample_id = s.id AND l.field = 'kind')
         ORDER BY s.content_hash, s.id
         LIMIT ?1",
    )?;
    let filas = st.query_map(params![por_clase * 2], |r| r.get::<_, i64>(0))?;
    for id in filas {
        ids.push(id?);
    }

    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Samples pendientes de etiquetar de una lista dada, en orden.
pub fn siguiente_sin_etiquetar(conn: &Connection, desde_id: i64) -> Result<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT s.id FROM samples s
             WHERE s.id > ?1
               AND NOT EXISTS (SELECT 1 FROM labels u
                               WHERE u.sample_id = s.id AND u.field = 'kind' AND u.source = 'user')
             ORDER BY s.id LIMIT 1",
            params![desde_id],
            |r| r.get::<_, i64>(0),
        )
        .optional()?)
}
