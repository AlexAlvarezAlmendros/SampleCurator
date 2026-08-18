//! SQL a mano, preparado y cacheado. Son pocas consultas y todas están en el camino caliente:
//! un ORM aquí solo escondería lo que hay que poder perfilar.

use crate::db::ahora_ms;
use crate::domain::*;
use crate::error::{AppError, Result};
use rusqlite::{
    params, params_from_iter, types::Value, Connection, OptionalExtension, Transaction,
};
use std::path::PathBuf;

/// Ruta absoluta actual de un sample: si se ha movido, manda `current_path`.
const ABS: &str = "COALESCE(s.current_path, so.path || '/' || s.rel_path)";

const COLUMNAS_FILA: &str = "
    s.id, s.filename, s.rel_path, s.ext, s.size, s.duration_ms, s.sample_rate, s.channels,
    (s.analyzed_at IS NOT NULL) AS analizado, s.status, s.rating,
    (s.content_hash IS NOT NULL AND EXISTS (
        SELECT 1 FROM samples x WHERE x.content_hash = s.content_hash AND x.id <> s.id
    )) AS duplicado,
    d.name AS dest_name";

fn fila(r: &rusqlite::Row<'_>) -> rusqlite::Result<SampleRow> {
    Ok(SampleRow {
        id: r.get(0)?,
        filename: r.get(1)?,
        rel_path: r.get(2)?,
        ext: r.get(3)?,
        size: r.get(4)?,
        duration_ms: r.get(5)?,
        sample_rate: r.get(6)?,
        channels: r.get(7)?,
        analyzed: r.get::<_, i64>(8)? != 0,
        status: SampleStatus::parse(&r.get::<_, String>(9)?),
        rating: r.get(10)?,
        duplicate: r.get::<_, i64>(11)? != 0,
        destination: r.get(12)?,
    })
}

// ─────────────────────────── fuentes ───────────────────────────

pub fn add_source(conn: &Connection, path: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO sources (path, added_at) VALUES (?1, ?2)
         ON CONFLICT(path) DO NOTHING",
        params![path, ahora_ms()],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM sources WHERE path = ?1",
        params![path],
        |r| r.get(0),
    )?;
    Ok(id)
}

pub fn sources(conn: &Connection) -> Result<Vec<SourceInfo>> {
    let mut st = conn.prepare_cached(
        "SELECT so.id, so.path, so.added_at,
                (SELECT count(*) FROM samples s WHERE s.source_id = so.id),
                (SELECT count(*) FROM samples s WHERE s.source_id = so.id AND s.analyzed_at IS NOT NULL)
         FROM sources so ORDER BY so.added_at",
    )?;
    let filas = st.query_map([], |r| {
        Ok(SourceInfo {
            id: r.get(0)?,
            path: r.get(1)?,
            added_at: r.get(2)?,
            total: r.get(3)?,
            analyzed: r.get(4)?,
        })
    })?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn remove_source(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM sources WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn touch_source(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE sources SET last_scan = ?2 WHERE id = ?1",
        params![id, ahora_ms()],
    )?;
    Ok(())
}

// ─────────────────────────── indexado ───────────────────────────

pub struct EntradaEscaneo {
    pub rel_path: String,
    pub filename: String,
    pub ext: String,
    pub size: i64,
    pub mtime: i64,
}

/// Inserta o actualiza un lote. Si cambian tamaño o mtime, el análisis se invalida
/// poniendo `analyzed_at = NULL`: es la forma barata de no volver a abrir 50.000 archivos.
pub fn upsert_batch(tx: &Transaction<'_>, source_id: i64, lote: &[EntradaEscaneo]) -> Result<()> {
    let mut st = tx.prepare_cached(
        "INSERT INTO samples (source_id, rel_path, filename, ext, size, mtime)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(source_id, rel_path) DO UPDATE SET
            size  = excluded.size,
            mtime = excluded.mtime,
            analyzed_at = CASE
                WHEN samples.size <> excluded.size OR samples.mtime <> excluded.mtime
                THEN NULL ELSE samples.analyzed_at END,
            broken = CASE
                WHEN samples.size <> excluded.size OR samples.mtime <> excluded.mtime
                THEN 0 ELSE samples.broken END",
    )?;
    for e in lote {
        st.execute(params![
            source_id, e.rel_path, e.filename, e.ext, e.size, e.mtime
        ])?;
    }
    Ok(())
}

pub struct Analisis {
    pub duration_ms: i64,
    pub sample_rate: i64,
    pub channels: i64,
    pub bit_depth: Option<i64>,
    pub loudness_db: f64,
    pub peaks: Vec<u8>,
    pub content_hash: Option<Vec<u8>>,
}

pub fn store_analysis(tx: &Transaction<'_>, id: i64, a: &Analisis) -> Result<()> {
    let mut st = tx.prepare_cached(
        "UPDATE samples SET duration_ms = ?2, sample_rate = ?3, channels = ?4, bit_depth = ?5,
                            loudness_db = ?6, peaks = ?7, content_hash = ?8,
                            analyzed_at = ?9, broken = 0
         WHERE id = ?1",
    )?;
    st.execute(params![
        id,
        a.duration_ms,
        a.sample_rate,
        a.channels,
        a.bit_depth,
        a.loudness_db,
        a.peaks,
        a.content_hash,
        ahora_ms()
    ])?;
    Ok(())
}

pub fn mark_broken(tx: &Transaction<'_>, id: i64) -> Result<()> {
    tx.prepare_cached("UPDATE samples SET broken = 1, analyzed_at = ?2 WHERE id = ?1")?
        .execute(params![id, ahora_ms()])?;
    Ok(())
}

/// Cola de análisis. El índice parcial hace esto O(log n) sin engordar el índice general.
pub fn pending_analysis(conn: &Connection, limite: i64) -> Result<Vec<(i64, PathBuf)>> {
    let sql = format!(
        "SELECT s.id, {ABS} FROM samples s
         JOIN sources so ON so.id = s.source_id
         WHERE s.analyzed_at IS NULL AND s.broken = 0
         ORDER BY s.id LIMIT ?1"
    );
    let mut st = conn.prepare_cached(&sql)?;
    let filas = st.query_map(params![limite], |r| {
        Ok((r.get::<_, i64>(0)?, PathBuf::from(r.get::<_, String>(1)?)))
    })?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn count_pending_analysis(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM samples WHERE analyzed_at IS NULL AND broken = 0",
        [],
        |r| r.get(0),
    )?)
}

// ─────────────────────────── consulta de la lista ───────────────────────────

/// Convierte lo que escribe el usuario en una expresión MATCH de FTS5 con prefijos.
fn expresion_fts(texto: &str) -> Option<String> {
    let terminos: Vec<String> = texto
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect::<String>()
        })
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect();
    if terminos.is_empty() {
        None
    } else {
        Some(terminos.join(" AND "))
    }
}

struct Filtro {
    where_sql: String,
    join_sql: String,
    params: Vec<Value>,
}

fn construir_filtro(q: &LibraryQuery) -> Filtro {
    let mut condiciones: Vec<String> = Vec::new();
    let mut params: Vec<Value> = Vec::new();
    let mut join = String::new();

    if let Some(sid) = q.source_id {
        condiciones.push("s.source_id = ?".into());
        params.push(Value::Integer(sid));
    }
    match q.status {
        StatusFilter::All => {}
        StatusFilter::Pending => condiciones.push("s.status = 'pending'".into()),
        StatusFilter::Decided => condiciones.push("s.status <> 'pending'".into()),
        StatusFilter::Kept => condiciones.push("s.status IN ('kept','moved')".into()),
        StatusFilter::Rejected => condiciones.push("s.status = 'rejected'".into()),
        StatusFilter::Duplicates => condiciones.push(
            "(s.content_hash IS NOT NULL AND EXISTS (
                SELECT 1 FROM samples x WHERE x.content_hash = s.content_hash AND x.id <> s.id))"
                .into(),
        ),
    }
    if let Some(min) = q.min_duration_ms {
        condiciones.push("(s.duration_ms IS NULL OR s.duration_ms >= ?)".into());
        params.push(Value::Integer(min));
    }
    if let Some(max) = q.max_duration_ms {
        condiciones.push("(s.duration_ms IS NULL OR s.duration_ms <= ?)".into());
        params.push(Value::Integer(max));
    }
    if q.min_rating > 0 {
        condiciones.push("s.rating >= ?".into());
        params.push(Value::Integer(q.min_rating));
    }
    if q.unrated {
        condiciones.push("s.rating = 0".into());
    }
    if let Some(dest) = q.dest_id {
        condiciones.push("s.dest_id = ?".into());
        params.push(Value::Integer(dest));
    }
    if let Some(etiqueta) = q.tag.as_deref() {
        condiciones.push(
            "EXISTS (SELECT 1 FROM sample_tags st JOIN tags t ON t.id = st.tag_id
                     WHERE st.sample_id = s.id AND t.name = ?)"
                .into(),
        );
        params.push(Value::Text(crate::db::tags::normalizar(etiqueta)));
    }
    if let Some(texto) = q.search.as_deref().and_then(expresion_fts) {
        join.push_str(" JOIN samples_fts f ON f.rowid = s.id ");
        condiciones.push("samples_fts MATCH ?".into());
        params.push(Value::Text(texto));
    }

    let where_sql = if condiciones.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", condiciones.join(" AND "))
    };
    Filtro {
        where_sql,
        join_sql: join,
        params,
    }
}

fn orden(sort: SortBy) -> &'static str {
    match sort {
        SortBy::Path => "s.rel_path COLLATE NOCASE ASC, s.id ASC",
        SortBy::Filename => "s.filename COLLATE NOCASE ASC, s.id ASC",
        SortBy::Duration => "s.duration_ms IS NULL, s.duration_ms ASC, s.id ASC",
        SortBy::Size => "s.size DESC, s.id ASC",
        SortBy::Loudness => "s.loudness_db IS NULL, s.loudness_db DESC, s.id ASC",
        SortBy::Recent => "s.mtime DESC, s.id ASC",
    }
}

pub fn page(conn: &Connection, q: &LibraryQuery) -> Result<LibraryPage> {
    let f = construir_filtro(q);
    let limite = q.limit.clamp(1, 1000);
    let offset = q.offset.max(0);

    let sql_total = format!(
        "SELECT count(*) FROM samples s{}{}",
        f.join_sql, f.where_sql
    );
    let total: i64 = conn.query_row(&sql_total, params_from_iter(f.params.iter()), |r| r.get(0))?;

    let sql = format!(
        "SELECT {COLUMNAS_FILA} FROM samples s{}
         LEFT JOIN destinations d ON d.id = s.dest_id{}
         ORDER BY {} LIMIT ? OFFSET ?",
        f.join_sql,
        f.where_sql,
        orden(q.sort)
    );
    let mut ps = f.params.clone();
    ps.push(Value::Integer(limite));
    ps.push(Value::Integer(offset));

    let mut st = conn.prepare_cached(&sql)?;
    let filas = st.query_map(params_from_iter(ps.iter()), fila)?;
    Ok(LibraryPage {
        rows: filas.collect::<rusqlite::Result<Vec<_>>>()?,
        total,
        offset,
    })
}

/// Posición de un sample dentro del filtro actual: lo necesita el "ir a" y el reanudar sesión.
pub fn index_of(conn: &Connection, q: &LibraryQuery, sample_id: i64) -> Result<Option<i64>> {
    let f = construir_filtro(q);
    let sql = format!(
        "SELECT n FROM (
            SELECT s.id AS sid, (ROW_NUMBER() OVER (ORDER BY {})) - 1 AS n
            FROM samples s{}{}
         ) WHERE sid = ?",
        orden(q.sort),
        f.join_sql,
        f.where_sql
    );
    let mut ps = f.params.clone();
    ps.push(Value::Integer(sample_id));
    let mut st = conn.prepare_cached(&sql)?;
    Ok(st
        .query_row(params_from_iter(ps.iter()), |r| r.get::<_, i64>(0))
        .optional()?)
}

pub fn stats(conn: &Connection, source_id: Option<i64>) -> Result<LibraryStats> {
    let (cond, ps): (&str, Vec<Value>) = match source_id {
        Some(id) => (" WHERE source_id = ?", vec![Value::Integer(id)]),
        None => ("", vec![]),
    };
    let sql = format!(
        "SELECT count(*),
                sum(status = 'pending'),
                sum(status = 'kept'),
                sum(status = 'rejected'),
                sum(status = 'moved'),
                sum(analyzed_at IS NOT NULL)
         FROM samples{cond}"
    );
    let (total, pending, kept, rejected, moved, analyzed): (i64, i64, i64, i64, i64, i64) = conn
        .query_row(&sql, params_from_iter(ps.iter()), |r| {
            Ok((
                r.get(0)?,
                r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                r.get::<_, Option<i64>>(5)?.unwrap_or(0),
            ))
        })?;
    let duplicates: i64 = conn.query_row(
        "SELECT count(*) FROM samples s WHERE s.content_hash IS NOT NULL
         AND EXISTS (SELECT 1 FROM samples x WHERE x.content_hash = s.content_hash AND x.id <> s.id)",
        [],
        |r| r.get(0),
    )?;
    Ok(LibraryStats {
        total,
        pending,
        kept,
        rejected,
        moved,
        analyzed,
        duplicates,
    })
}

// ─────────────────────────── un sample ───────────────────────────

pub fn abs_path(conn: &Connection, id: i64) -> Result<PathBuf> {
    let sql = format!(
        "SELECT {ABS} FROM samples s JOIN sources so ON so.id = s.source_id WHERE s.id = ?1"
    );
    let p: Option<String> = conn
        .prepare_cached(&sql)?
        .query_row(params![id], |r| r.get(0))
        .optional()?;
    p.map(PathBuf::from)
        .ok_or_else(|| AppError::NotFound(format!("sample {id}")))
}

/// Rutas de varios samples de una vez: lo usa el prefetch, que pide los vecinos en bloque.
pub fn abs_paths(conn: &Connection, ids: &[i64]) -> Result<Vec<(i64, PathBuf)>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let marcadores = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT s.id, {ABS} FROM samples s JOIN sources so ON so.id = s.source_id
         WHERE s.id IN ({marcadores})"
    );
    let mut st = conn.prepare_cached(&sql)?;
    let filas = st.query_map(params_from_iter(ids.iter()), |r| {
        Ok((r.get::<_, i64>(0)?, PathBuf::from(r.get::<_, String>(1)?)))
    })?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn peaks(conn: &Connection, id: i64) -> Result<Vec<u8>> {
    let p: Option<Option<Vec<u8>>> = conn
        .prepare_cached("SELECT peaks FROM samples WHERE id = ?1")?
        .query_row(params![id], |r| r.get(0))
        .optional()?;
    Ok(p.flatten().unwrap_or_default())
}

pub fn detail(conn: &Connection, id: i64) -> Result<SampleDetail> {
    let sql = format!(
        "SELECT {COLUMNAS_FILA}, {ABS}, s.loudness_db, s.bit_depth, s.notes
         FROM samples s
         JOIN sources so ON so.id = s.source_id
         LEFT JOIN destinations d ON d.id = s.dest_id
         WHERE s.id = ?1"
    );
    let mut st = conn.prepare_cached(&sql)?;
    let d = st
        .query_row(params![id], |r| {
            Ok(SampleDetail {
                row: fila(r)?,
                abs_path: r.get(13)?,
                loudness_db: r.get(14)?,
                bit_depth: r.get(15)?,
                tags: Vec::new(),
                notes: r.get(16)?,
            })
        })
        .optional()?;
    let mut d = d.ok_or_else(|| AppError::NotFound(format!("sample {id}")))?;
    d.tags = crate::db::tags::de_sample(conn, id)?;
    Ok(d)
}

pub fn set_rating(conn: &Connection, id: i64, rating: i64) -> Result<()> {
    conn.execute(
        "UPDATE samples SET rating = ?2 WHERE id = ?1",
        params![id, rating.clamp(0, 5)],
    )?;
    Ok(())
}

pub fn mark_seen(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE samples SET seen_at = ?2 WHERE id = ?1",
        params![id, ahora_ms()],
    )?;
    Ok(())
}
