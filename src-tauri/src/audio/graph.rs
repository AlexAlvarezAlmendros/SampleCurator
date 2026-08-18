//! CÓDIGO DE TIEMPO REAL. Esto corre dentro del callback de cpal.
//!
//! Prohibido aquí dentro: reservar o liberar memoria, bloquear, hacer I/O, loggear y panicar.
//! Todo acceso indexado va por `get()`. Los `Arc` que dejan de sonar NO se sueltan aquí:
//! se devuelven al hilo de control por un ring buffer, porque soltar el último Arc llamaría
//! al asignador dentro del hilo de audio.

use crate::codec::AudioBuffer;
use std::sync::Arc;

/// Mandos que el hilo de control envía al callback por un ring SPSC.
pub enum Cmd {
    Play {
        buf: Arc<AudioBuffer>,
        start_frame: usize,
        looping: bool,
        t_send_ns: u64,
    },
    Stop,
    Seek(usize),
    Gain(f32),
    Looping(bool),
}

struct Voz {
    buf: Arc<AudioBuffer>,
    pos: usize,
    gain: f32,
    objetivo: f32,
    paso: f32,
    looping: bool,
}

pub struct Graph {
    canales: usize,
    voz: Option<Voz>,
    saliendo: Option<Voz>,
    master: f32,
    master_objetivo: f32,
    paso_master: f32,
    paso_fade: f32,
    pub arranques: [u64; 8],
    pub arranques_len: usize,
    basura: Option<rtrb::Producer<Arc<AudioBuffer>>>,
    /// Frame actual de la voz activa, para que el hilo de control pueda leerlo sin bloquear.
    pub pos_actual: usize,
}

impl Graph {
    pub fn nuevo(
        sample_rate: u32,
        canales: u16,
        fade_ms: f32,
        basura: Option<rtrb::Producer<Arc<AudioBuffer>>>,
    ) -> Self {
        let frames_fade = (sample_rate as f32 * fade_ms / 1000.0).max(1.0);
        Self {
            canales: canales.max(1) as usize,
            voz: None,
            saliendo: None,
            master: 1.0,
            master_objetivo: 1.0,
            paso_master: 1.0 / (sample_rate as f32 * 0.010).max(1.0),
            paso_fade: 1.0 / frames_fade,
            arranques: [0; 8],
            arranques_len: 0,
            basura: None.or(basura),
            pos_actual: 0,
        }
    }

    /// Retira una voz sin soltar su Arc en el hilo de audio.
    fn reciclar(&mut self, voz: Option<Voz>) {
        if let Some(v) = voz {
            if let Some(b) = self.basura.as_mut() {
                let _ = b.push(v.buf);
            }
        }
    }

    pub fn aplicar(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Play {
                buf,
                start_frame,
                looping,
                t_send_ns,
            } => {
                let anterior = self.saliendo.take();
                self.reciclar(anterior);
                if let Some(mut v) = self.voz.take() {
                    v.objetivo = 0.0;
                    v.paso = self.paso_fade;
                    v.looping = false;
                    self.saliendo = Some(v);
                }
                let frames = buf.frames();
                self.voz = Some(Voz {
                    buf,
                    pos: start_frame.min(frames.saturating_sub(1)),
                    gain: 0.0,
                    objetivo: 1.0,
                    paso: self.paso_fade,
                    looping,
                });
                if self.arranques_len < self.arranques.len() {
                    self.arranques[self.arranques_len] = t_send_ns;
                    self.arranques_len += 1;
                }
            }
            Cmd::Stop => {
                if let Some(mut v) = self.voz.take() {
                    v.objetivo = 0.0;
                    v.paso = self.paso_fade;
                    v.looping = false;
                    let anterior = self.saliendo.take();
                    self.reciclar(anterior);
                    self.saliendo = Some(v);
                }
            }
            Cmd::Seek(frame) => {
                if let Some(v) = self.voz.as_mut() {
                    let frames = v.buf.frames();
                    v.pos = frame.min(frames.saturating_sub(1));
                }
            }
            Cmd::Gain(g) => self.master_objetivo = g.clamp(0.0, 4.0),
            Cmd::Looping(l) => {
                if let Some(v) = self.voz.as_mut() {
                    v.looping = l;
                }
            }
        }
    }

    pub fn process(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let ch = self.canales;

        // El master también va con rampa: un salto de ganancia se oye como un chasquido.
        if (self.master - self.master_objetivo).abs() > f32::EPSILON {
            let frames = out.len() / ch.max(1);
            let delta = self.paso_master * frames as f32;
            if self.master < self.master_objetivo {
                self.master = (self.master + delta).min(self.master_objetivo);
            } else {
                self.master = (self.master - delta).max(self.master_objetivo);
            }
        }
        let master = self.master;

        if let Some(v) = self.saliendo.as_mut() {
            if mezclar(v, out, ch, master) {
                let t = self.saliendo.take();
                self.reciclar(t);
            }
        }
        if let Some(v) = self.voz.as_mut() {
            let termina = mezclar(v, out, ch, master);
            self.pos_actual = v.pos;
            if termina {
                let t = self.voz.take();
                self.reciclar(t);
                self.pos_actual = 0;
            }
        }
    }

    pub fn limpiar_arranques(&mut self) {
        self.arranques_len = 0;
    }

    pub fn sonando(&self) -> bool {
        self.voz.is_some()
    }
}

/// Devuelve true cuando la voz ha terminado. Sin allocs, sin panics.
fn mezclar(v: &mut Voz, out: &mut [f32], out_ch: usize, master: f32) -> bool {
    let src_ch = v.buf.channels.max(1) as usize;
    if out_ch == 0 {
        return true;
    }
    let frames_src = v.buf.frames();
    if frames_src == 0 {
        return true;
    }
    let frames_out = out.len() / out_ch;

    for f in 0..frames_out {
        if v.pos >= frames_src {
            if v.looping {
                v.pos = 0;
            } else {
                return true;
            }
        }
        if v.gain < v.objetivo {
            v.gain = (v.gain + v.paso).min(v.objetivo);
        } else if v.gain > v.objetivo {
            v.gain = (v.gain - v.paso).max(v.objetivo);
        }
        let g = v.gain * master;
        let base = v.pos * src_ch;

        if src_ch == out_ch {
            for c in 0..out_ch {
                let s = v.buf.samples.get(base + c).copied().unwrap_or(0.0);
                if let Some(d) = out.get_mut(f * out_ch + c) {
                    *d += s * g;
                }
            }
        } else if src_ch == 1 {
            let s = v.buf.samples.get(base).copied().unwrap_or(0.0) * g;
            for c in 0..out_ch {
                if let Some(d) = out.get_mut(f * out_ch + c) {
                    *d += s;
                }
            }
        } else {
            let mut acc = 0.0f32;
            for c in 0..src_ch {
                acc += v.buf.samples.get(base + c).copied().unwrap_or(0.0);
            }
            let s = acc / src_ch as f32 * g;
            for c in 0..out_ch {
                if let Some(d) = out.get_mut(f * out_ch + c) {
                    *d += s;
                }
            }
        }

        v.pos += 1;
        if v.objetivo == 0.0 && v.gain <= 0.0 {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seno(sr: u32, ch: u16, freq: f32, dur: f32) -> Arc<AudioBuffer> {
        let frames = (sr as f32 * dur) as usize;
        let mut s = vec![0.0f32; frames * ch as usize];
        for f in 0..frames {
            let v = (std::f32::consts::TAU * freq * f as f32 / sr as f32).sin();
            for c in 0..ch as usize {
                s[f * ch as usize + c] = v;
            }
        }
        Arc::new(AudioBuffer {
            samples: s,
            channels: ch,
            sample_rate: sr,
            bit_depth: None,
        })
    }

    fn play(g: &mut Graph, buf: Arc<AudioBuffer>) {
        g.aplicar(Cmd::Play {
            buf,
            start_frame: 0,
            looping: false,
            t_send_ns: 0,
        });
    }

    /// El salto natural de un seno de 660 Hz a 48 kHz entre muestras consecutivas.
    const NATURAL: f32 = std::f32::consts::TAU * 660.0 / 48_000.0;

    fn max_delta_retriggeando(fade_ms: f32) -> f32 {
        let (sr, ch) = (48_000u32, 2u16);
        let mut g = Graph::nuevo(sr, ch, fade_ms, None);
        let a = seno(sr, ch, 440.0, 1.0);
        let b = seno(sr, ch, 660.0, 1.0);
        let bloque = 256usize;
        let mut out = vec![0.0f32; bloque * ch as usize];
        let mut previa = 0.0f32;
        let mut max = 0.0f32;

        for i in 0..200 {
            if i % 8 == 0 {
                play(&mut g, if i % 16 == 0 { a.clone() } else { b.clone() });
            }
            g.process(&mut out);
            for f in 0..bloque {
                let v = out[f * ch as usize];
                let d = (v - previa).abs();
                if i > 0 && d > max {
                    max = d;
                }
                previa = v;
            }
        }
        max
    }

    #[test]
    fn el_fade_de_5ms_elimina_los_clics_al_cambiar_de_sample() {
        let con_fade = max_delta_retriggeando(5.0);
        assert!(
            con_fade < NATURAL * 3.0,
            "salto de {con_fade} con fade; el natural del material es {NATURAL}"
        );
    }

    #[test]
    fn sin_fade_si_hay_clics_la_medida_lo_detecta() {
        // Control negativo: si esto dejara de fallar, el detector estaría roto y el test
        // de arriba no probaría nada.
        let sin_fade = max_delta_retriggeando(0.02);
        assert!(
            sin_fade > NATURAL * 3.0,
            "el control negativo debería mostrar un salto grande, y dio {sin_fade}"
        );
    }

    #[test]
    fn una_voz_sin_loop_termina_y_deja_silencio() {
        let (sr, ch) = (48_000u32, 2u16);
        let mut g = Graph::nuevo(sr, ch, 5.0, None);
        play(&mut g, seno(sr, ch, 440.0, 0.01)); // 480 frames
        let mut out = vec![0.0f32; 512 * ch as usize];
        g.process(&mut out);
        assert!(!g.sonando() || g.pos_actual == 0);
        g.process(&mut out);
        assert!(out.iter().all(|s| *s == 0.0), "debería quedar en silencio");
    }

    #[test]
    fn el_loop_no_deja_de_sonar() {
        let (sr, ch) = (48_000u32, 2u16);
        let mut g = Graph::nuevo(sr, ch, 5.0, None);
        g.aplicar(Cmd::Play {
            buf: seno(sr, ch, 440.0, 0.01),
            start_frame: 0,
            looping: true,
            t_send_ns: 0,
        });
        let mut out = vec![0.0f32; 512 * ch as usize];
        for _ in 0..20 {
            g.process(&mut out);
        }
        assert!(g.sonando(), "con loop la voz no debe terminar nunca");
        assert!(out.iter().any(|s| s.abs() > 0.1), "y debe seguir sonando");
    }

    #[test]
    fn arrancar_con_offset_respeta_la_posicion() {
        let (sr, ch) = (48_000u32, 1u16);
        let mut g = Graph::nuevo(sr, ch, 5.0, None);
        g.aplicar(Cmd::Play {
            buf: seno(sr, ch, 440.0, 1.0),
            start_frame: 24_000,
            looping: false,
            t_send_ns: 0,
        });
        let mut out = vec![0.0f32; 256];
        g.process(&mut out);
        assert!(g.pos_actual >= 24_000 && g.pos_actual <= 24_300);
    }

    #[test]
    fn los_buffers_retirados_van_a_la_basura_y_no_se_sueltan_en_el_callback() {
        let (mut tx, mut rx) = rtrb::RingBuffer::<Arc<AudioBuffer>>::new(16);
        // se descarta el productor original y se le da el nuestro al graph
        let mut g = Graph::nuevo(
            48_000,
            2,
            5.0,
            Some(std::mem::replace(
                &mut tx,
                rtrb::RingBuffer::<Arc<AudioBuffer>>::new(1).0,
            )),
        );
        let a = seno(48_000, 2, 440.0, 0.5);
        let b = seno(48_000, 2, 660.0, 0.5);
        play(&mut g, a);
        let mut out = vec![0.0f32; 512];
        g.process(&mut out);
        play(&mut g, b); // desplaza a la voz anterior
        g.process(&mut out);
        play(&mut g, seno(48_000, 2, 880.0, 0.5)); // recicla la que estaba saliendo
        g.process(&mut out);
        assert!(
            rx.pop().is_ok(),
            "el hilo de control debe recibir el buffer retirado"
        );
    }
}
