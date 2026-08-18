//! Escaneo de disco, indexado en SQLite y cálculo de picos. Todo lo que la app hará en `scan/`.

use anyhow::Result;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const EXTENSIONES: [&str; 8] = ["wav", "aiff", "aif", "flac", "mp3", "ogg", "m4a", "aac"];

pub fn es_audio(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            EXTENSIONES.contains(&e.as_str())
        })
        .unwrap_or(false)
}

pub struct Entrada {
    pub ruta: PathBuf,
    pub size: u64,
    pub mtime: i64,
}

/// Recorrido paralelo del árbol. Devuelve las entradas y los segundos que ha tardado.
pub fn recorrer(raiz: &Path) -> (Vec<Entrada>, f64) {
    let t0 = Instant::now();
    let mut out = Vec::with_capacity(1 << 16);
    for entrada in jwalk::WalkDir::new(raiz).skip_hidden(false) {
        let Ok(e) = entrada else { continue };
        if !e.file_type().is_file() {
            continue;
        }
        let ruta = e.path();
        if !es_audio(&ruta) {
            continue;
        }
        let (size, mtime) = match e.metadata() {
            Ok(m) => (
                m.len(),
                m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0),
            ),
            Err(_) => (0, 0),
        };
        out.push(Entrada { ruta, size, mtime });
    }
    (out, t0.elapsed().as_secs_f64())
}

pub fn listar_audio(raiz: &Path) -> Vec<PathBuf> {
    recorrer(raiz).0.into_iter().map(|e| e.ruta).collect()
}

/// Inserta en SQLite por lotes de 1.000 dentro de transacción, como hará la app.
pub fn indexar(entradas: &[Entrada], db: &Path) -> Result<f64> {
    let _ = std::fs::remove_file(db);
    let mut conn = rusqlite::Connection::open(db)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous  = NORMAL;
         CREATE TABLE samples (
            id       INTEGER PRIMARY KEY,
            rel_path TEXT NOT NULL,
            filename TEXT NOT NULL,
            ext      TEXT NOT NULL,
            size     INTEGER NOT NULL,
            mtime    INTEGER NOT NULL
         );",
    )?;

    let t0 = Instant::now();
    for lote in entradas.chunks(1000) {
        let tx = conn.transaction()?;
        {
            let mut st = tx.prepare_cached(
                "INSERT INTO samples (rel_path, filename, ext, size, mtime) VALUES (?1,?2,?3,?4,?5)",
            )?;
            for e in lote {
                let ruta = e.ruta.to_string_lossy();
                let nombre = e
                    .ruta
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let ext = e
                    .ruta
                    .extension()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                st.execute(rusqlite::params![ruta, nombre, ext, e.size as i64, e.mtime])?;
            }
        }
        tx.commit()?;
    }
    conn.execute_batch("CREATE INDEX idx_ext ON samples(ext);")?;
    Ok(t0.elapsed().as_secs_f64())
}

/// Picos min/max por bucket, 2 bytes por bucket: exactamente lo que la app guardará como BLOB.
pub fn picos(muestras: &[f32], canales: u16, buckets: usize) -> Vec<i8> {
    let ch = canales.max(1) as usize;
    let frames = muestras.len() / ch;
    let mut out = vec![0i8; buckets * 2];
    if frames == 0 {
        return out;
    }
    let por_bucket = (frames as f64 / buckets as f64).max(1.0);
    for b in 0..buckets {
        let desde = (b as f64 * por_bucket) as usize;
        let hasta = (((b + 1) as f64 * por_bucket) as usize).min(frames);
        let (mut mn, mut mx) = (0.0f32, 0.0f32);
        for f in desde..hasta {
            for c in 0..ch {
                let v = muestras[f * ch + c];
                if v < mn {
                    mn = v;
                }
                if v > mx {
                    mx = v;
                }
            }
        }
        out[b * 2] = (mn.clamp(-1.0, 1.0) * 127.0) as i8;
        out[b * 2 + 1] = (mx.clamp(-1.0, 1.0) * 127.0) as i8;
    }
    out
}

pub struct Analisis {
    pub ok: usize,
    pub fallos: usize,
    pub segundos: f64,
    pub bytes_audio: u64,
}

/// Decodifica + picos + hash de contenido sobre N archivos, en paralelo con rayon.
pub fn analizar(archivos: &[PathBuf]) -> Analisis {
    let t0 = Instant::now();
    let resultados: Vec<(bool, u64)> = archivos
        .par_iter()
        .map(|p| match crate::decode::decodificar(p) {
            Ok(buf) => {
                let _picos = picos(&buf.samples, buf.channels, 1000);
                // SAFETY: f32 no tiene bits inválidos ni relleno; alineación 4 ≥ 1.
                let bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(
                        buf.samples.as_ptr() as *const u8,
                        buf.samples.len() * 4,
                    )
                };
                let _hash = blake3::hash(bytes);
                (true, buf.bytes() as u64)
            }
            Err(_) => (false, 0),
        })
        .collect();

    Analisis {
        ok: resultados.iter().filter(|r| r.0).count(),
        fallos: resultados.iter().filter(|r| !r.0).count(),
        segundos: t0.elapsed().as_secs_f64(),
        bytes_audio: resultados.iter().map(|r| r.1).sum(),
    }
}
