//! Construcción del conjunto de evaluación de la Fase 8 (tarea 8.0.3 y 8.0.7).
//!
//! Pasa el extractor de nombres por toda la biblioteca para tener una referencia amplia y
//! gratis, y compara esa referencia con las correcciones del usuario para responder a la
//! pregunta del gate: **¿cuánto mienten los nombres de archivo?**

use crate::db::{labels, Db};
use crate::domain::{FieldCoverage, LabelReport, LabelSource, LabelStats, NOTAS};
use crate::error::Result;
use crate::music::{evaluacion, filename};
use std::time::Instant;

const LOTE: i64 = 2000;

/// Cuántas correcciones a mano se consideran suficientes para que los números signifiquen algo.
pub const OBJETIVO_ETIQUETAS: i64 = 200;

/// Pasa el extractor por todos los samples y guarda lo que salga como etiquetas `filename`.
///
/// Es idempotente: volver a ejecutarlo actualiza en vez de duplicar, así que se puede repetir
/// cada vez que mejore el extractor sin ensuciar nada.
pub fn extraer_de_nombres(db: &Db) -> Result<LabelReport> {
    let t0 = Instant::now();
    let mut informe = LabelReport::default();
    let mut desde = 0i64;

    loop {
        let lote: Vec<(i64, String)> = db.read(|conn| {
            let mut st = conn.prepare_cached(
                "SELECT id, rel_path FROM samples WHERE id > ?1 ORDER BY id LIMIT ?2",
            )?;
            let filas = st.query_map([desde, LOTE], |r| Ok((r.get(0)?, r.get(1)?)))?;
            Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
        })?;

        if lote.is_empty() {
            break;
        }
        desde = lote.last().map(|(id, _)| *id).unwrap_or(desde);

        db.write(|conn| {
            let tx = conn.transaction()?;
            for (id, rel_path) in &lote {
                let pistas = filename::leer(rel_path);
                informe.processed += 1;

                if let Some((kind, conf)) = pistas.kind {
                    labels::upsert(
                        &tx,
                        *id,
                        "kind",
                        kind.as_str(),
                        conf as f64,
                        LabelSource::Filename,
                    )?;
                    informe.kind += 1;
                }
                if let Some((bpm, conf)) = pistas.bpm {
                    labels::upsert(
                        &tx,
                        *id,
                        "bpm",
                        &format!("{bpm}"),
                        conf as f64,
                        LabelSource::Filename,
                    )?;
                    informe.bpm += 1;
                }
                if let Some((key, conf)) = pistas.key {
                    labels::upsert(
                        &tx,
                        *id,
                        "key",
                        &key.as_str(),
                        conf as f64,
                        LabelSource::Filename,
                    )?;
                    informe.key += 1;
                }
                if let Some((clase, conf)) = pistas.pitch {
                    let nota = NOTAS.get(clase as usize).copied().unwrap_or("C");
                    labels::upsert(&tx, *id, "pitch", nota, conf as f64, LabelSource::Filename)?;
                    informe.pitch += 1;
                }
            }
            tx.commit()?;
            Ok(())
        })?;
    }

    informe.millis = t0.elapsed().as_millis() as i64;
    Ok(informe)
}

/// La medida del gate: acuerdo entre la referencia débil y la verdad del usuario, campo a campo.
pub fn cobertura(db: &Db) -> Result<LabelStats> {
    let mut campos = Vec::new();

    for field in labels::CAMPOS {
        let (from_filename, from_user, only_user, pares) = db.read(|c| {
            Ok((
                labels::contar(c, field, LabelSource::Filename)?,
                labels::contar(c, field, LabelSource::User)?,
                labels::solo_usuario(c, field)?,
                labels::pares_para_evaluar(c, field)?,
            ))
        })?;

        let mut resumen = evaluacion::Resumen::default();
        for (_, debil, verdad) in &pares {
            resumen.anotar(evaluacion::comparar(field, verdad, debil));
        }

        campos.push(FieldCoverage {
            field: field.to_string(),
            from_filename,
            from_user,
            only_user,
            pairs: resumen.pares,
            exact: resumen.exactos,
            close: resumen.cercanos,
            wrong: resumen.fallos,
            accuracy: resumen.acierto(),
            mirex: resumen.mirex_medio(),
        });
    }

    let etiquetados = db.read(|c| {
        Ok(c.query_row(
            "SELECT count(DISTINCT sample_id) FROM labels WHERE source = 'user'",
            [],
            |r| r.get::<_, i64>(0),
        )?)
    })?;

    Ok(LabelStats {
        fields: campos,
        labeled_samples: etiquetados,
        target: OBJETIVO_ETIQUETAS,
    })
}
