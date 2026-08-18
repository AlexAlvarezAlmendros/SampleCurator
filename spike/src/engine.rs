//! Motor de audio del spike: hilo de control + callback de cpal.
//!
//! REGLAS DE TIEMPO REAL dentro de `Graph::process` y `Graph::aplicar`:
//!   · no se reserva ni se libera memoria
//!   · no se bloquea (nada de Mutex/RwLock)
//!   · no hay I/O ni logs
//!   · no se puede panicar (todo acceso indexado va por `get()`)
//!
//! Los `Arc<AudioBuffer>` que dejan de sonar NO se sueltan en el callback: se devuelven al hilo
//! de control por un ring buffer de basura, porque soltar el último Arc llamaría al asignador.

use crate::decode::AudioBuffer;
use anyhow::{anyhow, bail, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

pub fn epoca() -> &'static Instant {
    static E: OnceLock<Instant> = OnceLock::new();
    E.get_or_init(Instant::now)
}

/// Nanosegundos desde el arranque del proceso. En Linux es vDSO: ~20 ns, sin syscall.
pub fn ahora_ns() -> u64 {
    epoca().elapsed().as_nanos() as u64
}

pub enum Cmd {
    Play {
        buf: Arc<AudioBuffer>,
        t_send_ns: u64,
        id: u64,
        looping: bool,
    },
    Stop,
    Gain(f32),
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
    paso_fade: f32,
    pub arranques: [(u64, u64); 8], // (id, t_send_ns)
    pub arranques_len: usize,
    basura: Option<rtrb::Producer<Arc<AudioBuffer>>>,
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
            canales: canales as usize,
            voz: None,
            saliendo: None,
            master: 1.0,
            paso_fade: 1.0 / frames_fade,
            arranques: [(0, 0); 8],
            arranques_len: 0,
            basura,
        }
    }

    /// Retira una voz sin soltar su Arc en el hilo de audio.
    fn reciclar(&mut self, voz: Option<Voz>) {
        if let Some(v) = voz {
            if let Some(b) = self.basura.as_mut() {
                // Si la cola de basura está llena, el push falla y el Arc se suelta aquí.
                // Es el único caso degradado posible y el hilo de control la vacía en cada tick.
                let _ = b.push(v.buf);
            }
        }
    }

    /// RT-safe. Se llama desde el callback, justo antes de `process`.
    pub fn aplicar(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Play {
                buf,
                t_send_ns,
                id,
                looping,
            } => {
                // La voz anterior sale con fade; la que ya estaba saliendo se recicla.
                let anterior = self.saliendo.take();
                self.reciclar(anterior);
                if let Some(mut v) = self.voz.take() {
                    v.objetivo = 0.0;
                    v.paso = self.paso_fade;
                    v.looping = false;
                    self.saliendo = Some(v);
                }
                self.voz = Some(Voz {
                    buf,
                    pos: 0,
                    gain: 0.0,
                    objetivo: 1.0,
                    paso: self.paso_fade,
                    looping,
                });
                if self.arranques_len < self.arranques.len() {
                    self.arranques[self.arranques_len] = (id, t_send_ns);
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
            Cmd::Gain(g) => self.master = g.clamp(0.0, 4.0),
        }
    }

    /// RT-safe. Rellena `out` (intercalado) con la mezcla actual.
    pub fn process(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let ch = self.canales;
        let master = self.master;

        if let Some(v) = self.saliendo.as_mut() {
            if mezclar(v, out, ch, master) {
                let t = self.saliendo.take();
                self.reciclar(t);
            }
        }
        if let Some(v) = self.voz.as_mut() {
            if mezclar(v, out, ch, master) {
                let t = self.voz.take();
                self.reciclar(t);
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

/// Devuelve true si la voz ha terminado. Sin allocs, sin panics.
fn mezclar(v: &mut Voz, out: &mut [f32], out_ch: usize, master: f32) -> bool {
    let src_ch = v.buf.channels as usize;
    if src_ch == 0 || out_ch == 0 {
        return true;
    }
    let frames_src = v.buf.frames();
    let frames_out = out.len() / out_ch;

    for f in 0..frames_out {
        if v.pos >= frames_src {
            if v.looping && frames_src > 0 {
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
                let s = *v.buf.samples.get(base + c).unwrap_or(&0.0);
                if let Some(d) = out.get_mut(f * out_ch + c) {
                    *d += s * g;
                }
            }
        } else if src_ch == 1 {
            let s = *v.buf.samples.get(base).unwrap_or(&0.0) * g;
            for c in 0..out_ch {
                if let Some(d) = out.get_mut(f * out_ch + c) {
                    *d += s;
                }
            }
        } else {
            let mut acc = 0.0f32;
            for c in 0..src_ch {
                acc += *v.buf.samples.get(base + c).unwrap_or(&0.0);
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

pub struct Engine {
    _stream: cpal::Stream,
    cmd_tx: rtrb::Producer<Cmd>,
    lat_rx: rtrb::Consumer<u64>,
    adel_rx: rtrb::Consumer<u64>,
    basura_rx: rtrb::Consumer<Arc<AudioBuffer>>,
    pub sample_rate: u32,
    pub canales: u16,
    pub buffer_frames: String,
    pub dispositivo: String,
    siguiente_id: u64,
}

impl Engine {
    /// Abre el stream UNA sola vez. Nunca se cierra mientras viva el Engine.
    pub fn nuevo(buffer_frames: Option<u32>) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("no hay dispositivo de salida por defecto"))?;
        let nombre_dev = device.name().unwrap_or_else(|_| "?".into());
        let soportada = device.default_output_config()?;
        if soportada.sample_format() != cpal::SampleFormat::F32 {
            bail!(
                "el dispositivo por defecto no acepta f32 (formato: {:?})",
                soportada.sample_format()
            );
        }
        let sample_rate = soportada.sample_rate().0;
        let canales = soportada.channels();

        let (cmd_tx, mut cmd_rx) = rtrb::RingBuffer::<Cmd>::new(64);
        let (lat_tx, lat_rx) = rtrb::RingBuffer::<u64>::new(4096);
        let (adel_tx, adel_rx) = rtrb::RingBuffer::<u64>::new(4096);
        let (basura_tx, basura_rx) = rtrb::RingBuffer::<Arc<AudioBuffer>>::new(256);

        let mut graph = Graph::nuevo(sample_rate, canales, 5.0, Some(basura_tx));
        let mut lat_tx = lat_tx;
        let mut adel_tx = adel_tx;

        let etiqueta_buffer;
        let mut cfg: cpal::StreamConfig = soportada.clone().into();
        match buffer_frames {
            Some(n) => {
                cfg.buffer_size = cpal::BufferSize::Fixed(n);
                etiqueta_buffer = format!("{n} frames (solicitado)");
            }
            None => {
                cfg.buffer_size = cpal::BufferSize::Default;
                etiqueta_buffer = "por defecto del device".to_string();
            }
        }

        let callback = move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
            let t_cb = ahora_ns();
            let adelanto = info
                .timestamp()
                .playback
                .duration_since(&info.timestamp().callback);
            // Marcamos con u64::MAX el caso "el backend no sabe decirlo" para distinguirlo de 0.
            let adelanto_ns = adelanto.map(|d| d.as_nanos() as u64).unwrap_or(0);
            let _ = adel_tx.push(adelanto.map(|d| d.as_nanos() as u64).unwrap_or(u64::MAX));

            graph.limpiar_arranques();
            while let Ok(cmd) = cmd_rx.pop() {
                graph.aplicar(cmd);
            }
            graph.process(data);

            for i in 0..graph.arranques_len {
                let (_id, t_send) = graph.arranques[i];
                let lat = t_cb.saturating_sub(t_send) + adelanto_ns;
                let _ = lat_tx.push(lat);
            }
        };

        let stream = match device.build_output_stream(
            &cfg,
            callback,
            |e| eprintln!("error del stream: {e}"),
            None,
        ) {
            Ok(s) => s,
            Err(e) => bail!("no se pudo abrir el stream ({etiqueta_buffer}): {e}"),
        };
        stream.play()?;

        Ok(Self {
            _stream: stream,
            cmd_tx,
            lat_rx,
            adel_rx,
            basura_rx,
            sample_rate,
            canales,
            buffer_frames: etiqueta_buffer,
            dispositivo: nombre_dev,
            siguiente_id: 1,
        })
    }

    /// Dispara un sample. Devuelve el instante de envío en ns (el "momento de la tecla").
    pub fn play(&mut self, buf: Arc<AudioBuffer>, looping: bool) -> u64 {
        let id = self.siguiente_id;
        self.siguiente_id += 1;
        let t = ahora_ns();
        let _ = self.cmd_tx.push(Cmd::Play {
            buf,
            t_send_ns: t,
            id,
            looping,
        });
        t
    }

    /// Igual que `play`, pero con un instante de envío ya tomado (para medir el camino
    /// completo tecla → decodificación → sonido).
    pub fn play_desde(&mut self, buf: Arc<AudioBuffer>, looping: bool, t_send_ns: u64) {
        let id = self.siguiente_id;
        self.siguiente_id += 1;
        let _ = self.cmd_tx.push(Cmd::Play {
            buf,
            t_send_ns,
            id,
            looping,
        });
    }

    /// Ganancia general. El bench la baja para no atronar los altavoces del usuario.
    pub fn gain(&mut self, g: f32) {
        let _ = self.cmd_tx.push(Cmd::Gain(g));
    }

    pub fn stop(&mut self) {
        let _ = self.cmd_tx.push(Cmd::Stop);
    }

    /// Vacía la cola de latencias medidas por el callback (en ms).
    pub fn recoger_latencias_ms(&mut self) -> Vec<f64> {
        let mut v = Vec::new();
        while let Ok(ns) = self.lat_rx.pop() {
            v.push(ns as f64 / 1_000_000.0);
        }
        v
    }

    /// Adelanto de reproducción que reporta el backend en cada callback, en ms.
    /// `None` en la posición i significa que el backend no lo sabe.
    pub fn recoger_adelantos_ms(&mut self) -> (Vec<f64>, usize) {
        let mut v = Vec::new();
        let mut desconocidos = 0usize;
        while let Ok(ns) = self.adel_rx.pop() {
            if ns == u64::MAX {
                desconocidos += 1;
            } else {
                v.push(ns as f64 / 1_000_000.0);
            }
        }
        (v, desconocidos)
    }

    /// Suelta en el hilo de control los buffers que el callback ha dejado de usar.
    pub fn vaciar_basura(&mut self) {
        while self.basura_rx.pop().is_ok() {}
    }
}
