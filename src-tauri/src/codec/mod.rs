//! Códec: decodificar, remuestrear y describir audio. Lo usan `audio` (para reproducir) y
//! `scan` (para analizar), así que vive aparte de los dos.
//!
//! Nada de aquí se ejecuta en el callback de tiempo real: todo pasa por el hilo de control.

pub mod resample;

use crate::error::{AppError, Result};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const EXTENSIONES: [&str; 9] = [
    "wav", "wave", "aiff", "aif", "flac", "mp3", "ogg", "m4a", "aac",
];

pub fn es_audio(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTENSIONES.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Audio ya decodificado: f32 intercalado. Se comparte con el hilo de audio dentro de un `Arc`.
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
    pub bit_depth: Option<u32>,
}

impl AudioBuffer {
    pub fn frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }
    pub fn duration_ms(&self) -> i64 {
        if self.sample_rate == 0 {
            0
        } else {
            (self.frames() as f64 * 1000.0 / self.sample_rate as f64).round() as i64
        }
    }
    pub fn bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<f32>()
    }
}

pub fn decode(path: &Path) -> Result<AudioBuffer> {
    let fallo = |e: String| AppError::Decode {
        path: path.to_path_buf(),
        reason: e,
    };

    let archivo = std::fs::File::open(path).map_err(|e| fallo(e.to_string()))?;
    let mss = MediaSourceStream::new(Box::new(archivo), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| fallo(e.to_string()))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| fallo("el archivo no tiene pista de audio".into()))?;
    let track_id = track.id;
    let bit_depth = track.codec_params.bits_per_sample;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| fallo(e.to_string()))?;

    let mut samples: Vec<f32> = Vec::with_capacity(1 << 16);
    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut buf: Option<SampleBuffer<f32>> = None;

    // Al acabarse el stream (o al cortarse) `next_packet` devuelve Err: lo decodificado
    // hasta ese punto sigue valiendo, así que se sale del bucle sin considerarlo un fallo.
    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                if sample_rate == 0 {
                    sample_rate = spec.rate;
                    channels = spec.channels.count() as u16;
                }
                let sb = buf.get_or_insert_with(|| {
                    SampleBuffer::<f32>::new(decoded.capacity() as u64, spec)
                });
                sb.copy_interleaved_ref(decoded);
                samples.extend_from_slice(sb.samples());
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }

    if samples.is_empty() || channels == 0 || sample_rate == 0 {
        return Err(fallo("no se pudo decodificar ninguna muestra".into()));
    }
    Ok(AudioBuffer {
        samples,
        channels,
        sample_rate,
        bit_depth,
    })
}

/// Decodifica y deja el audio listo para el device: a su frecuencia, sin más trabajo pendiente.
pub fn decode_at(path: &Path, target_sr: u32) -> Result<AudioBuffer> {
    let buf = decode(path)?;
    if buf.sample_rate == target_sr || target_sr == 0 {
        return Ok(buf);
    }
    Ok(resample::resample(&buf, target_sr))
}

// ─────────────────────────── descripción del audio ───────────────────────────

pub const BUCKETS: usize = 1000;

/// Picos min/max por bucket, 2 bytes por bucket. Es exactamente el BLOB que se guarda y
/// exactamente lo que el canvas del frontend recibe como bytes crudos.
pub fn peaks(samples: &[f32], channels: u16, buckets: usize) -> Vec<u8> {
    let ch = channels.max(1) as usize;
    let frames = samples.len() / ch;
    let mut out = vec![0u8; buckets * 2];
    if frames == 0 {
        return out;
    }
    let por_bucket = (frames as f64 / buckets as f64).max(1.0);
    for b in 0..buckets {
        let desde = (b as f64 * por_bucket) as usize;
        let hasta = (((b + 1) as f64 * por_bucket) as usize).min(frames);
        let (mut mn, mut mx) = (0.0f32, 0.0f32);
        for f in desde..hasta {
            for c in 0..ch {
                let v = samples.get(f * ch + c).copied().unwrap_or(0.0);
                if v < mn {
                    mn = v;
                }
                if v > mx {
                    mx = v;
                }
            }
        }
        out[b * 2] = (mn.clamp(-1.0, 1.0) * 127.0) as i8 as u8;
        out[b * 2 + 1] = (mx.clamp(-1.0, 1.0) * 127.0) as i8 as u8;
    }
    out
}

/// RMS integrado en dB. Sirve para igualar el volumen de escucha entre samples.
pub fn loudness_db(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return -120.0;
    }
    let suma: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    let rms = (suma / samples.len() as f64).sqrt();
    if rms <= 1e-9 {
        -120.0
    } else {
        20.0 * rms.log10()
    }
}

/// Hash del contenido sonoro, no del archivo.
///
/// Detecta lo que de verdad puede detectar un hash exacto: **el mismo PCM en distinto
/// envoltorio**. El mismo kick como WAV y como FLAC, o con metadatos distintos, o duplicado
/// con otro nombre, da el mismo hash. También iguala una versión estéreo y su mezcla a mono.
///
/// Lo que NO detecta, y no puede: dos exportaciones del mismo master con dither distinto, o
/// con una diferencia de nivel de 0,1 dB. Eso es parecido perceptual y necesita una huella
/// acústica, no un hash (Fase 5).
///
/// Se mezcla a mono y se cuantiza a 16 bits con redondeo antes de hashear: así las diferencias
/// de profundidad de bits que sean estrictamente truncamiento sí coinciden.
pub fn content_hash(samples: &[f32], channels: u16) -> [u8; 32] {
    let ch = channels.max(1) as usize;
    let frames = samples.len() / ch;
    let mut hasher = blake3::Hasher::new();
    let mut bloque = Vec::with_capacity(4096);
    for f in 0..frames {
        let mut acc = 0.0f32;
        for c in 0..ch {
            acc += samples.get(f * ch + c).copied().unwrap_or(0.0);
        }
        let v = ((acc / ch as f32).clamp(-1.0, 1.0) * 32767.0).round() as i16;
        bloque.extend_from_slice(&v.to_le_bytes());
        if bloque.len() >= 4096 {
            hasher.update(&bloque);
            bloque.clear();
        }
    }
    if !bloque.is_empty() {
        hasher.update(&bloque);
    }
    *hasher.finalize().as_bytes()
}

/// Dónde empezar a reproducir. En un loop de cuatro compases lo interesante nunca está en el
/// primer milisegundo, así que por encima de 8 s se arranca en la ventana de más energía.
pub fn start_offset_ms(peaks_bytes: &[u8], duration_ms: i64) -> i64 {
    const UMBRAL_MS: i64 = 8_000;
    if duration_ms <= UMBRAL_MS || peaks_bytes.len() < 4 {
        return 0;
    }
    let buckets = peaks_bytes.len() / 2;
    let ventana = (buckets / 8).max(1); // ~1/8 del sample
    let energia = |i: usize| -> i32 {
        let mn = peaks_bytes[i * 2] as i8 as i32;
        let mx = peaks_bytes[i * 2 + 1] as i8 as i32;
        mn.abs().max(mx.abs())
    };
    let mut mejor = (0usize, i32::MIN);
    // solo se considera el primer 60 %: arrancar cerca del final desorienta
    let limite = (buckets * 6 / 10).max(ventana);
    for inicio in 0..=(limite - ventana) {
        let suma: i32 = (inicio..inicio + ventana).map(energia).sum();
        if suma > mejor.1 {
            mejor = (inicio, suma);
        }
    }
    (mejor.0 as i64 * duration_ms) / buckets as i64
}

/// Todo lo que el analizador guarda de un archivo, calculado de una pasada.
pub struct Analisis {
    pub duration_ms: i64,
    pub sample_rate: i64,
    pub channels: i64,
    pub bit_depth: Option<i64>,
    pub loudness_db: f64,
    pub peaks: Vec<u8>,
    pub content_hash: Option<Vec<u8>>,
}

pub fn analizar(path: &Path) -> Result<Analisis> {
    let buf = decode(path)?;
    let duration_ms = buf.duration_ms();
    // El hash solo tiene sentido en material corto: nadie tiene dos veces el mismo tema de 5 min
    // en un pack, y hashear todo multiplicaría el coste del análisis sin ganar nada.
    let hash = if duration_ms <= 30_000 {
        Some(content_hash(&buf.samples, buf.channels).to_vec())
    } else {
        None
    };
    Ok(Analisis {
        duration_ms,
        sample_rate: buf.sample_rate as i64,
        channels: buf.channels as i64,
        bit_depth: buf.bit_depth.map(|b| b as i64),
        loudness_db: loudness_db(&buf.samples),
        peaks: peaks(&buf.samples, buf.channels, BUCKETS),
        content_hash: hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seno(sr: u32, freq: f32, dur: f32, ch: u16) -> AudioBuffer {
        let frames = (sr as f32 * dur) as usize;
        let mut s = vec![0.0f32; frames * ch as usize];
        for f in 0..frames {
            let v = (std::f32::consts::TAU * freq * f as f32 / sr as f32).sin();
            for c in 0..ch as usize {
                s[f * ch as usize + c] = v;
            }
        }
        AudioBuffer {
            samples: s,
            channels: ch,
            sample_rate: sr,
            bit_depth: Some(16),
        }
    }

    #[test]
    fn los_picos_reflejan_la_amplitud() {
        let b = seno(44_100, 440.0, 0.5, 1);
        let p = peaks(&b.samples, 1, 100);
        assert_eq!(p.len(), 200);
        let mx = p[1] as i8;
        let mn = p[0] as i8;
        assert!(
            mx > 120,
            "el pico positivo debería estar cerca de 127: {mx}"
        );
        assert!(
            mn < -120,
            "el pico negativo debería estar cerca de -127: {mn}"
        );
    }

    #[test]
    fn el_hash_iguala_el_mismo_pcm_en_distinto_envoltorio() {
        // Caso real: el mismo pack distribuido en WAV y en FLAC. El PCM decodificado es
        // idéntico bit a bit, solo cambia el contenedor.
        let a = seno(44_100, 440.0, 0.2, 1);
        let b = seno(44_100, 440.0, 0.2, 1);
        assert_eq!(content_hash(&a.samples, 1), content_hash(&b.samples, 1));
    }

    #[test]
    fn el_hash_iguala_estereo_y_su_mezcla_a_mono() {
        let mono = seno(44_100, 440.0, 0.2, 1);
        let estereo = seno(44_100, 440.0, 0.2, 2);
        assert_eq!(
            content_hash(&mono.samples, 1),
            content_hash(&estereo.samples, 2),
            "el mismo sonido en mono y en estéreo es el mismo sonido"
        );
    }

    #[test]
    fn el_hash_no_sobrevive_a_un_dither_distinto() {
        // Documenta el límite: un hash exacto NO detecta parecido perceptual. Si esto
        // empezara a pasar, sería porque alguien ha cambiado el hash por una huella acústica.
        let a = seno(44_100, 440.0, 0.2, 1);
        let mut b = seno(44_100, 440.0, 0.2, 1);
        for (i, s) in b.samples.iter_mut().enumerate() {
            *s += if i % 2 == 0 { 1.0 } else { -1.0 } / 20_000.0;
        }
        assert_ne!(content_hash(&a.samples, 1), content_hash(&b.samples, 1));
    }

    #[test]
    fn el_hash_distingue_sonidos_distintos() {
        let a = seno(44_100, 440.0, 0.2, 1);
        let b = seno(44_100, 660.0, 0.2, 1);
        assert_ne!(content_hash(&a.samples, 1), content_hash(&b.samples, 1));
    }

    #[test]
    fn el_arranque_inteligente_solo_actua_en_samples_largos() {
        let corto = peaks(&seno(44_100, 440.0, 1.0, 1).samples, 1, 1000);
        assert_eq!(start_offset_ms(&corto, 1_000), 0);

        // sample largo con la energía en el segundo cuarto
        let mut p = vec![0u8; 2000];
        for i in 250..400 {
            p[i * 2] = (-120i8) as u8;
            p[i * 2 + 1] = 120u8;
        }
        let off = start_offset_ms(&p, 20_000);
        assert!(
            (4_000..=8_000).contains(&off),
            "debería arrancar cerca de la zona con energía, no en {off} ms"
        );
    }

    #[test]
    fn la_sonoridad_de_un_seno_es_menos_3_db() {
        let b = seno(44_100, 440.0, 0.5, 1);
        let db = loudness_db(&b.samples);
        assert!(
            (db + 3.01).abs() < 0.2,
            "el RMS de un seno a escala completa es -3,01 dB, no {db}"
        );
    }
}
