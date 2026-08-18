//! Prueba a escala real: 50.000 samples de verdad, con los números que exige
//! `docs/PERFORMANCE.md`. Se ejecuta a mano porque necesita una biblioteca grande en disco:
//!
//!   SC_LIB=~/.cache/samplecurator-spike/lib50k \
//!   SC_DB=/tmp/escala.db \
//!   cargo test --release --test escala -- --ignored --nocapture
//!
//! `SC_DB` puede apuntar a la base real de la app (~/.local/share/<id>/library.db) para
//! dejarla poblada y poder abrirla con datos.

use samplecurator_lib::db::{queries, Db};
use samplecurator_lib::domain::*;
use samplecurator_lib::scan;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

fn consulta(source_id: Option<i64>, busqueda: Option<&str>, estado: StatusFilter) -> LibraryQuery {
    LibraryQuery {
        source_id,
        search: busqueda.map(|s| s.to_string()),
        status: estado,
        sort: SortBy::Path,
        min_duration_ms: None,
        max_duration_ms: None,
        min_rating: 0,
        offset: 0,
        limit: 200,
    }
}

#[test]
#[ignore = "necesita una biblioteca grande; ver la cabecera del archivo"]
fn cincuenta_mil_samples_dentro_de_presupuesto() {
    let lib = PathBuf::from(std::env::var("SC_LIB").expect("falta SC_LIB"));
    let db_path = PathBuf::from(
        std::env::var("SC_DB").unwrap_or_else(|_| "/tmp/samplecurator-escala.db".to_string()),
    );
    assert!(lib.is_dir(), "SC_LIB no es una carpeta: {}", lib.display());

    let db = Arc::new(Db::open(&db_path).unwrap());
    let sid = db
        .read(|c| queries::add_source(c, lib.to_str().unwrap()))
        .unwrap();

    // ── escaneo ──────────────────────────────────────────────
    let t0 = Instant::now();
    scan::escanear(&db, sid, &lib, |_| {}).unwrap();
    let s_escaneo = t0.elapsed().as_secs_f64();
    let stats = db.read(|c| queries::stats(c, Some(sid))).unwrap();
    println!(
        "escaneo e indexado : {s_escaneo:.2} s · {} samples",
        stats.total
    );
    assert!(
        stats.total > 40_000,
        "se esperaban ~50.000, hay {}",
        stats.total
    );
    assert!(s_escaneo < 5.0, "presupuesto 5 s, tardó {s_escaneo:.2} s");

    // ── consulta de página ───────────────────────────────────
    let mut peor = 0.0f64;
    for offset in [0i64, 10_000, 25_000, 49_000] {
        let mut q = consulta(Some(sid), None, StatusFilter::All);
        q.offset = offset;
        let t = Instant::now();
        let p = db.read(|c| queries::page(c, &q)).unwrap();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        peor = peor.max(ms);
        assert_eq!(p.rows.len(), 200);
    }
    println!("página de 200      : {peor:.1} ms (la peor de cuatro posiciones)");
    assert!(peor < 50.0, "una página no puede tardar {peor:.1} ms");

    // ── búsqueda incremental ─────────────────────────────────
    let mut peor_busqueda = 0.0f64;
    for termino in ["kick", "sn", "hat 01", "loop"] {
        let t = Instant::now();
        let p = db
            .read(|c| queries::page(c, &consulta(Some(sid), Some(termino), StatusFilter::All)))
            .unwrap();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        peor_busqueda = peor_busqueda.max(ms);
        println!(
            "  buscar {termino:<8} → {} resultados en {ms:.1} ms",
            p.total
        );
    }
    assert!(
        peor_busqueda < 50.0,
        "presupuesto de búsqueda 50 ms, la peor tardó {peor_busqueda:.1} ms"
    );

    // ── análisis en segundo plano ────────────────────────────
    let pendientes = db.read(queries::count_pending_analysis).unwrap();
    if pendientes > 0 {
        let t = Instant::now();
        let cancelar = AtomicBool::new(false);
        let n = scan::analyzer::analizar_pendientes(&db, &cancelar, |_| {}).unwrap();
        let s = t.elapsed().as_secs_f64();
        println!("análisis completo  : {s:.1} s para {n} samples");
        assert!(s < 60.0, "presupuesto 1 min, tardó {s:.1} s");
    }

    let stats = db.read(|c| queries::stats(c, Some(sid))).unwrap();
    println!(
        "estado final       : {} analizados · {} duplicados",
        stats.analyzed, stats.duplicates
    );
    println!("base de datos      : {}", db_path.display());
}
