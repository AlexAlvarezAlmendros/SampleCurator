//! El arnés de evaluación de la Fase 8 (tarea 8.0.6).
//!
//! Comprueba que la báscula pesa bien antes de subirse a ella: que la referencia débil se
//! construye sola, que las correcciones del usuario mandan, y que el acuerdo entre ambas se
//! calcula como debe —incluidos los errores de octava y los relativos, que no son fallos
//! del todo pero tampoco aciertos.

use samplecurator_lib::db::{labels, queries, triage, Db};
use samplecurator_lib::domain::*;
use samplecurator_lib::scan;
use std::path::Path;

fn wav(dir: &Path, nombre: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let datos: Vec<u8> = (0..400u16).flat_map(|i| (i as i16).to_le_bytes()).collect();
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

struct Escenario {
    _tmp: tempfile::TempDir,
    db: Db,
}

fn montar(archivos: &[(&str, &str)]) -> Escenario {
    let tmp = tempfile::tempdir().unwrap();
    let lib = tmp.path().join("packs");
    for (sub, nombre) in archivos {
        wav(&lib.join(sub), nombre);
    }
    let db = Db::open(&tmp.path().join("t.db")).unwrap();
    let sid = db
        .read(|c| queries::add_source(c, lib.to_str().unwrap()))
        .unwrap();
    scan::escanear(&db, sid, &lib, |_| {}).unwrap();
    Escenario { _tmp: tmp, db }
}

fn id_de(e: &Escenario, nombre: &str) -> i64 {
    let q = LibraryQuery {
        source_id: None,
        search: None,
        status: StatusFilter::All,
        sort: SortBy::Path,
        min_duration_ms: None,
        max_duration_ms: None,
        min_rating: 0,
        unrated: false,
        dest_id: None,
        tag: None,
        offset: 0,
        limit: 500,
    };
    e.db.read(|c| queries::page(c, &q))
        .unwrap()
        .rows
        .iter()
        .find(|r| r.filename == nombre)
        .unwrap_or_else(|| panic!("no está {nombre}"))
        .id
}

fn etiqueta(e: &Escenario, id: i64, campo: &str, origen: LabelSource) -> Option<String> {
    e.db.read(|c| labels::de_sample(c, id))
        .unwrap()
        .into_iter()
        .find(|l| l.field == campo && l.source == origen)
        .map(|l| l.value)
}

fn poner_verdad(e: &Escenario, id: i64, campo: &str, valor: &str) {
    e.db.write(|conn| {
        let tx = conn.transaction()?;
        labels::upsert(&tx, id, campo, valor, 1.0, LabelSource::User)?;
        tx.commit()?;
        Ok(())
    })
    .unwrap();
}

#[test]
fn la_referencia_debil_se_construye_sola_desde_los_nombres() {
    let e = montar(&[
        ("kicks", "KICK_808_deep.wav"),
        ("loops", "Drumloop_124bpm_Amin.wav"),
        ("sin_pistas", "export_final_v3.wav"),
    ]);

    let informe = scan::labeling::extraer_de_nombres(&e.db).unwrap();
    assert_eq!(informe.processed, 3);
    assert_eq!(
        informe.kind, 2,
        "el archivo sin pistas no debe recibir tipo"
    );
    assert_eq!(informe.bpm, 1);
    assert_eq!(informe.key, 1);

    let kick = id_de(&e, "KICK_808_deep.wav");
    assert_eq!(
        etiqueta(&e, kick, "kind", LabelSource::Filename),
        Some("kick".into())
    );

    let loop_ = id_de(&e, "Drumloop_124bpm_Amin.wav");
    assert_eq!(
        etiqueta(&e, loop_, "bpm", LabelSource::Filename),
        Some("124".into())
    );
    assert_eq!(
        etiqueta(&e, loop_, "key", LabelSource::Filename),
        Some("A:min".into())
    );

    let mudo = id_de(&e, "export_final_v3.wav");
    assert_eq!(etiqueta(&e, mudo, "kind", LabelSource::Filename), None);
}

#[test]
fn repetir_la_extraccion_actualiza_en_vez_de_duplicar() {
    let e = montar(&[("kicks", "KICK_01.wav")]);
    scan::labeling::extraer_de_nombres(&e.db).unwrap();
    scan::labeling::extraer_de_nombres(&e.db).unwrap();

    let id = id_de(&e, "KICK_01.wav");
    let etiquetas = e.db.read(|c| labels::de_sample(c, id)).unwrap();
    let tipos = etiquetas.iter().filter(|l| l.field == "kind").count();
    assert_eq!(
        tipos, 1,
        "no puede haber dos etiquetas de tipo para el mismo origen"
    );
}

#[test]
fn la_correccion_del_usuario_convive_con_la_del_nombre_sin_pisarla() {
    // Carpeta neutra a propósito: si estuviera en `perc/`, la carpeta ganaría al nombre (esa
    // regla se prueba en el módulo del extractor) y aquí lo que se mide es otra cosa.
    let e = montar(&[("varios", "SNARE_ish_thing.wav")]);
    scan::labeling::extraer_de_nombres(&e.db).unwrap();
    let id = id_de(&e, "SNARE_ish_thing.wav");

    // el nombre dijo "snare"; el usuario dice que en realidad es un rimshot percusivo
    poner_verdad(&e, id, "kind", "perc");

    assert_eq!(
        etiqueta(&e, id, "kind", LabelSource::Filename),
        Some("snare".into())
    );
    assert_eq!(
        etiqueta(&e, id, "kind", LabelSource::User),
        Some("perc".into())
    );
}

#[test]
fn la_cobertura_mide_cuanto_mienten_los_nombres() {
    let e = montar(&[
        ("kicks", "KICK_a.wav"),
        ("kicks", "KICK_b.wav"),
        ("snares", "SNARE_a.wav"),
        ("loops", "loop_128bpm.wav"),
        ("loops", "loop_140bpm.wav"),
    ]);
    scan::labeling::extraer_de_nombres(&e.db).unwrap();

    // Tres aciertos y un fallo del nombre en el tipo
    poner_verdad(&e, id_de(&e, "KICK_a.wav"), "kind", "kick");
    poner_verdad(&e, id_de(&e, "KICK_b.wav"), "kind", "kick");
    poner_verdad(&e, id_de(&e, "SNARE_a.wav"), "kind", "clap");
    poner_verdad(&e, id_de(&e, "loop_128bpm.wav"), "kind", "loop");

    // Un BPM exacto y otro con error de octava
    poner_verdad(&e, id_de(&e, "loop_128bpm.wav"), "bpm", "128");
    poner_verdad(&e, id_de(&e, "loop_140bpm.wav"), "bpm", "70");

    let stats = scan::labeling::cobertura(&e.db).unwrap();
    let tipo = stats.fields.iter().find(|f| f.field == "kind").unwrap();
    assert_eq!(tipo.pairs, 4);
    assert_eq!(tipo.exact, 3);
    assert_eq!(tipo.wrong, 1);
    assert!(
        (tipo.accuracy - 0.75).abs() < 1e-9,
        "los nombres aciertan 3 de 4"
    );

    let bpm = stats.fields.iter().find(|f| f.field == "bpm").unwrap();
    assert_eq!(bpm.pairs, 2);
    assert_eq!(bpm.exact, 1);
    assert_eq!(
        bpm.close, 1,
        "140 contra 70 es error de octava, no un disparate"
    );
    assert_eq!(bpm.wrong, 0);

    assert_eq!(
        stats.labeled_samples, 5,
        "cinco samples distintos con verdad del usuario"
    );
}

#[test]
fn cuenta_aparte_lo_que_el_nombre_no_supo_describir() {
    let e = montar(&[("varios", "export_1.wav"), ("kicks", "KICK.wav")]);
    scan::labeling::extraer_de_nombres(&e.db).unwrap();

    poner_verdad(&e, id_de(&e, "export_1.wav"), "kind", "fx");
    poner_verdad(&e, id_de(&e, "KICK.wav"), "kind", "kick");

    let stats = scan::labeling::cobertura(&e.db).unwrap();
    let tipo = stats.fields.iter().find(|f| f.field == "kind").unwrap();
    assert_eq!(tipo.pairs, 1, "solo uno tiene ambas referencias");
    assert_eq!(
        tipo.only_user, 1,
        "el otro es material que el nombre no describía: ahí es donde el clasificador tendrá que ganarse el sueldo"
    );
}

#[test]
fn el_muestreo_reparte_entre_clases_en_vez_de_traer_solo_kicks() {
    let mut archivos: Vec<(String, String)> = (0..30)
        .map(|i| ("kicks".to_string(), format!("KICK_{i:02}.wav")))
        .collect();
    archivos.push(("snares".into(), "SNARE_01.wav".into()));
    archivos.push(("hats".into(), "HAT_01.wav".into()));
    let refs: Vec<(&str, &str)> = archivos
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();

    let e = montar(&refs);
    scan::labeling::extraer_de_nombres(&e.db).unwrap();

    let muestra = e.db.read(|c| labels::muestreo_estratificado(c, 2)).unwrap();
    let tipos: Vec<String> = muestra
        .iter()
        .filter_map(|id| etiqueta(&e, *id, "kind", LabelSource::Filename))
        .collect();

    assert!(
        tipos.contains(&"snare".to_string()),
        "la caja tiene que entrar"
    );
    assert!(tipos.contains(&"hat".to_string()), "el charles también");
    assert!(
        tipos.iter().filter(|t| *t == "kick").count() <= 2,
        "y no puede traer los 30 kicks: {tipos:?}"
    );
}

#[test]
fn el_proyecto_sigue_funcionando_con_la_migracion_nueva() {
    // Comprobación de que añadir la tabla de etiquetas no ha roto nada de lo anterior.
    let e = montar(&[("kicks", "k.wav")]);
    let p =
        e.db.read(|c| triage::create_project(c, "s", "/tmp", TriageMode::Move))
            .unwrap();
    assert_eq!(p.name, "s");
    let stats = e.db.read(|c| queries::stats(c, None)).unwrap();
    assert_eq!(stats.total, 1);
}

/// Ejecuta la tarea 8.0.3 sobre una biblioteca de verdad y publica los números.
///
///   SC_DB=~/.local/share/dev.alexalvarez.samplecurator/library.db \
///   cargo test --release --test etiquetado -- --ignored --nocapture
///
/// Aviso: sobre la biblioteca SINTÉTICA estos porcentajes no significan nada. Sus carpetas se
/// llaman exactamente como las clases, así que el extractor acierta el 100 % sin demostrar
/// nada. Lo único que mide ahí es el rendimiento.
#[test]
#[ignore = "necesita una biblioteca indexada; ver la cabecera"]
fn extraccion_a_escala() {
    let ruta = std::env::var("SC_DB").expect("falta SC_DB");
    let db = Db::open(Path::new(&ruta)).unwrap();

    let informe = scan::labeling::extraer_de_nombres(&db).unwrap();
    println!("\n── extracción de nombres ──");
    println!("  samples leídos : {}", informe.processed);
    println!(
        "  con tipo       : {} ({:.0} %)",
        informe.kind,
        100.0 * informe.kind as f64 / informe.processed.max(1) as f64
    );
    println!("  con BPM        : {}", informe.bpm);
    println!("  con tonalidad  : {}", informe.key);
    println!("  con altura     : {}", informe.pitch);
    println!(
        "  tiempo         : {} ms ({:.3} ms por archivo)",
        informe.millis,
        informe.millis as f64 / informe.processed.max(1) as f64
    );

    let stats = scan::labeling::cobertura(&db).unwrap();
    println!("\n── acuerdo con tus correcciones ──");
    println!(
        "  samples con verdad tuya: {} de {}",
        stats.labeled_samples, stats.target
    );
    for f in &stats.fields {
        if f.pairs == 0 {
            println!("  {:<6} sin comparaciones todavía", f.field);
            continue;
        }
        println!(
            "  {:<6} {:.0} % exacto · {} cercanos · {} fallos · {} pares · {} sin pista en el nombre",
            f.field, f.accuracy * 100.0, f.close, f.wrong, f.pairs, f.only_user
        );
    }
    assert!(informe.processed > 0, "la biblioteca está vacía");
}
