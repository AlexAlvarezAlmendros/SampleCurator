//! Decodificación con symphonia → buffer f32 intercalado, ya a la frecuencia del device.
//!
//! Ocurre SIEMPRE en el hilo de control, nunca en el callback de audio.

use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::Arc;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioBuffer {
    pub samples: Vec<f32>, // intercalado
    pub channels: u16,
    pub sample_rate: u32,
    pub nombre: String,
}

impl AudioBuffer {
    pub fn frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / self.channels as usize
        }
    }
    pub fn duracion_s(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.frames() as f64 / self.sample_rate as f64
        }
    }
    pub fn bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<f32>()
    }
}

/// Decodifica el archivo entero a f32 intercalado, en su frecuencia original.
pub fn decodificar(path: &Path) -> Result<AudioBuffer> {
    let archivo = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(archivo), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("sin pista de audio"))?;
    let track_id = track.id;

    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let mut samples: Vec<f32> = Vec::with_capacity(1 << 16);
    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut sbuf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break, // fin del stream
        };
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
                let buf = sbuf.get_or_insert_with(|| {
                    SampleBuffer::<f32>::new(decoded.capacity() as u64, spec)
                });
                buf.copy_interleaved_ref(decoded);
                samples.extend_from_slice(buf.samples());
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }

    if samples.is_empty() || channels == 0 {
        return Err(anyhow!("no se decodificó ninguna muestra"));
    }

    Ok(AudioBuffer {
        samples,
        channels,
        sample_rate,
        nombre: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string(),
    })
}

/// Decodifica y, si hace falta, remuestrea a `target_sr`.
///
/// El spike usa interpolación lineal: sirve para medir el coste y comprobar que el tono es
/// correcto. La app usará `rubato` (calidad), decidido en la Fase 3 — ver ADR-0002.
pub fn decodificar_a(path: &Path, target_sr: u32) -> Result<Arc<AudioBuffer>> {
    let buf = decodificar(path)?;
    if buf.sample_rate == target_sr {
        return Ok(Arc::new(buf));
    }
    Ok(Arc::new(remuestrear_lineal(buf, target_sr)))
}

pub fn remuestrear_lineal(buf: AudioBuffer, target_sr: u32) -> AudioBuffer {
    let ch = buf.channels as usize;
    let frames_in = buf.frames();
    let ratio = target_sr as f64 / buf.sample_rate as f64;
    let frames_out = ((frames_in as f64) * ratio).round() as usize;
    let mut out = vec![0.0f32; frames_out * ch];

    for f in 0..frames_out {
        let pos = f as f64 / ratio;
        let i = pos.floor() as usize;
        let frac = (pos - i as f64) as f32;
        let i2 = (i + 1).min(frames_in.saturating_sub(1));
        for c in 0..ch {
            let a = buf.samples[i * ch + c];
            let b = buf.samples[i2 * ch + c];
            out[f * ch + c] = a + (b - a) * frac;
        }
    }

    AudioBuffer {
        samples: out,
        channels: buf.channels,
        sample_rate: target_sr,
        nombre: buf.nombre,
    }
}
