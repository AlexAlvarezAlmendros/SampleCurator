//! Operaciones de triaje: mover, rechazar, conservar, deshacer, rehacer y reparar.
//!
//! El orden es siempre el mismo y no se cambia:
//!   1. se escribe la intención en el journal y se confirma la transacción
//!   2. se toca el disco
//!   3. se cierra el journal y se actualiza el estado en otra transacción
//!
//! Si el proceso muere entre 1 y 3, al arrancar quedan acciones con `done_at IS NULL` y
//! `reparar` decide qué pasó mirando qué archivos existen. Ante la duda, siempre se conserva
//! el original: es preferible re-triar un sample que perderlo.

pub mod export;
pub mod mover;
pub mod trash;

use crate::db::{queries, triage, Db};
use crate::domain::*;
use crate::error::{AppError, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static CONTADOR: AtomicU64 = AtomicU64::new(1);

fn nuevo_lote() -> String {
    format!(
        "b{}-{}",
        crate::db::ahora_ms(),
        CONTADOR.fetch_add(1, Ordering::Relaxed)
    )
}

fn nombre_de(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "sample".into())
}

struct Plan {
    sample_id: i64,
    action_id: i64,
    desde: PathBuf,
    hasta: PathBuf,
}

/// Envía samples a un destino. Mueve o copia según el modo del proyecto.
pub fn enviar(db: &Db, project_id: i64, dest_id: i64, ids: &[i64]) -> Result<TriageResult> {
    if ids.is_empty() {
        return Err(AppError::InvalidInput("no hay samples que enviar".into()));
    }
    let (proyecto, destino) = db.read(|c| {
        Ok((
            triage::project(c, project_id)?,
            triage::destination(c, dest_id)?,
        ))
    })?;
    let dir = PathBuf::from(&proyecto.dest_root).join(&destino.rel_path);
    std::fs::create_dir_all(&dir).map_err(|e| {
        AppError::Io(format!(
            "no se pudo preparar la carpeta {}: {e}",
            dir.display()
        ))
    })?;

    let kind = match proyecto.mode {
        TriageMode::Move => ActionKind::Move,
        TriageMode::Copy => ActionKind::Copy,
    };
    let lote = nuevo_lote();

    // 1 · intención
    let planes = db.write(|conn| {
        let tx = conn.transaction()?;
        let mut reservadas: HashSet<PathBuf> = HashSet::new();
        let mut planes = Vec::with_capacity(ids.len());
        for &id in ids {
            let desde = queries::abs_path(&tx, id)?;
            let prev = triage::sample_state(&tx, id)?;
            let hasta = mover::ruta_libre(&dir, &nombre_de(&desde), &reservadas);
            reservadas.insert(hasta.clone());
            let action_id = triage::begin_action(
                &tx,
                Some(project_id),
                id,
                Some(dest_id),
                kind,
                &desde.to_string_lossy(),
                &hasta.to_string_lossy(),
                prev.status,
                prev.dest_id,
                prev.current_path.as_deref(),
                &lote,
            )?;
            planes.push(Plan {
                sample_id: id,
                action_id,
                desde,
                hasta,
            });
        }
        tx.commit()?;
        Ok(planes)
    })?;

    // 2 · disco
    let mut hechos = Vec::new();
    let mut fallidos = Vec::new();
    for p in &planes {
        let r = match kind {
            ActionKind::Copy => mover::copiar(&p.desde, &p.hasta),
            _ => mover::mover(&p.desde, &p.hasta),
        };
        match r {
            Ok(()) => hechos.push(p),
            Err(e) => {
                eprintln!(
                    "[fileops] {} → {}: {e}",
                    p.desde.display(),
                    p.hasta.display()
                );
                fallidos.push(p);
            }
        }
    }

    // 3 · cierre
    let contador = db.write(|conn| {
        let tx = conn.transaction()?;
        for p in &hechos {
            let (estado, current) = match kind {
                ActionKind::Copy => (SampleStatus::Kept, None),
                _ => (
                    SampleStatus::Moved,
                    Some(p.hasta.to_string_lossy().to_string()),
                ),
            };
            triage::set_sample_state(&tx, p.sample_id, estado, Some(dest_id), current.as_deref())?;
            triage::finish_action(&tx, p.action_id)?;
        }
        for p in &fallidos {
            triage::drop_action(&tx, p.action_id)?;
        }
        let c = triage::bump_destination(&tx, dest_id, hechos.len() as i64)?;
        tx.commit()?;
        Ok(c)
    })?;

    Ok(TriageResult {
        batch_id: lote,
        affected: hechos.iter().map(|p| p.sample_id).collect(),
        destination_id: Some(dest_id),
        destination_count: Some(contador),
        kind,
    })
}

/// Rechaza samples. En modo mover van a la papelera gestionada; en modo copiar solo se
/// marcan, porque quien copia quiere su carpeta de origen intacta.
pub fn rechazar(db: &Db, project_id: i64, ids: &[i64]) -> Result<TriageResult> {
    if ids.is_empty() {
        return Err(AppError::InvalidInput("no hay samples que rechazar".into()));
    }
    let proyecto = db.read(|c| triage::project(c, project_id))?;
    let raiz = PathBuf::from(&proyecto.dest_root);
    let solo_marcar = proyecto.mode == TriageMode::Copy;
    let dir = trash::asegurar(&raiz)?;
    let lote = nuevo_lote();

    let planes = db.write(|conn| {
        let tx = conn.transaction()?;
        let mut reservadas: HashSet<PathBuf> = HashSet::new();
        let mut planes = Vec::with_capacity(ids.len());
        for &id in ids {
            let desde = queries::abs_path(&tx, id)?;
            let prev = triage::sample_state(&tx, id)?;
            let hasta = if solo_marcar {
                desde.clone()
            } else {
                let h = mover::ruta_libre(&dir, &nombre_de(&desde), &reservadas);
                reservadas.insert(h.clone());
                h
            };
            let action_id = triage::begin_action(
                &tx,
                Some(project_id),
                id,
                None,
                ActionKind::Reject,
                &desde.to_string_lossy(),
                &hasta.to_string_lossy(),
                prev.status,
                prev.dest_id,
                prev.current_path.as_deref(),
                &lote,
            )?;
            planes.push(Plan {
                sample_id: id,
                action_id,
                desde,
                hasta,
            });
        }
        tx.commit()?;
        Ok(planes)
    })?;

    let mut hechos = Vec::new();
    let mut fallidos = Vec::new();
    for p in &planes {
        if solo_marcar {
            hechos.push(p);
            continue;
        }
        match mover::mover(&p.desde, &p.hasta) {
            Ok(()) => {
                let _ = trash::anotar(&raiz, p.sample_id, &p.desde, &p.hasta);
                hechos.push(p);
            }
            Err(e) => {
                eprintln!("[fileops] rechazo {}: {e}", p.desde.display());
                fallidos.push(p);
            }
        }
    }

    db.write(|conn| {
        let tx = conn.transaction()?;
        for p in &hechos {
            let current = if solo_marcar {
                None
            } else {
                Some(p.hasta.to_string_lossy().to_string())
            };
            triage::set_sample_state(
                &tx,
                p.sample_id,
                SampleStatus::Rejected,
                None,
                current.as_deref(),
            )?;
            triage::finish_action(&tx, p.action_id)?;
        }
        for p in &fallidos {
            triage::drop_action(&tx, p.action_id)?;
        }
        tx.commit()?;
        Ok(())
    })?;

    Ok(TriageResult {
        batch_id: lote,
        affected: hechos.iter().map(|p| p.sample_id).collect(),
        destination_id: None,
        destination_count: None,
        kind: ActionKind::Reject,
    })
}

/// Conservar en su sitio: no toca el disco, pero deja rastro para poder deshacerlo.
pub fn conservar(db: &Db, project_id: i64, ids: &[i64]) -> Result<TriageResult> {
    if ids.is_empty() {
        return Err(AppError::InvalidInput(
            "no hay samples que conservar".into(),
        ));
    }
    let lote = nuevo_lote();
    db.write(|conn| {
        let tx = conn.transaction()?;
        for &id in ids {
            let ruta = queries::abs_path(&tx, id)?;
            let prev = triage::sample_state(&tx, id)?;
            let aid = triage::begin_action(
                &tx,
                Some(project_id),
                id,
                None,
                ActionKind::Keep,
                &ruta.to_string_lossy(),
                &ruta.to_string_lossy(),
                prev.status,
                prev.dest_id,
                prev.current_path.as_deref(),
                &lote,
            )?;
            triage::set_sample_state(
                &tx,
                id,
                SampleStatus::Kept,
                None,
                prev.current_path.as_deref(),
            )?;
            triage::finish_action(&tx, aid)?;
        }
        tx.commit()?;
        Ok(())
    })?;
    Ok(TriageResult {
        batch_id: lote,
        affected: ids.to_vec(),
        destination_id: None,
        destination_count: None,
        kind: ActionKind::Keep,
    })
}

/// Renombra un sample. Queda en el journal como cualquier otra operación, así que `Ctrl+Z`
/// también deshace un renombrado.
pub fn renombrar(db: &Db, project_id: Option<i64>, sample_id: i64, nombre: &str) -> Result<String> {
    let limpio = nombre.trim();
    if limpio.is_empty() || limpio.contains('/') || limpio.contains('\\') || limpio.contains("..") {
        return Err(AppError::InvalidInput(
            "el nombre no puede estar vacío ni contener barras".into(),
        ));
    }
    let desde = db.read(|c| queries::abs_path(c, sample_id))?;
    let Some(dir) = desde.parent() else {
        return Err(AppError::PathUnavailable(desde));
    };
    let hasta = dir.join(limpio);
    if hasta == desde {
        return Ok(limpio.to_string());
    }
    if hasta.exists() {
        return Err(AppError::Unsafe(format!(
            "ya hay un archivo llamado {limpio} en esa carpeta"
        )));
    }

    let lote = nuevo_lote();
    let action_id = db.write(|conn| {
        let tx = conn.transaction()?;
        let prev = triage::sample_state(&tx, sample_id)?;
        let id = triage::begin_action(
            &tx,
            project_id,
            sample_id,
            prev.dest_id,
            ActionKind::Rename,
            &desde.to_string_lossy(),
            &hasta.to_string_lossy(),
            prev.status,
            prev.dest_id,
            prev.current_path.as_deref(),
            &lote,
        )?;
        tx.commit()?;
        Ok(id)
    })?;

    if let Err(e) = std::fs::rename(&desde, &hasta) {
        db.write(|conn| {
            let tx = conn.transaction()?;
            triage::drop_action(&tx, action_id)?;
            tx.commit()?;
            Ok(())
        })?;
        return Err(AppError::Io(format!("no se pudo renombrar: {e}")));
    }

    db.write(|conn| {
        let tx = conn.transaction()?;
        aplicar_nombre(&tx, sample_id, &hasta)?;
        triage::finish_action(&tx, action_id)?;
        tx.commit()?;
        Ok(())
    })?;
    Ok(limpio.to_string())
}

/// Deja el nombre y la ruta relativa del sample acordes al archivo que hay en disco.
fn aplicar_nombre(tx: &rusqlite::Transaction<'_>, sample_id: i64, ruta: &Path) -> Result<()> {
    let filename = nombre_de(ruta);
    let raiz: String = tx.query_row(
        "SELECT so.path FROM samples s JOIN sources so ON so.id = s.source_id WHERE s.id = ?1",
        [sample_id],
        |r| r.get(0),
    )?;
    match crate::paths::relativa(Path::new(&raiz), ruta) {
        Some(rel) => {
            tx.execute(
                "UPDATE samples SET filename = ?2, rel_path = ?3, current_path = NULL WHERE id = ?1",
                rusqlite::params![sample_id, filename, rel],
            )?;
        }
        None => {
            tx.execute(
                "UPDATE samples SET filename = ?2, current_path = ?3 WHERE id = ?1",
                rusqlite::params![sample_id, filename, ruta.to_string_lossy()],
            )?;
        }
    }
    Ok(())
}

// ─────────────────────────── deshacer y rehacer ───────────────────────────

pub fn deshacer(db: &Db) -> Result<UndoResult> {
    let lote = db
        .read(triage::last_batch)?
        .ok_or(AppError::NothingToUndo)?;
    let acciones = db.read(|c| triage::batch(c, &lote))?;
    if acciones.is_empty() {
        return Err(AppError::NothingToUndo);
    }

    let mut restaurados = Vec::new();
    for a in &acciones {
        let desde = PathBuf::from(&a.to_path);
        let hasta = PathBuf::from(&a.from_path);
        let r = match a.kind {
            ActionKind::Keep => Ok(()),
            ActionKind::Copy => {
                // Se borra la copia que hicimos nosotros, nunca el original del usuario.
                if desde.exists() && hasta.exists() && desde != hasta {
                    std::fs::remove_file(&desde).map_err(AppError::from)
                } else {
                    Ok(())
                }
            }
            _ => {
                if desde == hasta || !desde.exists() {
                    Ok(()) // rechazo en modo copiar, o el archivo ya está en su sitio
                } else {
                    mover::mover(&desde, &hasta)
                }
            }
        };
        match r {
            Ok(()) => restaurados.push(a),
            Err(e) => eprintln!("[fileops] no se pudo deshacer {}: {e}", a.from_path),
        }
    }

    let mut contador = None;
    db.write(|conn| {
        let tx = conn.transaction()?;
        for a in &restaurados {
            if let Some(sid) = a.sample_id {
                if a.kind == ActionKind::Rename {
                    // Un renombrado deshecho tiene que devolver el nombre, no solo el archivo.
                    aplicar_nombre(&tx, sid, Path::new(&a.from_path))?;
                }
                triage::set_sample_state(
                    &tx,
                    sid,
                    SampleStatus::parse(&a.prev_status),
                    a.prev_dest,
                    a.prev_current.as_deref(),
                )?;
            }
            if let Some(d) = a.dest_id {
                contador = Some(triage::bump_destination(&tx, d, -1)?);
            }
        }
        triage::mark_batch_undone(&tx, &lote, true)?;
        tx.commit()?;
        Ok(())
    })?;

    Ok(UndoResult {
        batch_id: lote,
        restored: restaurados.iter().filter_map(|a| a.sample_id).collect(),
        focus_sample_id: restaurados.last().and_then(|a| a.sample_id),
        kind: acciones.first().map(|a| a.kind).unwrap_or(ActionKind::Move),
        destination_id: acciones.first().and_then(|a| a.dest_id),
        destination_count: contador,
    })
}

pub fn rehacer(db: &Db) -> Result<UndoResult> {
    let lote = db
        .read(triage::last_undone_batch)?
        .ok_or(AppError::NothingToUndo)?;
    let acciones = db.read(|c| triage::batch(c, &lote))?;
    if acciones.is_empty() {
        return Err(AppError::NothingToUndo);
    }

    let mut rehechos = Vec::new();
    for a in &acciones {
        let desde = PathBuf::from(&a.from_path);
        let hasta = PathBuf::from(&a.to_path);
        let r = match a.kind {
            ActionKind::Keep => Ok(()),
            ActionKind::Copy => {
                if hasta.exists() {
                    Ok(())
                } else {
                    mover::copiar(&desde, &hasta)
                }
            }
            _ => {
                if desde == hasta || hasta.exists() {
                    Ok(())
                } else {
                    mover::mover(&desde, &hasta)
                }
            }
        };
        match r {
            Ok(()) => rehechos.push(a),
            Err(e) => eprintln!("[fileops] no se pudo rehacer {}: {e}", a.to_path),
        }
    }

    let mut contador = None;
    db.write(|conn| {
        let tx = conn.transaction()?;
        for a in &rehechos {
            if let Some(sid) = a.sample_id {
                if a.kind == ActionKind::Rename {
                    aplicar_nombre(&tx, sid, Path::new(&a.to_path))?;
                }
                let (estado, current) = estado_tras(a);
                triage::set_sample_state(&tx, sid, estado, a.dest_id, current.as_deref())?;
            }
            if let Some(d) = a.dest_id {
                contador = Some(triage::bump_destination(&tx, d, 1)?);
            }
        }
        triage::mark_batch_undone(&tx, &lote, false)?;
        tx.commit()?;
        Ok(())
    })?;

    Ok(UndoResult {
        batch_id: lote,
        restored: rehechos.iter().filter_map(|a| a.sample_id).collect(),
        focus_sample_id: rehechos.last().and_then(|a| a.sample_id),
        kind: acciones.first().map(|a| a.kind).unwrap_or(ActionKind::Move),
        destination_id: acciones.first().and_then(|a| a.dest_id),
        destination_count: contador,
    })
}

/// Estado en el que queda un sample cuando su acción SÍ se ha llevado a cabo.
fn estado_tras(a: &triage::AccionFila) -> (SampleStatus, Option<String>) {
    match a.kind {
        // Renombrar no decide nada: el sample sigue como estaba en el triaje.
        ActionKind::Rename => (SampleStatus::parse(&a.prev_status), a.prev_current.clone()),
        ActionKind::Copy => (SampleStatus::Kept, a.prev_current.clone()),
        ActionKind::Keep => (SampleStatus::Kept, a.prev_current.clone()),
        ActionKind::Reject => {
            if a.from_path == a.to_path {
                (SampleStatus::Rejected, a.prev_current.clone())
            } else {
                (SampleStatus::Rejected, Some(a.to_path.clone()))
            }
        }
        _ => (SampleStatus::Moved, Some(a.to_path.clone())),
    }
}

// ─────────────────────────── papelera ───────────────────────────

/// Lo que hay en la papelera, cruzado con el índice para poder enseñarlo con su duración.
pub fn papelera(db: &Db, project_id: i64) -> Result<Vec<TrashEntry>> {
    let proyecto = db.read(|c| triage::project(c, project_id))?;
    let raiz = PathBuf::from(&proyecto.dest_root);
    let anotaciones = trash::entradas(&raiz);

    let mut salida = Vec::with_capacity(anotaciones.len());
    for a in anotaciones {
        let ruta = PathBuf::from(&a.to);
        let size = std::fs::metadata(&ruta)
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        let filename = nombre_de(&ruta);

        // El sample puede no estar ya en el índice: si se quitó su carpeta de origen, la fila
        // desapareció pero el archivo sigue en la papelera y hay que poder devolverlo.
        let del_indice = if a.sample_id > 0 {
            db.read(|c| queries::detail(c, a.sample_id)).ok()
        } else {
            None
        };

        salida.push(TrashEntry {
            sample_id: del_indice.as_ref().map(|d| d.row.id),
            filename,
            trash_path: a.to.clone(),
            original_path: a.from.clone(),
            at: a.at,
            size,
            duration_ms: del_indice.as_ref().and_then(|d| d.row.duration_ms),
            in_index: del_indice.is_some(),
        });
    }
    Ok(salida)
}

/// Devuelve un archivo de la papelera a su ruta original.
///
/// Mismo orden que cualquier otra operación: journal, disco, cierre. Y si en su sitio hay ya
/// otro archivo, no se sobrescribe: se restaura al lado con sufijo.
pub fn restaurar(db: &Db, project_id: i64, trash_path: &str) -> Result<i64> {
    let proyecto = db.read(|c| triage::project(c, project_id))?;
    let raiz = PathBuf::from(&proyecto.dest_root);

    let anotacion = trash::entradas(&raiz)
        .into_iter()
        .find(|a| a.to == trash_path)
        .ok_or_else(|| AppError::NotFound("esa entrada ya no está en la papelera".into()))?;

    if anotacion.from.is_empty() {
        return Err(AppError::Unsafe(
            "ese archivo no tiene anotada su ruta original, así que no se sabe dónde devolverlo"
                .into(),
        ));
    }

    let desde = PathBuf::from(&anotacion.to);
    let original = PathBuf::from(&anotacion.from);
    let Some(dir) = original.parent() else {
        return Err(AppError::PathUnavailable(original));
    };
    std::fs::create_dir_all(dir)?;
    let hasta = mover::ruta_libre(dir, &nombre_de(&original), &HashSet::new());

    let lote = nuevo_lote();
    let sample_id = if anotacion.sample_id > 0 {
        Some(anotacion.sample_id)
    } else {
        None
    };

    let action_id = match sample_id {
        Some(sid) => db.write(|conn| {
            let tx = conn.transaction()?;
            let prev = triage::sample_state(&tx, sid)?;
            let id = triage::begin_action(
                &tx,
                Some(project_id),
                sid,
                None,
                ActionKind::Move,
                &desde.to_string_lossy(),
                &hasta.to_string_lossy(),
                prev.status,
                prev.dest_id,
                prev.current_path.as_deref(),
                &lote,
            )?;
            tx.commit()?;
            Ok(Some(id))
        })?,
        None => None,
    };

    if let Err(e) = mover::mover(&desde, &hasta) {
        if let (Some(id), true) = (action_id, action_id.is_some()) {
            db.write(|conn| {
                let tx = conn.transaction()?;
                triage::drop_action(&tx, id)?;
                tx.commit()?;
                Ok(())
            })?;
        }
        return Err(e);
    }

    if let (Some(sid), Some(aid)) = (sample_id, action_id) {
        db.write(|conn| {
            let tx = conn.transaction()?;
            // Vuelve a la cola: restaurar es deshacer una decisión, no tomar otra.
            let current = if hasta == original {
                None
            } else {
                Some(hasta.to_string_lossy().to_string())
            };
            triage::set_sample_state(&tx, sid, SampleStatus::Pending, None, current.as_deref())?;
            triage::finish_action(&tx, aid)?;
            tx.commit()?;
            Ok(())
        })?;
    }

    trash::olvidar(&raiz, trash_path)?;
    Ok(sample_id.unwrap_or(0))
}

// ─────────────────────────── reparación al arrancar ───────────────────────────

/// Cierra las acciones que quedaron a medias. Ante la duda, conserva el original.
pub fn reparar(db: &Db) -> Result<i64> {
    let pendientes = db.read(triage::unfinished)?;
    if pendientes.is_empty() {
        return Ok(0);
    }
    let mut arregladas = 0i64;

    for a in &pendientes {
        let desde = PathBuf::from(&a.from_path);
        let hasta = PathBuf::from(&a.to_path);
        let origen_existe = desde.exists();
        let destino_existe = hasta.exists();
        let sin_disco = a.kind == ActionKind::Keep || desde == hasta;

        enum Decision {
            Completar,
            Descartar,
        }

        // Se completa cuando la acción no toca el disco, o cuando el archivo ya está en su
        // destino y no queda nada en el origen: la operación llegó a hacerse entera.
        let decision = if sin_disco || (destino_existe && !origen_existe) {
            Decision::Completar
        } else if origen_existe && !destino_existe {
            Decision::Descartar // nunca llegó a pasar nada
        } else if origen_existe && destino_existe {
            // Copia a medias de un movimiento entre dispositivos: no se puede saber si el
            // destino está completo. Se borra la copia dudosa y se conserva el original.
            if a.kind == ActionKind::Copy {
                Decision::Completar
            } else {
                let _ = std::fs::remove_file(&hasta);
                Decision::Descartar
            }
        } else {
            eprintln!(
                "[fileops] la acción {} no tiene ni origen ni destino en disco; se descarta",
                a.id
            );
            Decision::Descartar
        };

        db.write(|conn| {
            let tx = conn.transaction()?;
            match decision {
                Decision::Completar => {
                    if let Some(sid) = a.sample_id {
                        let (estado, current) = estado_tras(a);
                        triage::set_sample_state(&tx, sid, estado, a.dest_id, current.as_deref())?;
                    }
                    triage::finish_action(&tx, a.id)?;
                }
                Decision::Descartar => triage::drop_action(&tx, a.id)?,
            }
            tx.commit()?;
            Ok(())
        })?;
        arregladas += 1;
    }

    // Los contadores incrementales pueden haber quedado desfasados: se recuentan desde la
    // verdad, que es la tabla samples.
    let proyectos = db.read(triage::projects)?;
    for p in proyectos {
        db.read(|c| triage::recount_destinations(c, p.id))?;
    }
    Ok(arregladas)
}
