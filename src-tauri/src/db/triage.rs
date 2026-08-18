//! Consultas de proyectos, destinos y del journal de acciones.
//!
//! El journal es la pieza que hace que nada se pierda: la fila se escribe ANTES de tocar el
//! disco (`done_at IS NULL`) y se cierra después. Si el proceso muere a mitad, al arrancar
//! se ven las acciones a medias y se reparan.

use crate::db::ahora_ms;
use crate::domain::*;
use crate::error::{AppError, Result};
use rusqlite::{
    params, params_from_iter, types::Value, Connection, OptionalExtension, Transaction,
};

// ─────────────────────────── proyectos ───────────────────────────

fn proyecto(r: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: r.get(0)?,
        name: r.get(1)?,
        dest_root: r.get(2)?,
        mode: TriageMode::parse(&r.get::<_, String>(3)?),
        created_at: r.get(4)?,
    })
}

pub fn create_project(
    conn: &Connection,
    name: &str,
    dest_root: &str,
    mode: TriageMode,
) -> Result<Project> {
    conn.execute(
        "INSERT INTO projects (name, dest_root, mode, created_at, opened_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![name, dest_root, mode.as_str(), ahora_ms()],
    )?;
    project(conn, conn.last_insert_rowid())
}

pub fn project(conn: &Connection, id: i64) -> Result<Project> {
    conn.prepare_cached("SELECT id, name, dest_root, mode, created_at FROM projects WHERE id = ?1")?
        .query_row(params![id], proyecto)
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("proyecto {id}")))
}

pub fn projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut st = conn.prepare_cached(
        "SELECT id, name, dest_root, mode, created_at FROM projects ORDER BY opened_at DESC, id DESC",
    )?;
    let filas = st.query_map([], proyecto)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn last_project(conn: &Connection) -> Result<Option<Project>> {
    Ok(projects(conn)?.into_iter().next())
}

pub fn touch_project(conn: &Connection, id: i64, last_sample_id: Option<i64>) -> Result<()> {
    conn.execute(
        "UPDATE projects SET opened_at = ?2, last_sample_id = COALESCE(?3, last_sample_id)
         WHERE id = ?1",
        params![id, ahora_ms(), last_sample_id],
    )?;
    Ok(())
}

pub fn last_sample_of(conn: &Connection, project_id: i64) -> Result<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT last_sample_id FROM projects WHERE id = ?1",
            params![project_id],
            |r| r.get::<_, Option<i64>>(0),
        )
        .optional()?
        .flatten())
}

pub fn set_project_mode(conn: &Connection, id: i64, mode: TriageMode) -> Result<()> {
    conn.execute(
        "UPDATE projects SET mode = ?2 WHERE id = ?1",
        params![id, mode.as_str()],
    )?;
    Ok(())
}

pub fn delete_project(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
    Ok(())
}

// ─────────────────────────── destinos ───────────────────────────

const COLORES: [&str; 9] = [
    "dest-1", "dest-2", "dest-3", "dest-4", "dest-5", "dest-6", "dest-7", "dest-8", "dest-9",
];

fn destino(r: &rusqlite::Row<'_>) -> rusqlite::Result<Destination> {
    Ok(Destination {
        id: r.get(0)?,
        project_id: r.get(1)?,
        name: r.get(2)?,
        rel_path: r.get(3)?,
        hotkey: r.get(4)?,
        color: r.get(5)?,
        sort_order: r.get(6)?,
        count: r.get(7)?,
    })
}

pub fn destinations(conn: &Connection, project_id: i64) -> Result<Vec<Destination>> {
    let mut st = conn.prepare_cached(
        "SELECT id, project_id, name, rel_path, hotkey, color, sort_order, count
         FROM destinations WHERE project_id = ?1 ORDER BY sort_order, id",
    )?;
    let filas = st.query_map(params![project_id], destino)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn destination(conn: &Connection, id: i64) -> Result<Destination> {
    conn.prepare_cached(
        "SELECT id, project_id, name, rel_path, hotkey, color, sort_order, count
         FROM destinations WHERE id = ?1",
    )?
    .query_row(params![id], destino)
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("destino {id}")))
}

/// Crea un destino asignándole la primera tecla libre del 1 al 9.
pub fn create_destination(
    conn: &Connection,
    project_id: i64,
    name: &str,
    rel_path: &str,
) -> Result<Destination> {
    let existentes = destinations(conn, project_id)?;
    let ocupadas: Vec<String> = existentes.iter().filter_map(|d| d.hotkey.clone()).collect();
    let hotkey = (1..=9)
        .map(|n| n.to_string())
        .find(|k| !ocupadas.contains(k));
    let orden = existentes.len() as i64;
    let color = COLORES[existentes.len() % COLORES.len()];

    conn.execute(
        "INSERT INTO destinations (project_id, name, rel_path, hotkey, color, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![project_id, name, rel_path, hotkey, color, orden],
    )?;
    destination(conn, conn.last_insert_rowid())
}

pub fn rename_destination(conn: &Connection, id: i64, name: &str, rel_path: &str) -> Result<()> {
    conn.execute(
        "UPDATE destinations SET name = ?2, rel_path = ?3 WHERE id = ?1",
        params![id, name, rel_path],
    )?;
    Ok(())
}

pub fn delete_destination(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM destinations WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn bump_destination(tx: &Transaction<'_>, id: i64, delta: i64) -> Result<i64> {
    tx.execute(
        "UPDATE destinations SET count = max(0, count + ?2) WHERE id = ?1",
        params![id, delta],
    )?;
    Ok(tx.query_row(
        "SELECT count FROM destinations WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )?)
}

/// Recuenta desde la verdad (la tabla samples) en vez de fiarse del contador incremental.
pub fn recount_destinations(conn: &Connection, project_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE destinations SET count = (
            SELECT count(*) FROM samples s WHERE s.dest_id = destinations.id
         ) WHERE project_id = ?1",
        params![project_id],
    )?;
    Ok(())
}

// ─────────────────────────── journal ───────────────────────────

#[derive(Debug, Clone)]
pub struct AccionFila {
    pub id: i64,
    pub project_id: Option<i64>,
    pub sample_id: Option<i64>,
    pub dest_id: Option<i64>,
    pub kind: ActionKind,
    pub from_path: String,
    pub to_path: String,
    pub prev_status: String,
    pub prev_dest: Option<i64>,
    pub prev_current: Option<String>,
    pub batch_id: String,
}

fn accion(r: &rusqlite::Row<'_>) -> rusqlite::Result<AccionFila> {
    Ok(AccionFila {
        id: r.get(0)?,
        project_id: r.get(1)?,
        sample_id: r.get(2)?,
        dest_id: r.get(3)?,
        kind: ActionKind::parse(&r.get::<_, String>(4)?),
        from_path: r.get(5)?,
        to_path: r.get(6)?,
        prev_status: r.get(7)?,
        prev_dest: r.get(8)?,
        prev_current: r.get(9)?,
        batch_id: r.get(10)?,
    })
}

const COLUMNAS_ACCION: &str = "id, project_id, sample_id, dest_id, kind, from_path, to_path,
                               prev_status, prev_dest, prev_current, batch_id";

/// Escribe la intención ANTES de tocar el disco. Devuelve el id de la acción.
#[allow(clippy::too_many_arguments)]
pub fn begin_action(
    tx: &Transaction<'_>,
    project_id: Option<i64>,
    sample_id: i64,
    dest_id: Option<i64>,
    kind: ActionKind,
    from_path: &str,
    to_path: &str,
    prev_status: SampleStatus,
    prev_dest: Option<i64>,
    prev_current: Option<&str>,
    batch_id: &str,
) -> Result<i64> {
    tx.execute(
        "INSERT INTO actions (project_id, sample_id, dest_id, kind, from_path, to_path,
                              prev_status, prev_dest, prev_current, created_at, batch_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            project_id,
            sample_id,
            dest_id,
            kind.as_str(),
            from_path,
            to_path,
            prev_status.as_str(),
            prev_dest,
            prev_current,
            ahora_ms(),
            batch_id
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

pub fn finish_action(tx: &Transaction<'_>, id: i64) -> Result<()> {
    tx.execute(
        "UPDATE actions SET done_at = ?2 WHERE id = ?1",
        params![id, ahora_ms()],
    )?;
    Ok(())
}

pub fn drop_action(tx: &Transaction<'_>, id: i64) -> Result<()> {
    tx.execute("DELETE FROM actions WHERE id = ?1", params![id])?;
    Ok(())
}

/// Acciones que quedaron a medias (se escribió la intención pero no se cerró).
pub fn unfinished(conn: &Connection) -> Result<Vec<AccionFila>> {
    let sql = format!("SELECT {COLUMNAS_ACCION} FROM actions WHERE done_at IS NULL ORDER BY id");
    let mut st = conn.prepare(&sql)?;
    let filas = st.query_map([], accion)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// El último lote hecho y no deshecho: lo que se lleva `Ctrl+Z`.
pub fn last_batch(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT batch_id FROM actions
             WHERE done_at IS NOT NULL AND undone_at IS NULL
             ORDER BY done_at DESC, id DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

/// El último lote deshecho: lo que se lleva `Ctrl+Shift+Z`.
pub fn last_undone_batch(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT batch_id FROM actions
             WHERE undone_at IS NOT NULL
             ORDER BY undone_at DESC, id DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
}

pub fn batch(conn: &Connection, batch_id: &str) -> Result<Vec<AccionFila>> {
    let sql = format!("SELECT {COLUMNAS_ACCION} FROM actions WHERE batch_id = ?1 ORDER BY id DESC");
    let mut st = conn.prepare_cached(&sql)?;
    let filas = st.query_map(params![batch_id], accion)?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn mark_batch_undone(tx: &Transaction<'_>, batch_id: &str, undone: bool) -> Result<()> {
    if undone {
        tx.execute(
            "UPDATE actions SET undone_at = ?2 WHERE batch_id = ?1",
            params![batch_id, ahora_ms()],
        )?;
    } else {
        tx.execute(
            "UPDATE actions SET undone_at = NULL, done_at = ?2 WHERE batch_id = ?1",
            params![batch_id, ahora_ms()],
        )?;
    }
    Ok(())
}

pub fn set_sample_state(
    tx: &Transaction<'_>,
    sample_id: i64,
    status: SampleStatus,
    dest_id: Option<i64>,
    current_path: Option<&str>,
) -> Result<()> {
    tx.execute(
        "UPDATE samples SET status = ?2, dest_id = ?3, current_path = ?4, seen_at = ?5
         WHERE id = ?1",
        params![
            sample_id,
            status.as_str(),
            dest_id,
            current_path,
            ahora_ms()
        ],
    )?;
    Ok(())
}

pub struct EstadoSample {
    pub status: SampleStatus,
    pub dest_id: Option<i64>,
    pub current_path: Option<String>,
}

pub fn sample_state(conn: &Connection, sample_id: i64) -> Result<EstadoSample> {
    let (s, d, c): (String, Option<i64>, Option<String>) = conn
        .query_row(
            "SELECT status, dest_id, current_path FROM samples WHERE id = ?1",
            params![sample_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("sample {sample_id}")))?;
    Ok(EstadoSample {
        status: SampleStatus::parse(&s),
        dest_id: d,
        current_path: c,
    })
}

/// Cuenta cuántos samples han pasado ya por el triaje, para la barra de progreso de sesión.
pub fn session_progress(conn: &Connection, source_id: Option<i64>) -> Result<(i64, i64)> {
    let (cond, ps): (&str, Vec<Value>) = match source_id {
        Some(id) => (" WHERE source_id = ?", vec![Value::Integer(id)]),
        None => ("", vec![]),
    };
    let sql = format!("SELECT sum(status <> 'pending'), count(*) FROM samples{cond}");
    let (hechos, total): (Option<i64>, i64) =
        conn.query_row(&sql, params_from_iter(ps.iter()), |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    Ok((hechos.unwrap_or(0), total))
}
