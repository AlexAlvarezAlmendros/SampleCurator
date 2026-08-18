//! El motor: un hilo de control que posee el stream de cpal y habla con el callback por
//! ring buffers. El resto de la app solo ve `AudioHandle`, que es `Send + Sync`.
//!
//! El stream se abre UNA vez al arrancar la app y no se cierra hasta salir. Abrir un device
//! cuesta 50-200 ms: hacerlo por sample destruiría el producto (medido en la Fase 0).

use super::cache::Cache;
use super::graph::{Cmd, Graph};
use crate::codec::{self, AudioBuffer};
use crate::domain::AudioInfo;
use crate::error::{AppError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const FADE_MS: f32 = 5.0;
/// Calibrado en la Fase 0: con el buffer por defecto del device el p95 sube de 2,6 ms a 42 ms.
pub const BUFFER_FRAMES: u32 = 256;
pub const CACHE_BYTES: usize = 256 * 1024 * 1024;
const VENTANA_LATENCIA: usize = 256;

fn epoca() -> &'static Instant {
    static E: OnceLock<Instant> = OnceLock::new();
    E.get_or_init(Instant::now)
}

pub fn ahora_ns() -> u64 {
    epoca().elapsed().as_nanos() as u64
}

pub fn ahora_ms() -> f64 {
    epoca().elapsed().as_secs_f64() * 1000.0
}

enum Peticion {
    Play {
        sample_id: i64,
        path: PathBuf,
        start_ms: f64,
        looping: bool,
        t_send_ns: u64,
    },
    Stop,
    SeekMs(f64),
    Gain(f32),
    Looping(bool),
    Prefetch(Vec<(i64, PathBuf)>),
    Olvidar(Vec<i64>),
    Apagar,
}

#[derive(Default)]
struct Estado {
    sample_rate: AtomicU32,
    channels: AtomicU32,
    buffer_frames: AtomicU32,
    cache_bytes: AtomicU64,
    cache_limite: AtomicU64,
    cache_entradas: AtomicU64,
    lat_p50_us: AtomicU64,
    lat_p95_us: AtomicU64,
    disparos: AtomicU64,
    sonando: AtomicI64,
}

/// Lo que ve el resto de la app. Solo lleva un canal y unos atómicos: `Send + Sync` sin drama.
#[derive(Clone)]
pub struct AudioHandle {
    tx: Sender<Peticion>,
    estado: Arc<Estado>,
}

impl AudioHandle {
    pub fn sample_rate(&self) -> u32 {
        self.estado.sample_rate.load(Ordering::Relaxed)
    }

    pub fn play(&self, sample_id: i64, path: PathBuf, start_ms: f64, looping: bool) {
        self.estado.sonando.store(sample_id, Ordering::Relaxed);
        let _ = self.tx.send(Peticion::Play {
            sample_id,
            path,
            start_ms,
            looping,
            t_send_ns: ahora_ns(),
        });
    }

    pub fn stop(&self) {
        self.estado.sonando.store(0, Ordering::Relaxed);
        let _ = self.tx.send(Peticion::Stop);
    }

    pub fn seek_ms(&self, ms: f64) {
        let _ = self.tx.send(Peticion::SeekMs(ms));
    }

    pub fn gain(&self, g: f32) {
        let _ = self.tx.send(Peticion::Gain(g));
    }

    pub fn set_looping(&self, l: bool) {
        let _ = self.tx.send(Peticion::Looping(l));
    }

    /// Decodifica en segundo plano los vecinos de la selección: cuando el usuario llega,
    /// el sample ya está en RAM. Es lo que convierte 12 ms de latencia en 2,6 ms.
    pub fn prefetch(&self, items: Vec<(i64, PathBuf)>) {
        if !items.is_empty() {
            let _ = self.tx.send(Peticion::Prefetch(items));
        }
    }

    /// Saca de la caché samples cuya ruta ha cambiado (los acabamos de mover).
    pub fn olvidar(&self, ids: Vec<i64>) {
        if !ids.is_empty() {
            let _ = self.tx.send(Peticion::Olvidar(ids));
        }
    }

    pub fn info(&self) -> AudioInfo {
        let e = &self.estado;
        AudioInfo {
            sample_rate: e.sample_rate.load(Ordering::Relaxed) as i64,
            channels: e.channels.load(Ordering::Relaxed) as i64,
            buffer_frames: e.buffer_frames.load(Ordering::Relaxed) as i64,
            cache_bytes: e.cache_bytes.load(Ordering::Relaxed) as i64,
            cache_limit_bytes: e.cache_limite.load(Ordering::Relaxed) as i64,
            cache_entries: e.cache_entradas.load(Ordering::Relaxed) as i64,
            latency_p50_ms: e.lat_p50_us.load(Ordering::Relaxed) as f64 / 1000.0,
            latency_p95_ms: e.lat_p95_us.load(Ordering::Relaxed) as f64 / 1000.0,
            shots: e.disparos.load(Ordering::Relaxed) as i64,
        }
    }
}

impl Drop for AudioHandle {
    fn drop(&mut self) {
        // Solo apaga de verdad cuando cae el último handle; el resto son clones.
        if Arc::strong_count(&self.estado) == 1 {
            let _ = self.tx.send(Peticion::Apagar);
        }
    }
}

/// Arranca el motor. Devuelve el handle o el motivo por el que no hay audio: la app tiene que
/// poder abrirse igualmente aunque no haya tarjeta de sonido.
pub fn arrancar() -> Result<AudioHandle> {
    let _ = epoca();
    let (tx, rx) = std::sync::mpsc::channel::<Peticion>();
    let estado = Arc::new(Estado::default());
    let estado_hilo = Arc::clone(&estado);
    let (listo_tx, listo_rx) = std::sync::mpsc::channel::<Result<()>>();

    std::thread::Builder::new()
        .name("audio-control".into())
        .spawn(move || match preparar(&estado_hilo) {
            Ok(motor) => {
                let _ = listo_tx.send(Ok(()));
                bucle(motor, rx, estado_hilo);
            }
            Err(e) => {
                let _ = listo_tx.send(Err(e));
            }
        })
        .map_err(|e| AppError::Audio(format!("no se pudo crear el hilo de audio: {e}")))?;

    match listo_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(AudioHandle { tx, estado }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(AppError::Audio(
            "el dispositivo de audio no respondió en 5 s".into(),
        )),
    }
}

struct Motor {
    _stream: cpal::Stream,
    cmd_tx: rtrb::Producer<Cmd>,
    basura_rx: rtrb::Consumer<Arc<AudioBuffer>>,
    lat_rx: rtrb::Consumer<u64>,
    cache: Arc<Mutex<Cache>>,
    sample_rate: u32,
    latencias: Vec<f64>,
}

fn preparar(estado: &Arc<Estado>) -> Result<Motor> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| AppError::Audio("no hay dispositivo de salida por defecto".into()))?;

    let por_defecto = device
        .default_output_config()
        .map_err(|e| AppError::Audio(format!("no se pudo consultar el dispositivo: {e}")))?;

    // Se necesita f32: es lo que produce el decodificador y lo que mezcla el grafo.
    let soportada = if por_defecto.sample_format() == cpal::SampleFormat::F32 {
        por_defecto
    } else {
        device
            .supported_output_configs()
            .map_err(|e| AppError::Audio(e.to_string()))?
            .find(|c| c.sample_format() == cpal::SampleFormat::F32)
            .map(|c| c.with_max_sample_rate())
            .ok_or_else(|| {
                AppError::Audio("el dispositivo de salida no admite muestras f32".into())
            })?
    };

    let sample_rate = soportada.sample_rate();
    let canales = soportada.channels();

    let (cmd_tx, mut cmd_rx) = rtrb::RingBuffer::<Cmd>::new(64);
    let (basura_tx, basura_rx) = rtrb::RingBuffer::<Arc<AudioBuffer>>::new(256);
    let (lat_tx, lat_rx) = rtrb::RingBuffer::<u64>::new(1024);
    let mut lat_tx = lat_tx;

    let mut graph = Graph::nuevo(sample_rate, canales, FADE_MS, Some(basura_tx));

    let callback = move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
        let t_cb = ahora_ns();
        let adelanto_ns = info
            .timestamp()
            .playback
            .duration_since(&info.timestamp().callback)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        graph.limpiar_arranques();
        while let Ok(cmd) = cmd_rx.pop() {
            graph.aplicar(cmd);
        }
        graph.process(data);

        for i in 0..graph.arranques_len {
            let t_send = graph.arranques[i];
            if t_send != 0 {
                let _ = lat_tx.push(t_cb.saturating_sub(t_send) + adelanto_ns);
            }
        }
    };

    let mut cfg: cpal::StreamConfig = soportada.into();
    cfg.buffer_size = cpal::BufferSize::Fixed(BUFFER_FRAMES);

    // Si el device rechaza el tamaño fijo se reintenta con el suyo, pero se avisa: en la
    // Fase 0 el buffer por defecto costaba 42 ms de p95 frente a 2,6 ms con 256 frames.
    let (stream, frames) = match device.build_output_stream(
        &cfg,
        callback,
        |e| eprintln!("[audio] error del stream: {e}"),
        None,
    ) {
        Ok(s) => (s, BUFFER_FRAMES),
        Err(_) => {
            eprintln!(
                "[audio] el dispositivo no acepta un buffer fijo de {BUFFER_FRAMES} frames; \
                 se usa el suyo y la latencia será peor"
            );
            return Err(AppError::Audio(format!(
                "el dispositivo no acepta un buffer de {BUFFER_FRAMES} frames"
            )));
        }
    };
    stream
        .play()
        .map_err(|e| AppError::Audio(format!("no se pudo arrancar el stream: {e}")))?;

    let cache = Arc::new(Mutex::new(Cache::nueva(CACHE_BYTES)));
    estado.sample_rate.store(sample_rate, Ordering::Relaxed);
    estado.channels.store(canales as u32, Ordering::Relaxed);
    estado.buffer_frames.store(frames, Ordering::Relaxed);
    estado
        .cache_limite
        .store(CACHE_BYTES as u64, Ordering::Relaxed);

    Ok(Motor {
        _stream: stream,
        cmd_tx,
        basura_rx,
        lat_rx,
        cache,
        sample_rate,
        latencias: Vec::with_capacity(VENTANA_LATENCIA),
    })
}

fn bucle(mut motor: Motor, rx: Receiver<Peticion>, estado: Arc<Estado>) {
    loop {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(Peticion::Apagar) => break,
            Ok(p) => manejar(&mut motor, p),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        // Soltar aquí los buffers que el callback ha retirado: liberar memoria es cosa
        // del hilo de control, nunca del de audio.
        while motor.basura_rx.pop().is_ok() {}
        recoger_latencias(&mut motor, &estado);
        publicar_cache(&motor, &estado);
    }
}

fn manejar(motor: &mut Motor, p: Peticion) {
    match p {
        Peticion::Play {
            sample_id,
            path,
            start_ms,
            looping,
            t_send_ns,
        } => {
            let buf = match obtener(motor, sample_id, &path) {
                Some(b) => b,
                None => return,
            };
            let start_frame = ((start_ms / 1000.0) * motor.sample_rate as f64).max(0.0) as usize;
            let _ = motor.cmd_tx.push(Cmd::Play {
                buf,
                start_frame,
                looping,
                t_send_ns,
            });
        }
        Peticion::Stop => {
            let _ = motor.cmd_tx.push(Cmd::Stop);
        }
        Peticion::SeekMs(ms) => {
            let frame = ((ms / 1000.0) * motor.sample_rate as f64).max(0.0) as usize;
            let _ = motor.cmd_tx.push(Cmd::Seek(frame));
        }
        Peticion::Gain(g) => {
            let _ = motor.cmd_tx.push(Cmd::Gain(g));
        }
        Peticion::Looping(l) => {
            let _ = motor.cmd_tx.push(Cmd::Looping(l));
        }
        Peticion::Prefetch(items) => {
            let cache = Arc::clone(&motor.cache);
            let sr = motor.sample_rate;
            // En el pool de rayon: decodificar no puede bloquear la atención a los mandos.
            rayon::spawn(move || {
                for (id, path) in items {
                    let ya = cache.lock().map(|c| c.contiene(id)).unwrap_or(true);
                    if ya {
                        continue;
                    }
                    if let Ok(b) = codec::decode_at(&path, sr) {
                        if let Ok(mut c) = cache.lock() {
                            c.insertar(id, Arc::new(b));
                        }
                    }
                }
            });
        }
        Peticion::Olvidar(ids) => {
            if let Ok(mut c) = motor.cache.lock() {
                for id in ids {
                    c.quitar(id);
                }
            }
        }
        Peticion::Apagar => {}
    }
}

fn obtener(motor: &mut Motor, id: i64, path: &Path) -> Option<Arc<AudioBuffer>> {
    if let Ok(mut c) = motor.cache.lock() {
        if let Some(b) = c.obtener(id) {
            return Some(b);
        }
    }
    match codec::decode_at(path, motor.sample_rate) {
        Ok(b) => {
            let arc = Arc::new(b);
            if let Ok(mut c) = motor.cache.lock() {
                c.insertar(id, Arc::clone(&arc));
            }
            Some(arc)
        }
        Err(e) => {
            eprintln!("[audio] no se pudo decodificar {}: {e}", path.display());
            None
        }
    }
}

fn recoger_latencias(motor: &mut Motor, estado: &Arc<Estado>) {
    let mut nuevas = false;
    while let Ok(ns) = motor.lat_rx.pop() {
        if motor.latencias.len() >= VENTANA_LATENCIA {
            motor.latencias.remove(0);
        }
        motor.latencias.push(ns as f64 / 1000.0); // microsegundos
        nuevas = true;
        estado.disparos.fetch_add(1, Ordering::Relaxed);
    }
    if !nuevas {
        return;
    }
    let mut v = motor.latencias.clone();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pct = |p: f64| -> u64 {
        if v.is_empty() {
            return 0;
        }
        let i = ((p * (v.len() - 1) as f64).round() as usize).min(v.len() - 1);
        v[i] as u64
    };
    estado.lat_p50_us.store(pct(0.50), Ordering::Relaxed);
    estado.lat_p95_us.store(pct(0.95), Ordering::Relaxed);
}

fn publicar_cache(motor: &Motor, estado: &Arc<Estado>) {
    if let Ok(c) = motor.cache.lock() {
        estado
            .cache_bytes
            .store(c.bytes() as u64, Ordering::Relaxed);
        estado
            .cache_entradas
            .store(c.entradas() as u64, Ordering::Relaxed);
    }
}
