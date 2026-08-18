//! Comparar dos etiquetas no es `==`.
//!
//! Decir 87 donde eran 174 no es lo mismo que decir 103: el primero acertó el pulso y falló la
//! octava, y se arregla con una regla; el segundo no se enteró de nada. Igual con la tonalidad:
//! confundir Do mayor con La menor es el error clásico —comparten las mismas notas— y merece
//! contarse aparte de confundirlo con Fa sostenido mayor.
//!
//! Mezclar ambos casos en un único porcentaje esconde justo la información que sirve para
//! mejorar el estimador.

use crate::domain::KeyLabel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Veredicto {
    Exacto,
    /// Cerca, con el nombre del parentesco: `octava`, `relativo`, `paralelo`, `quinta`.
    Cercano(&'static str),
    Fallo,
}

impl Veredicto {
    pub fn es_acierto(self) -> bool {
        matches!(self, Self::Exacto)
    }
    pub fn etiqueta(self) -> &'static str {
        match self {
            Self::Exacto => "exacto",
            Self::Cercano(q) => q,
            Self::Fallo => "fallo",
        }
    }
}

/// Peso al estilo MIREX para la tonalidad: un relativo no vale lo mismo que un acierto, pero
/// tampoco lo mismo que un disparate.
pub fn peso_mirex(v: Veredicto) -> f64 {
    match v {
        Veredicto::Exacto => 1.0,
        Veredicto::Cercano("quinta") => 0.5,
        Veredicto::Cercano("relativo") => 0.3,
        Veredicto::Cercano("paralelo") => 0.2,
        Veredicto::Cercano(_) => 0.3,
        Veredicto::Fallo => 0.0,
    }
}

pub fn comparar(field: &str, verdad: &str, propuesta: &str) -> Veredicto {
    match field {
        "bpm" => comparar_bpm(verdad, propuesta),
        "key" => comparar_tonalidad(verdad, propuesta),
        // tipo y altura: o aciertas o no.
        _ => {
            if verdad.eq_ignore_ascii_case(propuesta) {
                Veredicto::Exacto
            } else {
                Veredicto::Fallo
            }
        }
    }
}

fn comparar_bpm(verdad: &str, propuesta: &str) -> Veredicto {
    let (Ok(a), Ok(b)) = (verdad.parse::<f64>(), propuesta.parse::<f64>()) else {
        return Veredicto::Fallo;
    };
    if a <= 0.0 || b <= 0.0 {
        return Veredicto::Fallo;
    }
    if (a - b).abs() <= 1.0 {
        return Veredicto::Exacto;
    }
    // Errores de octava (mitad o doble) y el de tresillo (×1,5), que también es habitual.
    for factor in [2.0, 0.5, 1.5, 2.0 / 3.0, 4.0, 0.25] {
        if (a * factor - b).abs() <= 1.0_f64.max(a * factor * 0.01) {
            return Veredicto::Cercano("octava");
        }
    }
    Veredicto::Fallo
}

fn comparar_tonalidad(verdad: &str, propuesta: &str) -> Veredicto {
    let (Some(a), Some(b)) = (KeyLabel::parse(verdad), KeyLabel::parse(propuesta)) else {
        return Veredicto::Fallo;
    };
    if a == b {
        Veredicto::Exacto
    } else if a.es_relativo_de(b) {
        Veredicto::Cercano("relativo")
    } else if a.es_paralelo_de(b) {
        Veredicto::Cercano("paralelo")
    } else if a.es_quinta_de(b) {
        Veredicto::Cercano("quinta")
    } else {
        Veredicto::Fallo
    }
}

/// Resumen de una comparación entre dos conjuntos de etiquetas.
#[derive(Debug, Clone, Default)]
pub struct Resumen {
    pub pares: i64,
    pub exactos: i64,
    pub cercanos: i64,
    pub fallos: i64,
    pub mirex: f64,
}

impl Resumen {
    pub fn anotar(&mut self, v: Veredicto) {
        self.pares += 1;
        self.mirex += peso_mirex(v);
        match v {
            Veredicto::Exacto => self.exactos += 1,
            Veredicto::Cercano(_) => self.cercanos += 1,
            Veredicto::Fallo => self.fallos += 1,
        }
    }
    pub fn acierto(&self) -> f64 {
        if self.pares == 0 {
            0.0
        } else {
            self.exactos as f64 / self.pares as f64
        }
    }
    pub fn mirex_medio(&self) -> f64 {
        if self.pares == 0 {
            0.0
        } else {
            self.mirex / self.pares as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_bpm_admite_un_punto_de_margen() {
        assert_eq!(comparar("bpm", "128", "128"), Veredicto::Exacto);
        assert_eq!(comparar("bpm", "128", "128.6"), Veredicto::Exacto);
        assert_eq!(comparar("bpm", "128", "131"), Veredicto::Fallo);
    }

    #[test]
    fn el_error_de_octava_se_cuenta_aparte_y_no_como_acierto() {
        assert_eq!(comparar("bpm", "174", "87"), Veredicto::Cercano("octava"));
        assert_eq!(comparar("bpm", "87", "174"), Veredicto::Cercano("octava"));
        assert_eq!(
            comparar("bpm", "120", "180"),
            Veredicto::Cercano("octava"),
            "tresillo"
        );
        assert!(
            !comparar("bpm", "174", "87").es_acierto(),
            "cerca no es acertar"
        );
    }

    #[test]
    fn la_tonalidad_distingue_los_parentescos() {
        assert_eq!(comparar("key", "A:min", "A:min"), Veredicto::Exacto);
        assert_eq!(
            comparar("key", "A:min", "C:maj"),
            Veredicto::Cercano("relativo")
        );
        assert_eq!(
            comparar("key", "C:maj", "C:min"),
            Veredicto::Cercano("paralelo")
        );
        assert_eq!(
            comparar("key", "C:maj", "G:maj"),
            Veredicto::Cercano("quinta")
        );
        assert_eq!(comparar("key", "C:maj", "F#:maj"), Veredicto::Fallo);
    }

    #[test]
    fn el_peso_mirex_ordena_los_errores_por_gravedad() {
        assert!(peso_mirex(Veredicto::Exacto) > peso_mirex(Veredicto::Cercano("quinta")));
        assert!(
            peso_mirex(Veredicto::Cercano("quinta")) > peso_mirex(Veredicto::Cercano("relativo"))
        );
        assert!(peso_mirex(Veredicto::Cercano("paralelo")) > peso_mirex(Veredicto::Fallo));
    }

    #[test]
    fn el_tipo_no_admite_medias_tintas() {
        assert_eq!(comparar("kind", "kick", "kick"), Veredicto::Exacto);
        assert_eq!(comparar("kind", "kick", "snare"), Veredicto::Fallo);
        assert_eq!(
            comparar("kind", "hat", "cymbal"),
            Veredicto::Fallo,
            "parecidos, pero distintos"
        );
    }

    #[test]
    fn el_resumen_separa_acierto_de_puntuacion_ponderada() {
        let mut r = Resumen::default();
        r.anotar(Veredicto::Exacto);
        r.anotar(Veredicto::Cercano("relativo"));
        r.anotar(Veredicto::Fallo);
        r.anotar(Veredicto::Exacto);
        assert_eq!(r.pares, 4);
        assert_eq!(r.exactos, 2);
        assert!((r.acierto() - 0.5).abs() < 1e-9);
        // 1 + 0,3 + 0 + 1 = 2,3 sobre 4
        assert!((r.mirex_medio() - 0.575).abs() < 1e-9);
    }

    #[test]
    fn una_etiqueta_ilegible_es_un_fallo_y_no_un_panico() {
        assert_eq!(comparar("bpm", "no-es-un-numero", "128"), Veredicto::Fallo);
        assert_eq!(comparar("key", "X:mayor", "A:min"), Veredicto::Fallo);
    }
}
