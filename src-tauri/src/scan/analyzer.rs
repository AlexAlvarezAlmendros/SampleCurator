//! Análisis en segundo plano: duración, formato, picos, sonoridad y hash de contenido.
//!
//! Va por detrás del escaneo a propósito. La lista se ve entera desde el primer segundo y los
//! datos van apareciendo solos; nunca hay una pantalla de carga bloqueante.

use crate::db::{queries, Db};
use crate::domain::ScanProgress;
use crate::error::Result;
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

const LOTE: i64 = 256;

/// Pool propio para el análisis, con DOS medidas para no robarle CPU al hilo de audio:
///
///   1. deja dos núcleos libres (uno para el audio, otro para la interfaz)
///   2. baja la prioridad de sus hilos con `nice(+10)`, así el hilo de audio siempre
///      les gana el turno
///
/// Sin esto se midió un *underrun* del stream analizando 50.000 samples de golpe: el motor
/// de audio quedaba a la cola detrás de 16 hilos de rayon del mismo proceso. Y sobra margen
/// para permitírselo: el análisis completo va 100× por debajo de su presupuesto (ADR-0005).
fn pool() -> Option<&'static rayon::ThreadPool> {
    static POOL: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    POOL.get_or_init(|| {
        let nucleos = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let hilos = nucleos.saturating_sub(2).max(1);
        rayon::ThreadPoolBuilder::new()
            .num_threads(hilos)
            .thread_name(|i| format!("analisis-{i}"))
            .start_handler(|_| {
                // Solo afecta al hilo que la llama, que es justo lo que queremos.
                #[cfg(unix)]
                unsafe {
                    libc::nice(10);
                }
            })
            .build()
            .ok()
    })
    .as_ref()
}

fn en_paralelo<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    match pool() {
        Some(p) => p.install(f),
        None => f(),
    }
}

/// Analiza todo lo pendiente. Se puede cancelar (cerrar la app, cambiar de biblioteca) y
/// reanudar más tarde: el estado vive en la columna `analyzed_at`, no en memoria.
pub fn analizar_pendientes(
    db: &Arc<Db>,
    cancelar: &AtomicBool,
    progreso: impl FnMut(ScanProgress),
) -> Result<i64> {
    let mut total_analizados = 0i64;
    let mut emisor = super::Emisor::nuevo(progreso);

    loop {
        if cancelar.load(Ordering::Relaxed) {
            break;
        }
        let pendientes = db.read(|c| queries::pending_analysis(c, LOTE))?;
        if pendientes.is_empty() {
            break;
        }

        let resultados: Vec<(i64, Option<crate::codec::Analisis>)> = en_paralelo(|| {
            pendientes
                .par_iter()
                .map(|(id, ruta)| (*id, crate::codec::analizar(ruta).ok()))
                .collect()
        });

        db.write(|conn| {
            let tx = conn.transaction()?;
            for (id, analisis) in &resultados {
                match analisis {
                    Some(a) => {
                        let fila = queries::Analisis {
                            duration_ms: a.duration_ms,
                            sample_rate: a.sample_rate,
                            channels: a.channels,
                            bit_depth: a.bit_depth,
                            loudness_db: a.loudness_db,
                            peaks: a.peaks.clone(),
                            content_hash: a.content_hash.clone(),
                        };
                        queries::store_analysis(&tx, *id, &fila)?;
                    }
                    None => queries::mark_broken(&tx, *id)?,
                }
            }
            tx.commit()?;
            Ok(())
        })?;

        total_analizados += resultados.len() as i64;
        let restantes = db.read(queries::count_pending_analysis)?;
        let e = emisor.estado_mut();
        e.analyzed = total_analizados;
        e.pending_analysis = restantes;
        emisor.quiza();
    }

    let e = emisor.estado_mut();
    e.done = true;
    e.pending_analysis = db.read(queries::count_pending_analysis)?;
    emisor.forzar();
    Ok(total_analizados)
}
