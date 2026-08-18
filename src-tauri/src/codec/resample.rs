//! Remuestreo con sinc enventanado y tabla polifásica.
//!
//! Se ejecuta una sola vez al cargar un sample, en el hilo de control, nunca en el callback.
//! El caso real es 48.000 ↔ 44.100: la interpolación lineal deja aliasing audible en material
//! brillante (hats, ruido), así que se usa un sinc de 32 taps con ventana de Blackman.
//!
//! La tabla polifásica evita llamar a `sin()` millones de veces: se precalculan `FASES`
//! desplazamientos del núcleo y luego el bucle interior es solo multiplicar y sumar.

use super::AudioBuffer;

const TAPS: usize = 32;
const FASES: usize = 512;

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 {
        1.0
    } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}

/// Ventana de Blackman sobre [-mitad, mitad].
fn blackman(x: f64, mitad: f64) -> f64 {
    if x.abs() > mitad {
        return 0.0;
    }
    let t = std::f64::consts::PI * x / mitad;
    0.42 + 0.5 * t.cos() + 0.08 * (2.0 * t).cos()
}

struct Nucleo {
    /// FASES × TAPS coeficientes ya normalizados.
    tabla: Vec<f32>,
}

impl Nucleo {
    fn nuevo(ratio: f64) -> Self {
        // Al bajar la frecuencia hay que filtrar por debajo del nuevo Nyquist o entra aliasing.
        let corte = 0.475 * ratio.min(1.0);
        let mitad = TAPS as f64 / 2.0;
        let mut tabla = vec![0.0f32; FASES * TAPS];

        for fase in 0..FASES {
            let frac = fase as f64 / FASES as f64;
            let inicio = fase * TAPS;
            let mut suma = 0.0f64;
            for k in 0..TAPS {
                // distancia, en muestras de entrada, entre el tap y el punto que se interpola
                let x = (k as f64 - mitad + 1.0) - frac;
                let w = blackman(x, mitad);
                let v = 2.0 * corte * sinc(2.0 * corte * x) * w;
                tabla[inicio + k] = v as f32;
                suma += v;
            }
            // Normalizar cada fase a ganancia unidad en continua: sin esto el nivel oscila
            // ligeramente según la fracción, y eso se oye como un temblor de volumen.
            if suma.abs() > 1e-9 {
                for k in 0..TAPS {
                    tabla[inicio + k] = (tabla[inicio + k] as f64 / suma) as f32;
                }
            }
        }
        Self { tabla }
    }
}

pub fn resample(entrada: &AudioBuffer, target_sr: u32) -> AudioBuffer {
    let ch = entrada.channels.max(1) as usize;
    let frames_in = entrada.frames();
    if frames_in == 0 || entrada.sample_rate == 0 || entrada.sample_rate == target_sr {
        return AudioBuffer {
            samples: entrada.samples.clone(),
            channels: entrada.channels,
            sample_rate: target_sr.max(entrada.sample_rate),
            bit_depth: entrada.bit_depth,
        };
    }

    let ratio = target_sr as f64 / entrada.sample_rate as f64;
    let frames_out = ((frames_in as f64) * ratio).round() as usize;
    let nucleo = Nucleo::nuevo(ratio);
    let mitad = (TAPS / 2) as i64;

    let mut out = vec![0.0f32; frames_out * ch];
    for (f, marco) in out.chunks_mut(ch).enumerate() {
        let pos = f as f64 / ratio;
        let base = pos.floor() as i64;
        let frac = pos - base as f64;
        let fase = ((frac * FASES as f64) as usize).min(FASES - 1);
        let coef = &nucleo.tabla[fase * TAPS..(fase + 1) * TAPS];

        for (c, muestra) in marco.iter_mut().enumerate() {
            let mut acc = 0.0f32;
            for (k, w) in coef.iter().enumerate() {
                let idx = base + k as i64 - mitad + 1;
                if idx < 0 || idx as usize >= frames_in {
                    continue;
                }
                acc += entrada.samples[idx as usize * ch + c] * w;
            }
            *muestra = acc;
        }
    }

    AudioBuffer {
        samples: out,
        channels: entrada.channels,
        sample_rate: target_sr,
        bit_depth: entrada.bit_depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seno(sr: u32, freq: f32, dur: f32) -> AudioBuffer {
        let frames = (sr as f32 * dur) as usize;
        let mut s = vec![0.0f32; frames];
        for (f, v) in s.iter_mut().enumerate() {
            *v = (std::f32::consts::TAU * freq * f as f32 / sr as f32).sin();
        }
        AudioBuffer {
            samples: s,
            channels: 1,
            sample_rate: sr,
            bit_depth: Some(16),
        }
    }

    /// Error cuadrático medio contra el seno generado analíticamente a la frecuencia destino.
    fn error_rms(salida: &[f32], sr: u32, freq: f32, desde: usize, hasta: usize) -> f64 {
        let fin = hasta.min(salida.len());
        let mut suma = 0.0f64;
        let mut n = 0usize;
        for (f, v) in salida.iter().enumerate().take(fin).skip(desde) {
            let esperado = (std::f32::consts::TAU * freq * f as f32 / sr as f32).sin();
            let d = (*v - esperado) as f64;
            suma += d * d;
            n += 1;
        }
        (suma / n.max(1) as f64).sqrt()
    }

    #[test]
    fn de_48k_a_44k1_reconstruye_el_seno() {
        let entrada = seno(48_000, 1_000.0, 0.5);
        let salida = resample(&entrada, 44_100);
        assert_eq!(salida.sample_rate, 44_100);
        // duración preservada dentro de un frame
        assert!(
            (salida.frames() as i64 - 22_050).abs() <= 2,
            "frames: {}",
            salida.frames()
        );
        // se ignoran los bordes, donde el núcleo se sale del buffer
        let e = error_rms(&salida.samples, 44_100, 1_000.0, 64, salida.frames() - 64);
        assert!(e < 0.01, "error RMS demasiado alto: {e}");
    }

    #[test]
    fn de_44k1_a_48k_reconstruye_el_seno() {
        let entrada = seno(44_100, 1_000.0, 0.5);
        let salida = resample(&entrada, 48_000);
        let e = error_rms(&salida.samples, 48_000, 1_000.0, 64, salida.frames() - 64);
        assert!(e < 0.01, "error RMS demasiado alto: {e}");
    }

    #[test]
    fn no_hay_deriva_de_nivel_entre_fases() {
        // Una continua debe salir como la misma continua, sea cual sea la fase del núcleo.
        let entrada = AudioBuffer {
            samples: vec![0.5f32; 4096],
            channels: 1,
            sample_rate: 48_000,
            bit_depth: None,
        };
        let salida = resample(&entrada, 44_100);
        let centro = &salida.samples[64..salida.frames() - 64];
        let max = centro.iter().cloned().fold(f32::MIN, f32::max);
        let min = centro.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            (max - 0.5).abs() < 0.002 && (min - 0.5).abs() < 0.002,
            "el nivel oscila entre {min} y {max}; debería quedarse en 0,5"
        );
    }

    #[test]
    fn el_estereo_mantiene_los_canales_separados() {
        let frames = 4_800;
        let mut s = vec![0.0f32; frames * 2];
        for f in 0..frames {
            s[f * 2] = 0.8;
            s[f * 2 + 1] = -0.4;
        }
        let entrada = AudioBuffer {
            samples: s,
            channels: 2,
            sample_rate: 48_000,
            bit_depth: None,
        };
        let salida = resample(&entrada, 44_100);
        let f = salida.frames() / 2;
        assert!((salida.samples[f * 2] - 0.8).abs() < 0.002);
        assert!((salida.samples[f * 2 + 1] + 0.4).abs() < 0.002);
    }
}
