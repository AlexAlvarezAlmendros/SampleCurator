//! Lo que el nombre del archivo ya nos está diciendo.
//!
//! En una librería de samples el nombre suele traer la respuesta escrita: `KICK_808_deep.wav`,
//! `Loop_128_Amin.wav`, `vox_chop_F#m_92.wav`. Es la referencia más barata que existe y la
//! primera de las tres señales del BPM.
//!
//! Dos avisos que conviene tener presentes al leer los números que salgan de aquí:
//!
//! 1. **El nombre miente a veces.** Un pack mal etiquetado, un `128` que era el número de pack
//!    o un `Am` que era el principio de «Ambient». Por eso esto es una referencia *débil*, y
//!    la tarea 8.0.7 existe para medir exactamente cuánto miente.
//! 2. **La carpeta cuenta, y mucho.** `pack_03/kicks/xyz.wav` no dice nada en el nombre pero
//!    lo dice todo en la ruta: la estructura de carpetas ES la clasificación que hizo el autor
//!    del pack. Por eso los tokens de carpeta pesan más que los del nombre.
//!
//! Ojo con evaluar esto sobre la biblioteca sintética: sus carpetas se llaman exactamente como
//! las clases, así que acertaría el 100 % sin demostrar nada. Solo vale medirlo sobre packs
//! reales.

use crate::domain::{KeyLabel, KeyMode, SampleKind};

/// Algo leído del nombre, con la confianza que merece.
type Lectura<T> = Option<(T, f32)>;

/// Lo que se ha podido leer del nombre, cada cosa con su confianza de 0 a 1.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pistas {
    pub kind: Option<(SampleKind, f32)>,
    pub bpm: Option<(f64, f32)>,
    pub key: Option<(KeyLabel, f32)>,
    /// Clase de altura (0 = Do) cuando el nombre trae una nota suelta como `F#` pero no dice
    /// el modo. Un 808 en fa sostenido es información útil; llamarlo «fa sostenido mayor»
    /// sería inventarse la mitad.
    pub pitch: Option<(i64, f32)>,
}

const BPM_MIN: f64 = 60.0;
const BPM_MAX: f64 = 200.0;

/// Palabras que delatan el tipo, con su peso. Las abreviaturas cortas pesan menos porque
/// chocan con demasiadas cosas: `ch` es «closed hat» pero también el principio de «chord».
const VOCABULARIO: &[(&str, SampleKind, f32)] = &[
    // bombo
    ("kick", SampleKind::Kick, 1.0),
    ("kicks", SampleKind::Kick, 1.0),
    ("kik", SampleKind::Kick, 0.8),
    ("bd", SampleKind::Kick, 0.6),
    ("bassdrum", SampleKind::Kick, 1.0),
    ("bombo", SampleKind::Kick, 1.0),
    // caja
    ("snare", SampleKind::Snare, 1.0),
    ("snares", SampleKind::Snare, 1.0),
    ("snr", SampleKind::Snare, 0.8),
    ("sd", SampleKind::Snare, 0.6),
    ("sn", SampleKind::Snare, 0.5),
    ("caja", SampleKind::Snare, 0.8),
    ("rimshot", SampleKind::Snare, 0.8),
    // palmas
    ("clap", SampleKind::Clap, 1.0),
    ("claps", SampleKind::Clap, 1.0),
    ("cp", SampleKind::Clap, 0.5),
    ("handclap", SampleKind::Clap, 1.0),
    ("palmas", SampleKind::Clap, 0.9),
    // charles
    ("hat", SampleKind::Hat, 1.0),
    ("hats", SampleKind::Hat, 1.0),
    ("hihat", SampleKind::Hat, 1.0),
    ("hh", SampleKind::Hat, 0.7),
    ("ch", SampleKind::Hat, 0.5),
    ("oh", SampleKind::Hat, 0.5),
    ("charles", SampleKind::Hat, 0.8),
    // platos
    ("crash", SampleKind::Cymbal, 1.0),
    ("ride", SampleKind::Cymbal, 0.9),
    ("cymbal", SampleKind::Cymbal, 1.0),
    ("cym", SampleKind::Cymbal, 0.7),
    ("splash", SampleKind::Cymbal, 0.9),
    ("china", SampleKind::Cymbal, 0.7),
    ("plato", SampleKind::Cymbal, 0.7),
    // toms
    ("tom", SampleKind::Tom, 1.0),
    ("toms", SampleKind::Tom, 1.0),
    ("timbal", SampleKind::Tom, 0.8),
    // percusión
    ("perc", SampleKind::Perc, 1.0),
    ("percussion", SampleKind::Perc, 1.0),
    ("shaker", SampleKind::Perc, 1.0),
    ("tamb", SampleKind::Perc, 0.9),
    ("tambourine", SampleKind::Perc, 1.0),
    ("conga", SampleKind::Perc, 1.0),
    ("bongo", SampleKind::Perc, 1.0),
    ("cowbell", SampleKind::Perc, 1.0),
    ("rim", SampleKind::Perc, 0.6),
    ("woodblock", SampleKind::Perc, 1.0),
    ("clave", SampleKind::Perc, 0.8),
    // bajo
    ("bass", SampleKind::Bass, 1.0),
    ("sub", SampleKind::Bass, 0.8),
    ("808", SampleKind::Bass, 0.5),
    ("bajo", SampleKind::Bass, 0.8),
    ("reese", SampleKind::Bass, 0.8),
    // sintetizador
    ("synth", SampleKind::Synth, 1.0),
    ("lead", SampleKind::Synth, 0.9),
    ("pad", SampleKind::Synth, 0.9),
    ("stab", SampleKind::Synth, 0.9),
    ("chord", SampleKind::Synth, 0.9),
    ("chords", SampleKind::Synth, 0.9),
    ("arp", SampleKind::Synth, 0.9),
    ("pluck", SampleKind::Synth, 0.9),
    ("keys", SampleKind::Synth, 0.8),
    ("piano", SampleKind::Synth, 0.9),
    // voz
    ("vox", SampleKind::Vocal, 1.0),
    ("vocal", SampleKind::Vocal, 1.0),
    ("vocals", SampleKind::Vocal, 1.0),
    ("voice", SampleKind::Vocal, 0.9),
    ("acapella", SampleKind::Vocal, 1.0),
    ("chant", SampleKind::Vocal, 0.8),
    ("adlib", SampleKind::Vocal, 0.9),
    ("voz", SampleKind::Vocal, 0.8),
    // efectos
    ("fx", SampleKind::Fx, 1.0),
    ("riser", SampleKind::Fx, 1.0),
    ("sweep", SampleKind::Fx, 0.9),
    ("impact", SampleKind::Fx, 0.9),
    ("downlifter", SampleKind::Fx, 1.0),
    ("uplifter", SampleKind::Fx, 1.0),
    ("whoosh", SampleKind::Fx, 1.0),
    ("transition", SampleKind::Fx, 0.8),
    ("noise", SampleKind::Fx, 0.7),
    ("efecto", SampleKind::Fx, 0.7),
    // bucles
    ("loop", SampleKind::Loop, 1.0),
    ("loops", SampleKind::Loop, 1.0),
    ("groove", SampleKind::Loop, 0.9),
    ("break", SampleKind::Loop, 0.8),
    ("beat", SampleKind::Loop, 0.7),
    ("bucle", SampleKind::Loop, 0.9),
];

/// Frases de dos palabras que hay que cazar antes de trocear.
const FRASES: &[(&str, SampleKind, f32)] = &[
    ("bass drum", SampleKind::Kick, 1.0),
    ("kick drum", SampleKind::Kick, 1.0),
    ("hi hat", SampleKind::Hat, 1.0),
    ("open hat", SampleKind::Hat, 1.0),
    ("closed hat", SampleKind::Hat, 1.0),
    ("snare drum", SampleKind::Snare, 1.0),
    ("drum loop", SampleKind::Loop, 1.0),
    ("vocal chop", SampleKind::Vocal, 1.0),
];

/// Trocea la ruta en palabras, separando también letra/dígito (`KICK808` → `kick`, `808`).
fn tokenizar(texto: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut actual = String::new();
    let mut era_digito: Option<bool> = None;

    for c in texto.chars() {
        if c.is_alphanumeric() || c == '#' {
            let es_digito = c.is_ascii_digit();
            if let Some(anterior) = era_digito {
                if anterior != es_digito && c != '#' && !actual.is_empty() {
                    tokens.push(std::mem::take(&mut actual));
                }
            }
            era_digito = Some(es_digito);
            actual.push(c.to_ascii_lowercase());
        } else if !actual.is_empty() {
            tokens.push(std::mem::take(&mut actual));
            era_digito = None;
        }
    }
    if !actual.is_empty() {
        tokens.push(actual);
    }
    tokens
}

/// Los tokens de carpeta pesan más: la estructura de carpetas es la clasificación que hizo
/// el autor del pack, y suele ser más fiable que el nombre del archivo.
const PESO_CARPETA: f32 = 1.2;

fn tipo(rel_path: &str) -> Option<(SampleKind, f32)> {
    let bajo = rel_path.to_ascii_lowercase();
    let mut puntos: Vec<(SampleKind, f32)> = Vec::new();

    for (frase, kind, peso) in FRASES {
        if bajo.contains(frase) {
            puntos.push((*kind, *peso));
        }
    }

    let (carpetas, archivo) = match bajo.rsplit_once('/') {
        Some((c, a)) => (c, a),
        None => ("", bajo.as_str()),
    };

    for (texto, multiplicador) in [(carpetas, PESO_CARPETA), (archivo, 1.0)] {
        for token in tokenizar(texto) {
            for (palabra, kind, peso) in VOCABULARIO {
                if token == *palabra {
                    puntos.push((*kind, peso * multiplicador));
                }
            }
        }
    }

    if puntos.is_empty() {
        return None;
    }

    let mut acumulado: Vec<(SampleKind, f32)> = Vec::new();
    for (kind, peso) in &puntos {
        match acumulado.iter_mut().find(|(k, _)| k == kind) {
            Some((_, p)) => *p += peso,
            None => acumulado.push((*kind, *peso)),
        }
    }
    acumulado.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let total: f32 = acumulado.iter().map(|(_, p)| p).sum();
    let (ganador, peso) = acumulado[0];
    // La confianza es cuánto domina el ganador sobre el resto de pistas del nombre.
    let confianza = (peso / total).clamp(0.0, 1.0) * peso.min(1.2) / 1.2;
    Some((ganador, confianza.clamp(0.05, 0.95)))
}

fn bpm(rel_path: &str) -> Option<(f64, f32)> {
    let tokens = tokenizar(rel_path);
    let mut explicitos: Vec<f64> = Vec::new();
    let mut sueltos: Vec<f64> = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        // "128bpm", "128 bpm", "bpm 128"
        let vecino_es_bpm = tokens
            .get(i + 1)
            .map(|t| t == "bpm" || t == "bpms")
            .unwrap_or(false)
            || (i > 0 && tokens.get(i - 1).map(|t| t == "bpm").unwrap_or(false));

        let Ok(n) = token.parse::<f64>() else {
            continue;
        };
        if !(BPM_MIN..=BPM_MAX).contains(&n) {
            continue;
        }
        // Un `08` con cero delante es un índice, no un tempo.
        if token.len() > 1 && token.starts_with('0') {
            continue;
        }
        if vecino_es_bpm {
            explicitos.push(n);
        } else {
            sueltos.push(n);
        }
    }

    if let Some(n) = explicitos.first() {
        return Some((*n, 0.95));
    }
    match sueltos.len() {
        0 => None,
        // Un único número plausible en todo el nombre: probablemente el tempo, pero podría
        // ser el número del pack.
        1 => Some((sueltos[0], 0.5)),
        // Varios candidatos: se devuelve el primero avisando de que es poco fiable.
        _ => Some((sueltos[0], 0.25)),
    }
}

fn nota_a_clase(letra: char, alteracion: Option<char>) -> Option<i64> {
    let base: i64 = match letra.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let desplazamiento: i64 = match alteracion {
        Some('#') => 1,
        Some('b') => -1,
        _ => 0,
    };
    Some((base + desplazamiento).rem_euclid(12))
}

fn tonalidad(rel_path: &str) -> (Lectura<KeyLabel>, Lectura<i64>) {
    let chars: Vec<char> = rel_path.chars().collect();
    let mut mejor_tono: Lectura<KeyLabel> = None;
    let mut mejor_altura: Lectura<i64> = None;

    for i in 0..chars.len() {
        let letra = chars[i];
        if !"ABCDEFGabcdefg".contains(letra) {
            continue;
        }
        // La nota tiene que empezar palabra: la `a` de «clap» no es un La.
        if i > 0 && chars[i - 1].is_ascii_alphanumeric() {
            continue;
        }

        // Se prueban las dos lecturas posibles y se queda la que encuentre modo. Así
        // `Bb_min` es si bemol menor y `Ableton` no es nada.
        let mut lecturas: Vec<(Option<char>, usize)> = vec![(None, i + 1)];
        match chars.get(i + 1) {
            Some('#') => lecturas.push((Some('#'), i + 2)),
            Some('b') => lecturas.push((Some('b'), i + 2)),
            _ => {}
        }

        for (alteracion, mut j) in lecturas {
            let Some(root) = nota_a_clase(letra, alteracion) else {
                continue;
            };
            // Puede haber un separador entre la nota y el modo: "A_min", "F# maj".
            if matches!(chars.get(j), Some('_' | '-' | ' ')) {
                j += 1;
            }
            let resto: String = chars[j..].iter().collect::<String>().to_ascii_lowercase();
            let siguiente_es_palabra = resto.starts_with(|c: char| c.is_ascii_alphanumeric());

            if resto.starts_with("minor") || resto.starts_with("min") {
                proponer_tono(&mut mejor_tono, root, KeyMode::Minor, 0.9);
            } else if resto.starts_with("major") || resto.starts_with("maj") {
                proponer_tono(&mut mejor_tono, root, KeyMode::Major, 0.9);
            } else if resto.starts_with('m')
                && !resto[1..].starts_with(|c: char| c.is_ascii_alphanumeric())
            {
                // "Am", "F#m": habitual en nombres de packs, algo menos seguro.
                proponer_tono(&mut mejor_tono, root, KeyMode::Minor, 0.75);
            } else if !siguiente_es_palabra {
                // Una nota suelta ("F#", "_C_") es una ALTURA, no una tonalidad. Inventarle
                // un modo sería el tipo de mentira que hace que el usuario deje de fiarse
                // de la columna entera.
                let confianza = if alteracion.is_some() { 0.35 } else { 0.2 };
                if mejor_altura.as_ref().is_none_or(|(_, c)| confianza > *c) {
                    mejor_altura = Some((root, confianza));
                }
            }
        }
    }
    (mejor_tono, mejor_altura)
}

fn proponer_tono(mejor: &mut Lectura<KeyLabel>, root: i64, mode: KeyMode, confianza: f32) {
    if mejor.as_ref().is_none_or(|(_, c)| confianza > *c) {
        *mejor = Some((KeyLabel { root, mode }, confianza));
    }
}

/// Lee todas las pistas de una ruta relativa (con sus carpetas, que cuentan).
pub fn leer(rel_path: &str) -> Pistas {
    let (key, pitch) = tonalidad(rel_path);
    Pistas {
        kind: tipo(rel_path),
        bpm: bpm(rel_path),
        key,
        pitch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind_de(ruta: &str) -> Option<SampleKind> {
        leer(ruta).kind.map(|(k, _)| k)
    }
    fn bpm_de(ruta: &str) -> Option<f64> {
        leer(ruta).bpm.map(|(b, _)| b)
    }
    fn key_de(ruta: &str) -> Option<String> {
        leer(ruta).key.map(|(k, _)| k.as_str())
    }

    // ── tipo ──────────────────────────────────────────────────

    #[test]
    fn lee_los_tipos_habituales_de_un_pack() {
        assert_eq!(kind_de("KICK_808_deep.wav"), Some(SampleKind::Kick));
        assert_eq!(kind_de("Snare_acoustic_02.wav"), Some(SampleKind::Snare));
        assert_eq!(kind_de("HH_closed_01.wav"), Some(SampleKind::Hat));
        assert_eq!(kind_de("Clap_layered.wav"), Some(SampleKind::Clap));
        assert_eq!(kind_de("Crash_long.wav"), Some(SampleKind::Cymbal));
        assert_eq!(kind_de("shaker_16th.wav"), Some(SampleKind::Perc));
        assert_eq!(kind_de("Riser_8bar.wav"), Some(SampleKind::Fx));
        assert_eq!(kind_de("vox_chop_dry.wav"), Some(SampleKind::Vocal));
    }

    #[test]
    fn la_carpeta_manda_cuando_el_nombre_no_dice_nada() {
        // La estructura de carpetas ES la clasificación que hizo el autor del pack.
        assert_eq!(
            kind_de("VengeancePack/kicks/vpe_0231.wav"),
            Some(SampleKind::Kick)
        );
        assert_eq!(kind_de("pack_03/hats/02.wav"), Some(SampleKind::Hat));
        assert_eq!(kind_de("sin_pistas/archivo_1234.wav"), None);
    }

    #[test]
    fn la_carpeta_pesa_mas_que_el_nombre_cuando_se_contradicen() {
        // Un archivo suelto llamado "kick" dentro de la carpeta de snares suele ser un nombre
        // heredado; el autor lo colocó en snares a propósito.
        assert_eq!(
            kind_de("pack/snares/kick_ish_layer.wav"),
            Some(SampleKind::Snare)
        );
    }

    #[test]
    fn las_frases_de_dos_palabras_se_cazan_enteras() {
        assert_eq!(kind_de("Bass Drum 01.wav"), Some(SampleKind::Kick));
        assert_eq!(kind_de("Hi Hat Open.wav"), Some(SampleKind::Hat));
        assert_eq!(kind_de("Vocal Chop Wet.wav"), Some(SampleKind::Vocal));
    }

    #[test]
    fn las_abreviaturas_cortas_no_se_disparan_dentro_de_otra_palabra() {
        // "ch" es closed hat, pero "chord" es un sintetizador.
        assert_eq!(kind_de("chord_stab_01.wav"), Some(SampleKind::Synth));
        // "sn" es snare, pero "snap" no debería contar como tal por dentro.
        assert_eq!(kind_de("finger_snap.wav"), None);
    }

    #[test]
    fn el_808_se_lee_como_bajo_pero_sin_alardear() {
        let p = leer("808_slide_F#.wav");
        assert_eq!(p.kind.map(|(k, _)| k), Some(SampleKind::Bass));
        assert!(
            p.kind.unwrap().1 < 0.8,
            "«808» es ambiguo (bajo o bombo): no puede salir con confianza alta"
        );
    }

    // ── BPM ───────────────────────────────────────────────────

    #[test]
    fn el_bpm_explicito_va_con_confianza_alta() {
        for ruta in ["Loop_128bpm.wav", "Loop 128 BPM.wav", "loop_bpm_128.wav"] {
            let p = leer(ruta);
            assert_eq!(p.bpm.map(|(b, _)| b), Some(128.0), "en {ruta}");
            assert!(p.bpm.unwrap().1 > 0.9, "en {ruta}");
        }
    }

    #[test]
    fn un_numero_suelto_plausible_se_acepta_con_reservas() {
        let p = leer("Drumloop_174_dnb.wav");
        assert_eq!(p.bpm.map(|(b, _)| b), Some(174.0));
        let c = p.bpm.unwrap().1;
        assert!((0.4..0.7).contains(&c), "confianza media, no alta: {c}");
    }

    #[test]
    fn los_indices_no_son_tempos() {
        assert_eq!(
            bpm_de("Kick 01.wav"),
            None,
            "un 01 con cero delante es un índice"
        );
        assert_eq!(bpm_de("Snare_08.wav"), None);
        assert_eq!(bpm_de("Hat_04_dry.wav"), None);
    }

    #[test]
    fn los_numeros_que_no_son_tempos_se_descartan_por_rango() {
        assert_eq!(bpm_de("808_kick.wav"), None, "808 no es un tempo");
        assert_eq!(bpm_de("909_snare.wav"), None);
        assert_eq!(
            bpm_de("sample_44100_24bit.wav"),
            None,
            "ni la frecuencia ni los bits"
        );
        assert_eq!(
            bpm_de("kick_16.wav"),
            None,
            "16 está por debajo del rango musical"
        );
    }

    #[test]
    fn con_varios_candidatos_avisa_de_que_no_se_fia() {
        let p = leer("Pack_120/loop_140_dry.wav");
        assert!(p.bpm.is_some());
        assert!(
            p.bpm.unwrap().1 < 0.3,
            "dos números plausibles: la confianza tiene que desplomarse"
        );
    }

    // ── tonalidad y altura ────────────────────────────────────

    #[test]
    fn lee_las_tonalidades_escritas_de_las_formas_habituales() {
        assert_eq!(key_de("Loop_Amin_128.wav"), Some("A:min".into()));
        assert_eq!(key_de("Loop_A_min_128.wav"), Some("A:min".into()));
        assert_eq!(key_de("Bass_Cmaj_140bpm.wav"), Some("C:maj".into()));
        assert_eq!(key_de("pad_F#m_slow.wav"), Some("F#:min".into()));
        assert_eq!(
            key_de("Chords_Bbmin.wav"),
            Some("A#:min".into()),
            "si bemol = la sostenido"
        );
        assert_eq!(key_de("stab_Dminor.wav"), Some("D:min".into()));
        assert_eq!(key_de("lead_Gmajor.wav"), Some("G:maj".into()));
    }

    #[test]
    fn no_inventa_tonalidades_dentro_de_palabras_normales() {
        // Estos son los falsos positivos que arruinarían la columna entera.
        assert_eq!(
            key_de("Ambient_pad_long.wav"),
            None,
            "«Ambient» no es la menor"
        );
        assert_eq!(key_de("Clap_layered.wav"), None);
        assert_eq!(key_de("Ableton_export.wav"), None);
        assert_eq!(key_de("Fmaster_bus.wav"), None, "«Fmaster» no es fa mayor");
        assert_eq!(
            key_de("Emin_ent_riser.wav"),
            Some("E:min".into()),
            "pero «Emin» sí lo es"
        );
    }

    #[test]
    fn una_nota_suelta_es_una_altura_y_no_una_tonalidad() {
        let p = leer("808_slide_F#.wav");
        assert_eq!(p.key, None, "no se le puede inventar el modo");
        assert_eq!(p.pitch.map(|(n, _)| n), Some(6), "fa sostenido = clase 6");
        assert!(p.pitch.unwrap().1 < 0.5);
    }

    #[test]
    fn una_ruta_completa_de_pack_se_lee_entera() {
        let p = leer("Packs/Deep House Vol 2/loops/Drumloop_124bpm_Amin.wav");
        assert_eq!(p.kind.map(|(k, _)| k), Some(SampleKind::Loop));
        assert_eq!(p.bpm.map(|(b, _)| b), Some(124.0));
        assert_eq!(p.key.map(|(k, _)| k.as_str()), Some("A:min".into()));
    }

    #[test]
    fn un_nombre_sin_pistas_no_devuelve_nada() {
        let p = leer("audio_export_final_v3.wav");
        assert_eq!(p.kind, None);
        assert_eq!(p.bpm, None);
        assert_eq!(p.key, None);
    }

    // ── utilidades del dominio que esto ejercita ──────────────

    #[test]
    fn las_relaciones_entre_tonalidades_estan_bien() {
        let do_may = KeyLabel::parse("C:maj").unwrap();
        let la_men = KeyLabel::parse("A:min").unwrap();
        let do_men = KeyLabel::parse("C:min").unwrap();
        let sol_may = KeyLabel::parse("G:maj").unwrap();

        assert!(
            do_may.es_relativo_de(la_men),
            "Do mayor y La menor son relativos"
        );
        assert!(la_men.es_relativo_de(do_may));
        assert!(
            do_may.es_paralelo_de(do_men),
            "Do mayor y Do menor son paralelos"
        );
        assert!(do_may.es_quinta_de(sol_may), "Do y Sol están a una quinta");
        assert!(!do_may.es_relativo_de(sol_may));
    }

    #[test]
    fn la_taxonomia_sabe_a_quien_tiene_sentido_preguntarle() {
        assert!(
            !SampleKind::Kick.puede_ser_tonal(),
            "un bombo no tiene nota"
        );
        assert!(
            !SampleKind::Hat.puede_tener_bpm(),
            "un charles no tiene tempo"
        );
        assert!(SampleKind::Bass.puede_ser_tonal());
        assert!(SampleKind::Loop.puede_tener_bpm());
    }
}
