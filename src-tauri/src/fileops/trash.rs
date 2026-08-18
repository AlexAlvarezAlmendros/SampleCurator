//! Papelera gestionada. `X` no borra: mueve a `<destino>/.samplecurator-trash/` y deja
//! constancia en un manifiesto, para que restaurar sea posible incluso sin la base de datos.

use crate::error::Result;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const CARPETA: &str = ".samplecurator-trash";
const MANIFIESTO: &str = "manifiesto.jsonl";

pub fn carpeta(dest_root: &Path) -> PathBuf {
    dest_root.join(CARPETA)
}

pub fn asegurar(dest_root: &Path) -> Result<PathBuf> {
    let dir = carpeta(dest_root);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Anota en el manifiesto de dónde venía el archivo. Si algún día se pierde el índice,
/// esto sigue bastando para devolver todo a su sitio.
pub fn anotar(dest_root: &Path, sample_id: i64, desde: &Path, hasta: &Path) -> Result<()> {
    let dir = asegurar(dest_root)?;
    let linea = format!(
        "{{\"at\":{},\"sampleId\":{},\"from\":{},\"to\":{}}}\n",
        crate::db::ahora_ms(),
        sample_id,
        escapar(&desde.to_string_lossy()),
        escapar(&hasta.to_string_lossy())
    );
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(MANIFIESTO))?;
    f.write_all(linea.as_bytes())?;
    Ok(())
}

fn escapar(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Una línea del manifiesto, tal cual se escribió.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Anotacion {
    pub at: i64,
    #[serde(rename = "sampleId")]
    pub sample_id: i64,
    pub from: String,
    pub to: String,
}

/// Lee el manifiesto y añade los archivos que estén en la carpeta sin anotación.
///
/// Puede haber archivos sin línea —manifiesto perdido, copiado a mano— y se listan igual: la
/// carpeta es la verdad, el manifiesto solo dice de dónde venía cada uno.
pub fn entradas(dest_root: &Path) -> Vec<Anotacion> {
    let dir = carpeta(dest_root);
    let mut anotadas: Vec<Anotacion> = Vec::new();

    if let Ok(texto) = std::fs::read_to_string(dir.join(MANIFIESTO)) {
        for linea in texto.lines() {
            if let Ok(a) = serde_json::from_str::<Anotacion>(linea) {
                if Path::new(&a.to).exists() {
                    anotadas.push(a);
                }
            }
        }
    }

    let conocidas: std::collections::HashSet<String> =
        anotadas.iter().map(|a| a.to.clone()).collect();

    if let Ok(entradas) = std::fs::read_dir(&dir) {
        for e in entradas.flatten() {
            if e.file_name() == MANIFIESTO {
                continue;
            }
            if !e.metadata().map(|m| m.is_file()).unwrap_or(false) {
                continue;
            }
            let ruta = e.path().to_string_lossy().to_string();
            if conocidas.contains(&ruta) {
                continue;
            }
            // Sin anotación no se sabe de dónde venía: se deja `from` vacío y quien restaure
            // tendrá que elegir destino.
            anotadas.push(Anotacion {
                at: 0,
                sample_id: 0,
                from: String::new(),
                to: ruta,
            });
        }
    }

    // Lo último rechazado, primero: es lo que más probablemente quieras recuperar.
    anotadas.sort_by_key(|a| std::cmp::Reverse(a.at));
    anotadas
}

/// Quita una línea del manifiesto tras restaurar. Se reescribe entero de forma atómica: son
/// unos pocos kilobytes y así no puede quedar a medias.
pub fn olvidar(dest_root: &Path, trash_path: &str) -> Result<()> {
    let dir = carpeta(dest_root);
    let archivo = dir.join(MANIFIESTO);
    let Ok(texto) = std::fs::read_to_string(&archivo) else {
        return Ok(());
    };
    let quedan: Vec<&str> = texto
        .lines()
        .filter(|l| {
            serde_json::from_str::<Anotacion>(l)
                .map(|a| a.to != trash_path)
                .unwrap_or(true)
        })
        .collect();

    let temporal = archivo.with_extension("jsonl.tmp");
    std::fs::write(
        &temporal,
        quedan.join("\n") + if quedan.is_empty() { "" } else { "\n" },
    )?;
    std::fs::rename(&temporal, &archivo)?;
    Ok(())
}

pub struct Resumen {
    pub archivos: i64,
    pub bytes: i64,
}

pub fn resumen(dest_root: &Path) -> Resumen {
    let dir = carpeta(dest_root);
    let mut r = Resumen {
        archivos: 0,
        bytes: 0,
    };
    let Ok(entradas) = std::fs::read_dir(&dir) else {
        return r;
    };
    for e in entradas.flatten() {
        if e.file_name() == MANIFIESTO {
            continue;
        }
        if let Ok(m) = e.metadata() {
            if m.is_file() {
                r.archivos += 1;
                r.bytes += m.len() as i64;
            }
        }
    }
    r
}

/// Vaciar la papelera es la ÚNICA operación irreversible de la app, y por eso es la única
/// que el frontend confirma con un diálogo.
pub fn vaciar(dest_root: &Path) -> Result<i64> {
    let dir = carpeta(dest_root);
    let mut borrados = 0i64;
    let Ok(entradas) = std::fs::read_dir(&dir) else {
        return Ok(0);
    };
    for e in entradas.flatten() {
        if e.file_name() == MANIFIESTO {
            continue;
        }
        if e.metadata().map(|m| m.is_file()).unwrap_or(false) {
            std::fs::remove_file(e.path())?;
            borrados += 1;
        }
    }
    let _ = std::fs::remove_file(dir.join(MANIFIESTO));
    Ok(borrados)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_manifiesto_guarda_la_ruta_original_con_comillas_escapadas() {
        let tmp = tempfile::tempdir().unwrap();
        anotar(
            tmp.path(),
            7,
            Path::new("/musica/un \"kick\".wav"),
            &carpeta(tmp.path()).join("un kick.wav"),
        )
        .unwrap();
        let txt = std::fs::read_to_string(carpeta(tmp.path()).join(MANIFIESTO)).unwrap();
        assert!(
            txt.contains("\\\"kick\\\""),
            "las comillas deben ir escapadas: {txt}"
        );
        assert!(txt.contains("\"sampleId\":7"));
        // y debe ser JSON válido línea a línea
        assert!(txt.lines().all(|l| l.starts_with('{') && l.ends_with('}')));
    }

    #[test]
    fn el_resumen_no_cuenta_el_manifiesto() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = asegurar(tmp.path()).unwrap();
        std::fs::write(dir.join("a.wav"), b"12345").unwrap();
        anotar(tmp.path(), 1, Path::new("/x/a.wav"), &dir.join("a.wav")).unwrap();
        let r = resumen(tmp.path());
        assert_eq!(r.archivos, 1);
        assert_eq!(r.bytes, 5);
    }
}
