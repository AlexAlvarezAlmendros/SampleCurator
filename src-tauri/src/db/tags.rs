//! Etiquetas libres y notas.
//!
//! Las tablas existían desde la migración 001 y no las usaba nadie. Aquí entran en juego,
//! y con una regla: **esto vive en el índice, nunca dentro del archivo de audio**. Escribir
//! etiquetas en el `.wav` significaría reescribir archivos del usuario, y los DAW mayormente
//! las ignoran en samples.

use crate::error::Result;
use rusqlite::{params, Connection, OptionalExtension};

/// Normaliza una etiqueta: sin espacios sobrantes y en minúsculas, para que «808», « 808 » y
/// «808 » sean la misma y no acabemos con tres etiquetas que parecen una.
pub fn normalizar(nombre: &str) -> String {
    nombre.trim().to_lowercase()
}

pub fn id_de_etiqueta(conn: &Connection, nombre: &str) -> Result<i64> {
    let limpio = normalizar(nombre);
    conn.execute(
        "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
        params![limpio],
    )?;
    Ok(conn.query_row(
        "SELECT id FROM tags WHERE name = ?1",
        params![limpio],
        |r| r.get(0),
    )?)
}

pub fn poner(conn: &Connection, sample_id: i64, nombre: &str) -> Result<()> {
    let tag_id = id_de_etiqueta(conn, nombre)?;
    conn.execute(
        "INSERT INTO sample_tags (sample_id, tag_id) VALUES (?1, ?2)
         ON CONFLICT DO NOTHING",
        params![sample_id, tag_id],
    )?;
    Ok(())
}

pub fn quitar(conn: &Connection, sample_id: i64, nombre: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM sample_tags WHERE sample_id = ?1
         AND tag_id = (SELECT id FROM tags WHERE name = ?2)",
        params![sample_id, normalizar(nombre)],
    )?;
    // Una etiqueta que ya no usa nadie no tiene por qué seguir en la lista.
    conn.execute(
        "DELETE FROM tags WHERE NOT EXISTS
           (SELECT 1 FROM sample_tags st WHERE st.tag_id = tags.id)",
        [],
    )?;
    Ok(())
}

pub fn de_sample(conn: &Connection, sample_id: i64) -> Result<Vec<String>> {
    let mut st = conn.prepare_cached(
        "SELECT t.name FROM tags t
         JOIN sample_tags st ON st.tag_id = t.id
         WHERE st.sample_id = ?1 ORDER BY t.name",
    )?;
    let filas = st.query_map(params![sample_id], |r| r.get::<_, String>(0))?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Todas las etiquetas con cuántos samples las llevan, para el autocompletado y el filtro.
pub fn todas(conn: &Connection) -> Result<Vec<(String, i64)>> {
    let mut st = conn.prepare_cached(
        "SELECT t.name, count(st.sample_id) FROM tags t
         LEFT JOIN sample_tags st ON st.tag_id = t.id
         GROUP BY t.id ORDER BY count(st.sample_id) DESC, t.name",
    )?;
    let filas = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn set_notas(conn: &Connection, sample_id: i64, notas: &str) -> Result<()> {
    let limpio = notas.trim();
    conn.execute(
        "UPDATE samples SET notes = ?2 WHERE id = ?1",
        params![
            sample_id,
            if limpio.is_empty() {
                None
            } else {
                Some(limpio)
            }
        ],
    )?;
    Ok(())
}

pub fn notas(conn: &Connection, sample_id: i64) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT notes FROM samples WHERE id = ?1",
            params![sample_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn db_con_sample() -> (tempfile::TempDir, Db, i64) {
        let tmp = tempfile::tempdir().unwrap();
        let db = Db::open(&tmp.path().join("t.db")).unwrap();
        let id = db
            .read(|c| {
                c.execute("INSERT INTO sources (path, added_at) VALUES ('/x', 0)", [])?;
                c.execute(
                    "INSERT INTO samples (source_id, rel_path, filename, ext, size, mtime)
                     VALUES (1, 'a.wav', 'a.wav', 'wav', 1, 1)",
                    [],
                )?;
                Ok(c.last_insert_rowid())
            })
            .unwrap();
        (tmp, db, id)
    }

    #[test]
    fn las_etiquetas_se_normalizan_para_no_duplicarse() {
        let (_t, db, id) = db_con_sample();
        db.read(|c| {
            poner(c, id, "  808 ")?;
            poner(c, id, "808")?;
            poner(c, id, "GRAVE")?;
            Ok(())
        })
        .unwrap();
        let etiquetas = db.read(|c| de_sample(c, id)).unwrap();
        assert_eq!(
            etiquetas,
            vec!["808", "grave"],
            "«  808 » y «808» son la misma"
        );
    }

    #[test]
    fn quitar_una_etiqueta_la_borra_del_catalogo_si_no_la_usa_nadie() {
        let (_t, db, id) = db_con_sample();
        db.read(|c| poner(c, id, "efimera")).unwrap();
        assert_eq!(db.read(todas).unwrap().len(), 1);
        db.read(|c| quitar(c, id, "efimera")).unwrap();
        assert!(
            db.read(todas).unwrap().is_empty(),
            "sin usos, fuera del catálogo"
        );
    }

    #[test]
    fn las_notas_vacias_se_guardan_como_nada_y_no_como_cadena_vacia() {
        let (_t, db, id) = db_con_sample();
        db.read(|c| set_notas(c, id, "  suena bien con el pack de Vengeance  "))
            .unwrap();
        assert_eq!(
            db.read(|c| notas(c, id)).unwrap().as_deref(),
            Some("suena bien con el pack de Vengeance")
        );
        db.read(|c| set_notas(c, id, "   ")).unwrap();
        assert_eq!(db.read(|c| notas(c, id)).unwrap(), None);
    }

    #[test]
    fn el_catalogo_ordena_por_uso() {
        let (_t, db, id) = db_con_sample();
        let id2 = db
            .read(|c| {
                c.execute(
                    "INSERT INTO samples (source_id, rel_path, filename, ext, size, mtime)
                     VALUES (1, 'b.wav', 'b.wav', 'wav', 1, 1)",
                    [],
                )?;
                Ok(c.last_insert_rowid())
            })
            .unwrap();
        db.read(|c| {
            poner(c, id, "comun")?;
            poner(c, id2, "comun")?;
            poner(c, id, "rara")?;
            Ok(())
        })
        .unwrap();
        let todas = db.read(todas).unwrap();
        assert_eq!(todas[0], ("comun".into(), 2));
        assert_eq!(todas[1], ("rara".into(), 1));
    }
}
