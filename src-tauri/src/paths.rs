//! Rutas relativas normalizadas.
//!
//! El índice guarda siempre `kicks/snare.wav`, con barra hacia delante, también en Windows.
//! Para abrir el archivo da igual —las APIs de Windows aceptan `/` sin rechistar— pero hace
//! que todo lo que compara cadenas funcione igual en los dos sistemas: podar lo que ya no
//! está en disco, la búsqueda FTS, el `library.json` de la copia de seguridad y el `UNIQUE`
//! de `(source_id, rel_path)`.
//!
//! Sin esto, la misma librería indexada en Linux y en Windows daría rutas distintas para el
//! mismo archivo.

use std::path::Path;

pub fn relativa(base: &Path, completa: &Path) -> Option<String> {
    let rel = completa.strip_prefix(base).ok()?;
    let mut salida = String::with_capacity(rel.as_os_str().len());
    for (i, parte) in rel.components().enumerate() {
        if i > 0 {
            salida.push('/');
        }
        salida.push_str(&parte.as_os_str().to_string_lossy());
    }
    Some(salida)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normaliza_una_ruta_de_varios_niveles() {
        let base = PathBuf::from("/musica/packs");
        let completa = base.join("deep").join("kicks").join("k1.wav");
        assert_eq!(
            relativa(&base, &completa).as_deref(),
            Some("deep/kicks/k1.wav")
        );
    }

    #[test]
    fn un_archivo_en_la_raiz_no_lleva_separador() {
        let base = PathBuf::from("/musica");
        assert_eq!(
            relativa(&base, &base.join("k.wav")).as_deref(),
            Some("k.wav")
        );
    }

    #[test]
    fn fuera_de_la_base_no_hay_ruta_relativa() {
        assert_eq!(relativa(Path::new("/a"), Path::new("/b/c.wav")), None);
    }

    #[test]
    fn los_espacios_y_los_acentos_sobreviven() {
        let base = PathBuf::from("/música");
        let completa = base.join("Pack Nº 2").join("kick á.wav");
        assert_eq!(
            relativa(&base, &completa).as_deref(),
            Some("Pack Nº 2/kick á.wav")
        );
    }
}
