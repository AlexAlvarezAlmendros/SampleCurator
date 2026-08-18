//! Mover y copiar archivos del usuario sin perder ni sobrescribir nada.
//!
//! Reglas que no se negocian:
//!   · nunca se sobrescribe un archivo existente: se añade sufijo ` (2)`, ` (3)`…
//!   · entre dispositivos se copia, se VERIFICA y solo entonces se borra el origen
//!   · el borrado del origen es la única operación irreversible, y va después del hash

use crate::error::{AppError, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Devuelve una ruta libre dentro de `dir` para `nombre`, añadiendo ` (2)`, ` (3)`…
///
/// `reservadas` son rutas que aún no existen en disco pero que ya se han apalabrado en este
/// mismo lote. Sin esto, mandar tres `kick.wav` de carpetas distintas al mismo destino
/// planificaría la misma ruta tres veces y dos se perderían.
pub fn ruta_libre(dir: &Path, nombre: &str, reservadas: &HashSet<PathBuf>) -> PathBuf {
    let candidata = dir.join(nombre);
    if !candidata.exists() && !reservadas.contains(&candidata) {
        return candidata;
    }
    let base = Path::new(nombre);
    let tallo = base
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| nombre.to_string());
    let ext = base.extension().map(|s| s.to_string_lossy().to_string());

    for n in 2..10_000u32 {
        let intento = match &ext {
            Some(e) => dir.join(format!("{tallo} ({n}).{e}")),
            None => dir.join(format!("{tallo} ({n})")),
        };
        if !intento.exists() && !reservadas.contains(&intento) {
            return intento;
        }
    }
    // Caso absurdo (10.000 colisiones): se cae del lado seguro con un nombre único por tiempo.
    dir.join(format!("{tallo}-{}", crate::db::ahora_ms()))
}

fn hash_archivo(p: &Path) -> Result<[u8; 32]> {
    let mut hasher = blake3::Hasher::new();
    let mut f = std::fs::File::open(p)?;
    std::io::copy(&mut f, &mut hasher)?;
    Ok(*hasher.finalize().as_bytes())
}

/// Mueve un archivo. Si el destino está en otro dispositivo, copia + verifica + borra.
pub fn mover(desde: &Path, hasta: &Path) -> Result<()> {
    if !desde.exists() {
        return Err(AppError::PathUnavailable(desde.to_path_buf()));
    }
    if hasta.exists() {
        return Err(AppError::Unsafe(format!(
            "el destino ya existe y no se sobrescribe: {}",
            hasta.display()
        )));
    }
    if let Some(dir) = hasta.parent() {
        std::fs::create_dir_all(dir)?;
    }

    match std::fs::rename(desde, hasta) {
        Ok(()) => Ok(()),
        // Entre dispositivos `rename` no vale: hay que copiar, verificar y solo entonces borrar.
        Err(_) => mover_entre_dispositivos(desde, hasta),
    }
}

/// Copia, verifica byte a byte y solo entonces borra el origen.
///
/// Es la única operación irreversible de la app sobre un archivo del usuario, así que la
/// verificación es un hash completo, no una comparación de tamaños: dos archivos del mismo
/// tamaño pueden diferir, y aquí equivocarse significa perder audio.
pub fn mover_entre_dispositivos(desde: &Path, hasta: &Path) -> Result<()> {
    std::fs::copy(desde, hasta)?;
    let (a, b) = (
        std::fs::metadata(desde)?.len(),
        std::fs::metadata(hasta)?.len(),
    );
    if a != b {
        let _ = std::fs::remove_file(hasta); // limpia la copia a medias, jamás el original
        return Err(AppError::Unsafe(format!(
            "la copia quedó incompleta ({b} de {a} bytes); el original sigue intacto"
        )));
    }
    if hash_archivo(desde)? != hash_archivo(hasta)? {
        let _ = std::fs::remove_file(hasta);
        return Err(AppError::Unsafe(
            "la copia no coincide con el original; el original sigue intacto".into(),
        ));
    }
    std::fs::remove_file(desde)?;
    Ok(())
}

pub fn copiar(desde: &Path, hasta: &Path) -> Result<()> {
    if !desde.exists() {
        return Err(AppError::PathUnavailable(desde.to_path_buf()));
    }
    if hasta.exists() {
        return Err(AppError::Unsafe(format!(
            "el destino ya existe y no se sobrescribe: {}",
            hasta.display()
        )));
    }
    if let Some(dir) = hasta.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::copy(desde, hasta)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nunca_sobrescribe_y_va_numerando() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path();
        let vacio = HashSet::new();
        std::fs::write(d.join("kick.wav"), b"uno").unwrap();
        let r2 = ruta_libre(d, "kick.wav", &vacio);
        assert_eq!(r2.file_name().unwrap(), "kick (2).wav");
        std::fs::write(&r2, b"dos").unwrap();
        let r3 = ruta_libre(d, "kick.wav", &vacio);
        assert_eq!(r3.file_name().unwrap(), "kick (3).wav");
        // el original sigue siendo el original
        assert_eq!(std::fs::read(d.join("kick.wav")).unwrap(), b"uno");
    }

    #[test]
    fn las_reservas_del_lote_evitan_planificar_dos_veces_la_misma_ruta() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path();
        let mut reservadas = HashSet::new();
        let a = ruta_libre(d, "kick.wav", &reservadas);
        reservadas.insert(a.clone());
        let b = ruta_libre(d, "kick.wav", &reservadas);
        assert_ne!(
            a, b,
            "dos samples del mismo lote no pueden ir a la misma ruta"
        );
        assert_eq!(b.file_name().unwrap(), "kick (2).wav");
    }

    #[test]
    fn mover_lleva_el_archivo_y_deja_el_origen_vacio() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.wav");
        let b = tmp.path().join("sub/b.wav");
        std::fs::write(&a, b"contenido").unwrap();
        mover(&a, &b).unwrap();
        assert!(!a.exists());
        assert_eq!(std::fs::read(&b).unwrap(), b"contenido");
    }

    #[test]
    fn mover_se_niega_a_sobrescribir() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.wav");
        let b = tmp.path().join("b.wav");
        std::fs::write(&a, b"origen").unwrap();
        std::fs::write(&b, b"no me pises").unwrap();
        assert!(mover(&a, &b).is_err());
        assert_eq!(std::fs::read(&b).unwrap(), b"no me pises");
        assert!(
            a.exists(),
            "el origen no se toca si la operación no se hace"
        );
    }

    #[test]
    fn entre_dispositivos_copia_verifica_y_solo_entonces_borra() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.wav");
        let b = tmp.path().join("b.wav");
        let contenido: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&a, &contenido).unwrap();
        mover_entre_dispositivos(&a, &b).unwrap();
        assert!(!a.exists(), "el origen se borra solo después de verificar");
        assert_eq!(std::fs::read(&b).unwrap(), contenido);
    }

    #[test]
    fn mover_algo_que_no_existe_falla_sin_efectos() {
        let tmp = tempfile::tempdir().unwrap();
        let r = mover(&tmp.path().join("fantasma.wav"), &tmp.path().join("x.wav"));
        assert!(r.is_err());
        assert!(!tmp.path().join("x.wav").exists());
    }
}
