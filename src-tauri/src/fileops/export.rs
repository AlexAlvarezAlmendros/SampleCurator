//! Copia de seguridad de las decisiones del usuario.
//!
//! El índice SQLite es una caché reconstruible, pero las DECISIONES no: qué se conservó, a qué
//! destino fue cada sample y cómo se valoró. Eso se vuelca a `<destino>/library.json` para que
//! sobreviva a perder el índice, a reinstalar o a cambiar de equipo.

use crate::db::{queries, triage, Db};
use crate::error::Result;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct Decision {
    rel_path: String,
    filename: String,
    status: String,
    destination: Option<String>,
    rating: i64,
}

#[derive(Serialize)]
struct DestinoExportado {
    name: String,
    rel_path: String,
    hotkey: Option<String>,
    count: i64,
}

#[derive(Serialize)]
struct Copia {
    version: u32,
    exported_at: i64,
    project: String,
    dest_root: String,
    mode: String,
    destinations: Vec<DestinoExportado>,
    decisions: Vec<Decision>,
}

pub fn exportar(db: &Db, project_id: i64) -> Result<PathBuf> {
    let proyecto = db.read(|c| triage::project(c, project_id))?;
    let destinos = db.read(|c| triage::destinations(c, project_id))?;

    let decisiones: Vec<Decision> = db.read(|conn| {
        let mut st = conn.prepare(
            "SELECT s.rel_path, s.filename, s.status, d.name, s.rating
             FROM samples s LEFT JOIN destinations d ON d.id = s.dest_id
             WHERE s.status <> 'pending'
             ORDER BY s.rel_path",
        )?;
        let filas = st.query_map([], |r| {
            Ok(Decision {
                rel_path: r.get(0)?,
                filename: r.get(1)?,
                status: r.get(2)?,
                destination: r.get(3)?,
                rating: r.get(4)?,
            })
        })?;
        Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
    })?;

    let copia = Copia {
        version: 1,
        exported_at: crate::db::ahora_ms(),
        project: proyecto.name.clone(),
        dest_root: proyecto.dest_root.clone(),
        mode: proyecto.mode.as_str().to_string(),
        destinations: destinos
            .into_iter()
            .map(|d| DestinoExportado {
                name: d.name,
                rel_path: d.rel_path,
                hotkey: d.hotkey,
                count: d.count,
            })
            .collect(),
        decisions: decisiones,
    };

    let destino = PathBuf::from(&proyecto.dest_root).join("library.json");
    let json = serde_json::to_string_pretty(&copia)
        .map_err(|e| crate::error::AppError::Io(format!("no se pudo serializar: {e}")))?;

    // Escritura atómica: primero a un temporal y luego rename. Si se corta la luz a mitad,
    // la copia anterior sigue entera en vez de quedar truncada.
    let temporal = destino.with_extension("json.tmp");
    std::fs::write(&temporal, json)?;
    std::fs::rename(&temporal, &destino)?;

    // Aprovechamos para dejar los contadores cuadrados con la realidad.
    db.read(|c| triage::recount_destinations(c, project_id))?;
    let _ = queries::count_pending_analysis;
    Ok(destino)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TriageMode;

    #[test]
    fn escribe_un_json_valido_con_las_decisiones() {
        let tmp = tempfile::tempdir().unwrap();
        let raiz = tmp.path().join("libreria");
        std::fs::create_dir_all(&raiz).unwrap();
        let db = Db::open(&tmp.path().join("t.db")).unwrap();

        let p = db
            .read(|c| triage::create_project(c, "s", raiz.to_str().unwrap(), TriageMode::Move))
            .unwrap();
        db.read(|c| triage::create_destination(c, p.id, "Kicks", "Kicks"))
            .unwrap();

        let ruta = exportar(&db, p.id).unwrap();
        assert!(ruta.exists());
        let texto = std::fs::read_to_string(&ruta).unwrap();
        let v: serde_json::Value = serde_json::from_str(&texto).unwrap();
        assert_eq!(v["version"], 1);
        assert_eq!(v["destinations"][0]["name"], "Kicks");
        assert_eq!(v["destinations"][0]["hotkey"], "1");
        // y no queda ningún temporal por medio
        assert!(!raiz.join("library.json.tmp").exists());
    }
}
