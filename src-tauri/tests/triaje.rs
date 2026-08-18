//! Integración del triaje sobre archivos de verdad (en TempDir): mover, rechazar, deshacer,
//! colisiones y reparación tras un corte. Es la parte donde un bug destruye trabajo del
//! usuario, así que aquí no se escatima.

use samplecurator_lib::db::{queries, triage, Db};
use samplecurator_lib::domain::*;
use samplecurator_lib::{fileops, scan};
use std::path::{Path, PathBuf};

fn wav(dir: &Path, nombre: &str, semilla: u16) {
    std::fs::create_dir_all(dir).unwrap();
    let datos: Vec<u8> = (0..2000u16)
        .flat_map(|i| (i.wrapping_mul(semilla) as i16).to_le_bytes())
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

struct Escenario {
    _tmp: tempfile::TempDir,
    db: Db,
    origen: PathBuf,
    destino_raiz: PathBuf,
    proyecto: Project,
    dest_kicks: Destination,
    source_id: i64,
}

fn montar(modo: TriageMode, archivos: &[(&str, &str)]) -> Escenario {
    let tmp = tempfile::tempdir().unwrap();
    let origen = tmp.path().join("desorden");
    let destino_raiz = tmp.path().join("libreria");
    std::fs::create_dir_all(&origen).unwrap();
    std::fs::create_dir_all(&destino_raiz).unwrap();
    for (i, (sub, nombre)) in archivos.iter().enumerate() {
        wav(&origen.join(sub), nombre, (i as u16) + 3);
    }

    let db = Db::open(&tmp.path().join("index.db")).unwrap();
    let source_id = db
        .read(|c| queries::add_source(c, origen.to_str().unwrap()))
        .unwrap();
    scan::escanear(&db, source_id, &origen, |_| {}).unwrap();

    let proyecto = db
        .read(|c| triage::create_project(c, "sesión", destino_raiz.to_str().unwrap(), modo))
        .unwrap();
    let dest_kicks = db
        .read(|c| triage::create_destination(c, proyecto.id, "Kicks", "Kicks"))
        .unwrap();

    Escenario {
        _tmp: tmp,
        db,
        origen,
        destino_raiz,
        proyecto,
        dest_kicks,
        source_id,
    }
}

/// Busca por nombre en vez de por posición: el orden de la lista es por ruta, y depender de
/// él hace que el test mienta en cuanto cambia el escenario.
fn id_de(e: &Escenario, nombre: &str) -> i64 {
    let q = LibraryQuery {
        source_id: Some(e.source_id),
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
        .unwrap_or_else(|| panic!("no está {nombre} en el índice"))
        .id
}

fn ids(e: &Escenario) -> Vec<i64> {
    let q = LibraryQuery {
        source_id: Some(e.source_id),
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
        .map(|r| r.id)
        .collect()
}

#[test]
fn mover_lleva_el_archivo_al_destino_y_deshacer_lo_devuelve() {
    let e = montar(
        TriageMode::Move,
        &[("kicks", "k1.wav"), ("kicks", "k2.wav")],
    );
    let todos = ids(&e);
    let origen_k1 = e.origen.join("kicks/k1.wav");
    assert!(origen_k1.exists());

    let r = fileops::enviar(&e.db, e.proyecto.id, e.dest_kicks.id, &todos[..1]).unwrap();
    assert_eq!(r.affected.len(), 1);
    assert_eq!(r.destination_count, Some(1));
    assert!(
        !origen_k1.exists(),
        "en modo mover el original ya no está en el origen"
    );
    assert!(e.destino_raiz.join("Kicks/k1.wav").exists());

    let estado = e.db.read(|c| triage::sample_state(c, todos[0])).unwrap();
    assert_eq!(estado.status, SampleStatus::Moved);
    assert_eq!(estado.dest_id, Some(e.dest_kicks.id));

    // deshacer devuelve el archivo, el estado, el contador y el foco
    let u = fileops::deshacer(&e.db).unwrap();
    assert_eq!(u.restored, vec![todos[0]]);
    assert_eq!(u.focus_sample_id, Some(todos[0]));
    assert_eq!(u.destination_count, Some(0));
    assert!(
        origen_k1.exists(),
        "el archivo vuelve exactamente a donde estaba"
    );
    assert!(!e.destino_raiz.join("Kicks/k1.wav").exists());

    let estado = e.db.read(|c| triage::sample_state(c, todos[0])).unwrap();
    assert_eq!(estado.status, SampleStatus::Pending);
    assert_eq!(estado.dest_id, None);
    assert_eq!(estado.current_path, None);
}

#[test]
fn tres_samples_con_el_mismo_nombre_no_se_pisan() {
    let e = montar(
        TriageMode::Move,
        &[
            ("packA", "kick.wav"),
            ("packB", "kick.wav"),
            ("packC", "kick.wav"),
        ],
    );
    let todos = ids(&e);
    assert_eq!(todos.len(), 3);

    let r = fileops::enviar(&e.db, e.proyecto.id, e.dest_kicks.id, &todos).unwrap();
    assert_eq!(r.affected.len(), 3, "los tres deben llegar");

    let dir = e.destino_raiz.join("Kicks");
    let mut nombres: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|x| x.file_name().to_string_lossy().to_string())
        .collect();
    nombres.sort();
    assert_eq!(nombres, vec!["kick (2).wav", "kick (3).wav", "kick.wav"]);

    // y los tres archivos siguen siendo distintos: ninguno se ha sobrescrito con otro
    let mut contenidos: Vec<Vec<u8>> = nombres
        .iter()
        .map(|n| std::fs::read(dir.join(n)).unwrap())
        .collect();
    contenidos.sort();
    contenidos.dedup();
    assert_eq!(
        contenidos.len(),
        3,
        "ningún archivo se ha perdido por colisión"
    );
}

#[test]
fn un_lote_de_cuarenta_se_deshace_con_una_sola_llamada() {
    let archivos: Vec<(String, String)> = (0..40)
        .map(|i| ("todo".to_string(), format!("s{i:02}.wav")))
        .collect();
    let refs: Vec<(&str, &str)> = archivos
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let e = montar(TriageMode::Move, &refs);
    let todos = ids(&e);
    assert_eq!(todos.len(), 40);

    let r = fileops::enviar(&e.db, e.proyecto.id, e.dest_kicks.id, &todos).unwrap();
    assert_eq!(r.affected.len(), 40);
    assert_eq!(
        std::fs::read_dir(e.destino_raiz.join("Kicks"))
            .unwrap()
            .count(),
        40
    );

    let u = fileops::deshacer(&e.db).unwrap();
    assert_eq!(
        u.restored.len(),
        40,
        "un solo Ctrl+Z deshace el lote entero"
    );
    assert_eq!(
        std::fs::read_dir(e.destino_raiz.join("Kicks"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        std::fs::read_dir(e.origen.join("todo")).unwrap().count(),
        40
    );
}

#[test]
fn rechazar_manda_a_la_papelera_y_deja_manifiesto() {
    let e = montar(TriageMode::Move, &[("fx", "ruido.wav")]);
    let todos = ids(&e);

    fileops::rechazar(&e.db, e.proyecto.id, &todos).unwrap();
    let papelera = fileops::trash::carpeta(&e.destino_raiz);
    assert!(
        papelera.join("ruido.wav").exists(),
        "no se borra: se aparta"
    );
    assert!(!e.origen.join("fx/ruido.wav").exists());

    let manifiesto = std::fs::read_to_string(papelera.join("manifiesto.jsonl")).unwrap();
    assert!(manifiesto.contains("ruido.wav"));

    let estado = e.db.read(|c| triage::sample_state(c, todos[0])).unwrap();
    assert_eq!(estado.status, SampleStatus::Rejected);

    // y se puede deshacer
    fileops::deshacer(&e.db).unwrap();
    assert!(e.origen.join("fx/ruido.wav").exists());
}

#[test]
fn en_modo_copiar_el_origen_no_se_toca_nunca() {
    let e = montar(TriageMode::Copy, &[("kicks", "k.wav"), ("fx", "malo.wav")]);
    let bueno = id_de(&e, "k.wav");
    let malo = id_de(&e, "malo.wav");

    fileops::enviar(&e.db, e.proyecto.id, e.dest_kicks.id, &[bueno]).unwrap();
    assert!(
        e.origen.join("kicks/k.wav").exists(),
        "copiar deja el original"
    );
    assert!(e.destino_raiz.join("Kicks/k.wav").exists());

    // rechazar en modo copiar solo marca: quien copia quiere su carpeta intacta
    fileops::rechazar(&e.db, e.proyecto.id, &[malo]).unwrap();
    assert!(e.origen.join("fx/malo.wav").exists());
    let estado = e.db.read(|c| triage::sample_state(c, malo)).unwrap();
    assert_eq!(estado.status, SampleStatus::Rejected);

    // deshacer la copia se lleva la copia, no el original
    fileops::deshacer(&e.db).unwrap(); // deshace el rechazo
    let u = fileops::deshacer(&e.db).unwrap(); // deshace la copia
    assert_eq!(u.kind, ActionKind::Copy);
    assert!(e.origen.join("kicks/k.wav").exists());
    assert!(!e.destino_raiz.join("Kicks/k.wav").exists());
}

#[test]
fn rehacer_vuelve_a_aplicar_lo_deshecho() {
    let e = montar(TriageMode::Move, &[("kicks", "k.wav")]);
    let todos = ids(&e);
    fileops::enviar(&e.db, e.proyecto.id, e.dest_kicks.id, &todos).unwrap();
    fileops::deshacer(&e.db).unwrap();
    assert!(e.origen.join("kicks/k.wav").exists());

    let r = fileops::rehacer(&e.db).unwrap();
    assert_eq!(r.restored, todos);
    assert!(e.destino_raiz.join("Kicks/k.wav").exists());
    assert_eq!(r.destination_count, Some(1));
    let estado = e.db.read(|c| triage::sample_state(c, todos[0])).unwrap();
    assert_eq!(estado.status, SampleStatus::Moved);
}

#[test]
fn un_corte_a_mitad_se_repara_conservando_el_original() {
    let e = montar(TriageMode::Move, &[("kicks", "k.wav")]);
    let todos = ids(&e);
    let origen = e.origen.join("kicks/k.wav");
    let destino = e.destino_raiz.join("Kicks/k.wav");
    std::fs::create_dir_all(destino.parent().unwrap()).unwrap();

    // Se simula el peor caso: journal escrito, copia hecha, y el proceso muere antes de
    // borrar el origen y antes de cerrar la acción.
    e.db.write(|conn| {
        let tx = conn.transaction()?;
        triage::begin_action(
            &tx,
            Some(e.proyecto.id),
            todos[0],
            Some(e.dest_kicks.id),
            ActionKind::Move,
            origen.to_str().unwrap(),
            destino.to_str().unwrap(),
            SampleStatus::Pending,
            None,
            None,
            "lote-roto",
        )?;
        tx.commit()?;
        Ok(())
    })
    .unwrap();
    std::fs::copy(&origen, &destino).unwrap();

    let arregladas = fileops::reparar(&e.db).unwrap();
    assert_eq!(arregladas, 1);
    assert!(
        origen.exists(),
        "ante la duda se conserva SIEMPRE el original"
    );
    assert!(!destino.exists(), "la copia dudosa se retira");
    let estado = e.db.read(|c| triage::sample_state(c, todos[0])).unwrap();
    assert_eq!(
        estado.status,
        SampleStatus::Pending,
        "el sample vuelve a la cola"
    );
}

#[test]
fn una_operacion_que_nunca_llego_a_pasar_se_descarta_al_reparar() {
    let e = montar(TriageMode::Move, &[("kicks", "k.wav")]);
    let todos = ids(&e);
    let origen = e.origen.join("kicks/k.wav");
    let destino = e.destino_raiz.join("Kicks/k.wav");

    e.db.write(|conn| {
        let tx = conn.transaction()?;
        triage::begin_action(
            &tx,
            Some(e.proyecto.id),
            todos[0],
            Some(e.dest_kicks.id),
            ActionKind::Move,
            origen.to_str().unwrap(),
            destino.to_str().unwrap(),
            SampleStatus::Pending,
            None,
            None,
            "lote-roto",
        )?;
        tx.commit()?;
        Ok(())
    })
    .unwrap();

    assert_eq!(fileops::reparar(&e.db).unwrap(), 1);
    assert!(origen.exists());
    let estado = e.db.read(|c| triage::sample_state(c, todos[0])).unwrap();
    assert_eq!(estado.status, SampleStatus::Pending);
    // y el journal queda limpio: no debe quedar nada que deshacer
    assert!(e.db.read(triage::last_batch).unwrap().is_none());
}

#[test]
fn conservar_no_toca_el_disco_pero_se_puede_deshacer() {
    let e = montar(TriageMode::Move, &[("kicks", "k.wav")]);
    let todos = ids(&e);
    let origen = e.origen.join("kicks/k.wav");

    fileops::conservar(&e.db, e.proyecto.id, &todos).unwrap();
    assert!(origen.exists());
    assert_eq!(
        e.db.read(|c| triage::sample_state(c, todos[0]))
            .unwrap()
            .status,
        SampleStatus::Kept
    );

    fileops::deshacer(&e.db).unwrap();
    assert_eq!(
        e.db.read(|c| triage::sample_state(c, todos[0]))
            .unwrap()
            .status,
        SampleStatus::Pending
    );
    assert!(origen.exists());
}

#[test]
fn renombrar_cambia_el_archivo_y_el_indice_y_se_puede_deshacer() {
    let e = montar(TriageMode::Move, &[("kicks", "sin_nombre_01.wav")]);
    let id = id_de(&e, "sin_nombre_01.wav");
    let original = e.origen.join("kicks/sin_nombre_01.wav");

    fileops::renombrar(&e.db, Some(e.proyecto.id), id, "KICK_808_grave.wav").unwrap();

    assert!(!original.exists());
    assert!(e.origen.join("kicks/KICK_808_grave.wav").exists());
    let d = e.db.read(|c| queries::detail(c, id)).unwrap();
    assert_eq!(d.row.filename, "KICK_808_grave.wav");
    assert_eq!(d.row.rel_path, "kicks/KICK_808_grave.wav");

    // y el nombre también vuelve con Ctrl+Z
    fileops::deshacer(&e.db).unwrap();
    assert!(original.exists());
    let d = e.db.read(|c| queries::detail(c, id)).unwrap();
    assert_eq!(d.row.filename, "sin_nombre_01.wav");
    assert_eq!(d.row.rel_path, "kicks/sin_nombre_01.wav");
}

#[test]
fn renombrar_no_pisa_un_archivo_existente_ni_acepta_rutas() {
    let e = montar(TriageMode::Move, &[("kicks", "a.wav"), ("kicks", "b.wav")]);
    let id = id_de(&e, "a.wav");

    assert!(fileops::renombrar(&e.db, None, id, "b.wav").is_err());
    assert!(fileops::renombrar(&e.db, None, id, "../fuera.wav").is_err());
    assert!(fileops::renombrar(&e.db, None, id, "sub/otro.wav").is_err());
    assert!(fileops::renombrar(&e.db, None, id, "   ").is_err());

    // nada se ha movido
    assert!(e.origen.join("kicks/a.wav").exists());
    assert!(e.origen.join("kicks/b.wav").exists());
    assert_eq!(
        e.db.read(|c| queries::detail(c, id)).unwrap().row.filename,
        "a.wav"
    );
}

#[test]
fn la_copia_de_seguridad_recoge_las_decisiones() {
    let e = montar(TriageMode::Move, &[("kicks", "k.wav"), ("fx", "malo.wav")]);
    let bueno = id_de(&e, "k.wav");
    let malo = id_de(&e, "malo.wav");

    fileops::enviar(&e.db, e.proyecto.id, e.dest_kicks.id, &[bueno]).unwrap();
    fileops::rechazar(&e.db, e.proyecto.id, &[malo]).unwrap();

    let ruta = fileops::export::exportar(&e.db, e.proyecto.id).unwrap();
    let texto = std::fs::read_to_string(&ruta).unwrap();
    assert!(texto.contains("\"destination\": \"Kicks\""));
    assert!(texto.contains("\"status\": \"rejected\""));
    assert!(texto.contains("\"hotkey\": \"1\""));
}

// ─────────────────────────── papelera ───────────────────────────

#[test]
fn lo_rechazado_se_ve_en_la_papelera_y_se_puede_devolver_a_su_sitio() {
    let e = montar(TriageMode::Move, &[("fx", "ruido.wav")]);
    let id = id_de(&e, "ruido.wav");
    let original = e.origen.join("fx/ruido.wav");

    fileops::rechazar(&e.db, e.proyecto.id, &[id]).unwrap();
    assert!(!original.exists());

    let papelera = fileops::papelera(&e.db, e.proyecto.id).unwrap();
    assert_eq!(papelera.len(), 1);
    assert_eq!(papelera[0].filename, "ruido.wav");
    assert_eq!(papelera[0].sample_id, Some(id));
    assert!(papelera[0].in_index);
    assert!(papelera[0].size > 0, "se ve cuánto ocupa antes de decidir");

    fileops::restaurar(&e.db, e.proyecto.id, &papelera[0].trash_path).unwrap();

    assert!(original.exists(), "vuelve exactamente a su carpeta");
    let estado = e.db.read(|c| triage::sample_state(c, id)).unwrap();
    assert_eq!(
        estado.status,
        SampleStatus::Pending,
        "restaurar devuelve a la cola"
    );
    assert_eq!(estado.current_path, None);
    assert!(
        fileops::papelera(&e.db, e.proyecto.id).unwrap().is_empty(),
        "y desaparece de la papelera"
    );
}

#[test]
fn restaurar_no_pisa_un_archivo_que_haya_ocupado_su_sitio() {
    let e = montar(TriageMode::Move, &[("fx", "ruido.wav")]);
    let id = id_de(&e, "ruido.wav");
    let original = e.origen.join("fx/ruido.wav");

    fileops::rechazar(&e.db, e.proyecto.id, &[id]).unwrap();
    // Mientras estaba en la papelera, alguien puso otro archivo con ese nombre.
    std::fs::write(&original, b"soy otro archivo distinto").unwrap();

    let papelera = fileops::papelera(&e.db, e.proyecto.id).unwrap();
    fileops::restaurar(&e.db, e.proyecto.id, &papelera[0].trash_path).unwrap();

    assert_eq!(
        std::fs::read(&original).unwrap(),
        b"soy otro archivo distinto",
        "el intruso no se toca"
    );
    assert!(
        e.origen.join("fx/ruido (2).wav").exists(),
        "el restaurado se coloca al lado, con sufijo"
    );
}

#[test]
fn una_entrada_sin_anotacion_no_se_restaura_a_ciegas() {
    let e = montar(TriageMode::Move, &[("fx", "a.wav")]);
    let dir = fileops::trash::asegurar(&e.destino_raiz).unwrap();
    // Un archivo que apareció en la papelera sin pasar por la app: no se sabe de dónde venía.
    std::fs::write(dir.join("huerfano.wav"), b"1234").unwrap();

    let papelera = fileops::papelera(&e.db, e.proyecto.id).unwrap();
    let huerfano = papelera
        .iter()
        .find(|x| x.filename == "huerfano.wav")
        .unwrap();
    assert!(!huerfano.in_index);
    assert_eq!(huerfano.original_path, "");

    let r = fileops::restaurar(&e.db, e.proyecto.id, &huerfano.trash_path);
    assert!(
        r.is_err(),
        "sin saber su origen, restaurar sería inventarse un destino"
    );
    assert!(
        dir.join("huerfano.wav").exists(),
        "y el archivo sigue donde estaba"
    );
}

#[test]
fn el_manifiesto_se_limpia_al_restaurar_pero_conserva_el_resto() {
    let e = montar(TriageMode::Move, &[("fx", "a.wav"), ("fx", "b.wav")]);
    let ids = ids(&e);
    fileops::rechazar(&e.db, e.proyecto.id, &ids).unwrap();

    let papelera = fileops::papelera(&e.db, e.proyecto.id).unwrap();
    assert_eq!(papelera.len(), 2);
    fileops::restaurar(&e.db, e.proyecto.id, &papelera[0].trash_path).unwrap();

    let quedan = fileops::papelera(&e.db, e.proyecto.id).unwrap();
    assert_eq!(quedan.len(), 1, "solo se va la restaurada");
    assert_ne!(quedan[0].trash_path, papelera[0].trash_path);

    let manifiesto =
        std::fs::read_to_string(fileops::trash::carpeta(&e.destino_raiz).join("manifiesto.jsonl"))
            .unwrap();
    assert_eq!(
        manifiesto.lines().count(),
        1,
        "y su línea desaparece del manifiesto"
    );
}
