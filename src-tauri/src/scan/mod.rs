//! Escaneo del disco e indexado. Todo el trabajo pesado ocurre aquí, nunca en el hilo del
//! comando IPC: quien llama recibe el progreso por un callback con throttle.

pub mod analyzer;
pub mod labeling;

use crate::db::{queries, Db};
use crate::domain::ScanProgress;
use crate::error::Result;
use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

const LOTE: usize = 1000;
/// Diez mensajes por segundo como mucho. Uno por archivo ahogaría el WebView.
const CADENCIA_PROGRESO: Duration = Duration::from_millis(100);

pub struct Emisor<F: FnMut(ScanProgress)> {
    f: F,
    ultimo: Instant,
    estado: ScanProgress,
}

impl<F: FnMut(ScanProgress)> Emisor<F> {
    pub fn nuevo(f: F) -> Self {
        Self {
            f,
            ultimo: Instant::now() - CADENCIA_PROGRESO,
            estado: ScanProgress {
                found: 0,
                indexed: 0,
                analyzed: 0,
                pending_analysis: 0,
                done: false,
            },
        }
    }
    pub fn estado_mut(&mut self) -> &mut ScanProgress {
        &mut self.estado
    }
    pub fn quiza(&mut self) {
        if self.ultimo.elapsed() >= CADENCIA_PROGRESO {
            self.ultimo = Instant::now();
            (self.f)(self.estado);
        }
    }
    pub fn forzar(&mut self) {
        self.ultimo = Instant::now();
        (self.f)(self.estado);
    }
}

/// Recorre la carpeta e indexa. La lista se puede usar antes de que esto termine: los datos
/// van entrando por lotes y el frontend refresca con el progreso.
pub fn escanear(
    db: &Db,
    source_id: i64,
    raiz: &Path,
    progreso: impl FnMut(ScanProgress),
) -> Result<i64> {
    let mut emisor = Emisor::nuevo(progreso);
    let mut lote: Vec<queries::EntradaEscaneo> = Vec::with_capacity(LOTE);
    let mut vistos: HashSet<String> = HashSet::with_capacity(1 << 14);
    let mut encontrados = 0i64;

    for entrada in jwalk::WalkDir::new(raiz).skip_hidden(true) {
        let Ok(e) = entrada else { continue };
        if !e.file_type().is_file() {
            continue;
        }
        let ruta = e.path();
        if !crate::codec::es_audio(&ruta) {
            continue;
        }
        // Siempre con `/`, también en Windows: ver `crate::paths`.
        let Some(rel_path) = crate::paths::relativa(raiz, &ruta) else {
            continue;
        };
        let (size, mtime) = match e.metadata() {
            Ok(m) => (
                m.len() as i64,
                m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0),
            ),
            Err(_) => continue,
        };

        vistos.insert(rel_path.clone());
        lote.push(queries::EntradaEscaneo {
            filename: ruta
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            ext: ruta
                .extension()
                .map(|s| s.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default(),
            rel_path,
            size,
            mtime,
        });
        encontrados += 1;

        if lote.len() >= LOTE {
            escribir_lote(db, source_id, &lote)?;
            lote.clear();
            let e = emisor.estado_mut();
            e.found = encontrados;
            e.indexed = encontrados;
            emisor.quiza();
        }
    }

    if !lote.is_empty() {
        escribir_lote(db, source_id, &lote)?;
    }

    let borrados = podar(db, source_id, &vistos)?;
    db.read(|c| queries::touch_source(c, source_id))?;

    let pendientes = db.read(queries::count_pending_analysis)?;
    let e = emisor.estado_mut();
    e.found = encontrados;
    e.indexed = encontrados;
    e.pending_analysis = pendientes;
    emisor.forzar();
    Ok(borrados)
}

fn escribir_lote(db: &Db, source_id: i64, lote: &[queries::EntradaEscaneo]) -> Result<()> {
    db.write(|conn| {
        let tx = conn.transaction()?;
        queries::upsert_batch(&tx, source_id, lote)?;
        tx.commit()?;
        Ok(())
    })
}

/// Quita del índice los archivos que ya no están en disco.
///
/// Solo toca los que siguen `pending` y sin `current_path`: los que hemos movido nosotros ya no
/// están en su ruta original a propósito, y borrarlos aquí sería perder el historial.
fn podar(db: &Db, source_id: i64, vistos: &HashSet<String>) -> Result<i64> {
    let candidatos: Vec<(i64, String)> = db.read(|conn| {
        let mut st = conn.prepare(
            "SELECT id, rel_path FROM samples
             WHERE source_id = ?1 AND status = 'pending' AND current_path IS NULL",
        )?;
        let filas = st.query_map([source_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(filas.collect::<rusqlite::Result<Vec<_>>>()?)
    })?;

    let fantasmas: Vec<i64> = candidatos
        .into_iter()
        .filter(|(_, rel)| !vistos.contains(rel))
        .map(|(id, _)| id)
        .collect();

    if fantasmas.is_empty() {
        return Ok(0);
    }
    db.write(|conn| {
        let tx = conn.transaction()?;
        {
            let mut st = tx.prepare_cached("DELETE FROM samples WHERE id = ?1")?;
            for id in &fantasmas {
                st.execute([id])?;
            }
        }
        tx.commit()?;
        Ok(())
    })?;
    Ok(fantasmas.len() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries;

    fn wav_de_prueba(dir: &Path, nombre: &str, muestras: usize) {
        // WAV mono 16 bits mínimo, escrito a mano para no depender de nada.
        let datos: Vec<u8> = (0..muestras)
            .flat_map(|i| ((i as i16).wrapping_mul(300)).to_le_bytes())
            .collect();
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36 + datos.len() as u32).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&44100u32.to_le_bytes());
        w.extend_from_slice(&88200u32.to_le_bytes());
        w.extend_from_slice(&2u16.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&(datos.len() as u32).to_le_bytes());
        w.extend_from_slice(&datos);
        std::fs::write(dir.join(nombre), w).unwrap();
    }

    #[test]
    fn escanea_indexa_y_no_reindexa_lo_que_no_cambia() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        std::fs::create_dir_all(lib.join("kicks")).unwrap();
        wav_de_prueba(&lib.join("kicks"), "k1.wav", 1000);
        wav_de_prueba(&lib.join("kicks"), "k2.wav", 1000);
        std::fs::write(lib.join("kicks/notas.txt"), "no soy audio").unwrap();

        let db = Db::open(&tmp.path().join("t.db")).unwrap();
        let sid = db
            .read(|c| queries::add_source(c, lib.to_str().unwrap()))
            .unwrap();

        escanear(&db, sid, &lib, |_| {}).unwrap();
        let stats = db.read(|c| queries::stats(c, Some(sid))).unwrap();
        assert_eq!(stats.total, 2, "el .txt no debe entrar en el índice");
        assert_eq!(stats.analyzed, 0, "el análisis va aparte del escaneo");

        // segundo escaneo: mismo resultado, sin duplicar
        escanear(&db, sid, &lib, |_| {}).unwrap();
        let stats = db.read(|c| queries::stats(c, Some(sid))).unwrap();
        assert_eq!(stats.total, 2);
    }

    #[test]
    fn poda_los_archivos_que_desaparecen_del_disco() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        std::fs::create_dir_all(&lib).unwrap();
        wav_de_prueba(&lib, "a.wav", 500);
        wav_de_prueba(&lib, "b.wav", 500);

        let db = Db::open(&tmp.path().join("t.db")).unwrap();
        let sid = db
            .read(|c| queries::add_source(c, lib.to_str().unwrap()))
            .unwrap();
        escanear(&db, sid, &lib, |_| {}).unwrap();
        assert_eq!(db.read(|c| queries::stats(c, Some(sid))).unwrap().total, 2);

        std::fs::remove_file(lib.join("b.wav")).unwrap();
        let borrados = escanear(&db, sid, &lib, |_| {}).unwrap();
        assert_eq!(borrados, 1);
        assert_eq!(db.read(|c| queries::stats(c, Some(sid))).unwrap().total, 1);
    }
}
