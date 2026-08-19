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
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
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
    Reconectar,
    Apagar,
}

#[derive(Default)]
struct Estado {
    sample_rate: AtomicU32,
    channels: AtomicU32,
    buffer_frames: AtomicU32,
    /// Si el backend deja fijar el tamaño de buffer. Cuando no (WASAPI), `buffer_frames`
    /// queda a 0 porque lo elige el sistema.
    buffer_fijo: std::sync::atomic::AtomicBool,
    cache_bytes: AtomicU64,
    cache_limite: AtomicU64,
    cache_entradas: AtomicU64,
    lat_p50_us: AtomicU64,
    lat_p95_us: AtomicU64,
    disparos: AtomicU64,
    sonando: AtomicI64,
    reconexiones: AtomicU64,
    /// Nombre del dispositivo en uso. Lo escribe el hilo de control al abrir o reabrir el
    /// stream y lo lee la IPC: nunca lo toca el callback, así que un Mutex aquí es legal.
    dispositivo: Mutex<String>,
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

    /// Vuelve a abrir el dispositivo de salida. Es la salida de emergencia por si la
    /// detección automática no llega a tiempo en algún sistema.
    pub fn reconectar(&self) {
        let _ = self.tx.send(Peticion::Reconectar);
    }

    pub fn info(&self) -> AudioInfo {
        let e = &self.estado;
        AudioInfo {
            device: e
                .dispositivo
                .lock()
                .map(|d| d.clone())
                .unwrap_or_else(|_| "desconocido".into()),
            sample_rate: e.sample_rate.load(Ordering::Relaxed) as i64,
            channels: e.channels.load(Ordering::Relaxed) as i64,
            buffer_frames: e.buffer_frames.load(Ordering::Relaxed) as i64,
            buffer_fixed: e.buffer_fijo.load(Ordering::Relaxed),
            cache_bytes: e.cache_bytes.load(Ordering::Relaxed) as i64,
            cache_limit_bytes: e.cache_limite.load(Ordering::Relaxed) as i64,
            cache_entries: e.cache_entradas.load(Ordering::Relaxed) as i64,
            latency_p50_ms: e.lat_p50_us.load(Ordering::Relaxed) as f64 / 1000.0,
            latency_p95_ms: e.lat_p95_us.load(Ordering::Relaxed) as f64 / 1000.0,
            shots: e.disparos.load(Ordering::Relaxed) as i64,
            reconnections: e.reconexiones.load(Ordering::Relaxed) as i64,
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
    stream: cpal::Stream,
    cmd_tx: rtrb::Producer<Cmd>,
    basura_rx: rtrb::Consumer<Arc<AudioBuffer>>,
    lat_rx: rtrb::Consumer<u64>,
    cache: Arc<Mutex<Cache>>,
    sample_rate: u32,
    dispositivo: String,
    latencias: Vec<f64>,
    /// El callback lo incrementa en cada bloque. Si deja de subir, el stream está muerto.
    latidos: Arc<AtomicU64>,
    fallo: Arc<AtomicBool>,
    /// Ganancia actual, para restituirla en el grafo nuevo tras reconectar.
    gain: f32,
}

struct PiezasStream {
    stream: cpal::Stream,
    cmd_tx: rtrb::Producer<Cmd>,
    basura_rx: rtrb::Consumer<Arc<AudioBuffer>>,
    lat_rx: rtrb::Consumer<u64>,
    sample_rate: u32,
    dispositivo: String,
}

fn preparar(estado: &Arc<Estado>) -> Result<Motor> {
    let latidos = Arc::new(AtomicU64::new(0));
    let fallo = Arc::new(AtomicBool::new(false));
    let piezas = abrir_stream(estado, &latidos, &fallo)?;
    Ok(Motor {
        stream: piezas.stream,
        cmd_tx: piezas.cmd_tx,
        basura_rx: piezas.basura_rx,
        lat_rx: piezas.lat_rx,
        cache: Arc::new(Mutex::new(Cache::nueva(CACHE_BYTES))),
        sample_rate: piezas.sample_rate,
        dispositivo: piezas.dispositivo,
        latencias: Vec::with_capacity(VENTANA_LATENCIA),
        latidos,
        fallo,
        gain: 1.0,
    })
}

/// Vuelve a abrir el dispositivo y monta un grafo nuevo, conservando la caché si puede.
///
/// Se llama cuando el stream deja de latir o da error — al enchufar unos cascos, conectar un
/// Bluetooth o cambiar de salida. Antes de esto, el stream se quedaba muerto y la única forma
/// de volver a oír algo era reiniciar la aplicación.
fn reconectar(motor: &mut Motor, estado: &Arc<Estado>) -> Result<()> {
    motor.fallo.store(false, Ordering::Relaxed);
    let piezas = abrir_stream(estado, &motor.latidos, &motor.fallo)?;

    // Si el dispositivo nuevo va a otra frecuencia, lo que hay en la caché está remuestreado
    // para la anterior: sonaría desafinado. Se tira y se vuelve a decodificar según haga falta.
    if piezas.sample_rate != motor.sample_rate {
        if let Ok(mut c) = motor.cache.lock() {
            c.limpiar();
        }
        eprintln!(
            "[audio] la frecuencia cambió de {} a {} Hz: se vacía la caché",
            motor.sample_rate, piezas.sample_rate
        );
    }

    // El stream viejo se suelta AQUÍ, ya con el nuevo montado: cuanto menos tiempo sin salida,
    // mejor.
    motor.stream = piezas.stream;
    motor.cmd_tx = piezas.cmd_tx;
    motor.basura_rx = piezas.basura_rx;
    motor.lat_rx = piezas.lat_rx;
    motor.sample_rate = piezas.sample_rate;
    motor.dispositivo = piezas.dispositivo;
    motor.latencias.clear();

    // El grafo es nuevo y nace con ganancia 1: hay que devolverle la del usuario.
    let _ = motor.cmd_tx.push(Cmd::Gain(motor.gain));
    estado.reconexiones.fetch_add(1, Ordering::Relaxed);
    eprintln!("[audio] reconectado a «{}»", motor.dispositivo);
    Ok(())
}

/// Abre el dispositivo de salida y monta el grafo. Se usa al arrancar y en cada reconexión.
fn abrir_stream(
    estado: &Arc<Estado>,
    latidos: &Arc<AtomicU64>,
    fallo: &Arc<AtomicBool>,
) -> Result<PiezasStream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| AppError::Audio("no hay dispositivo de salida por defecto".into()))?;
    // `description()` en vez del `name()` obsoleto: en cpal 0.17 es la vía viva y da el
    // nombre legible del dispositivo.
    let nombre_dev = device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "desconocido".into());

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

    let latidos_cb = Arc::clone(latidos);
    let callback = move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
        // Un latido por bloque. Es un atómico: legal dentro del callback, y es lo que permite
        // al hilo de control notar que el stream ha dejado de servirse.
        latidos_cb.fetch_add(1, Ordering::Relaxed);
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

    // Se PREGUNTA antes de construir, en vez de intentarlo y reintentar: el callback solo se
    // puede mover una vez, así que un reintento obligaría a rehacerlo entero.
    //
    // En ALSA el backend declara un rango y se puede fijar el buffer, que es de donde salen
    // los 2,6 ms de la Fase 0. En WASAPI (Windows) cpal informa `Unknown` porque el modo
    // compartido no deja elegir tamaño: allí se usa el del sistema, que ronda los 10 ms.
    // Es peor que 2,6 ms pero perfectamente usable — y muy lejos de los 42 ms que costaba el
    // buffer por defecto de ALSA, que era lo que había que evitar a toda costa.
    let (buffer_size, frames, fijo) = match soportada.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => {
            let n = BUFFER_FRAMES.clamp(*min, *max);
            (cpal::BufferSize::Fixed(n), n, true)
        }
        cpal::SupportedBufferSize::Unknown => (cpal::BufferSize::Default, 0, false),
    };

    let mut cfg: cpal::StreamConfig = soportada.into();
    cfg.buffer_size = buffer_size;

    let fallo_cb = Arc::clone(fallo);
    let stream = device
        .build_output_stream(
            &cfg,
            callback,
            move |e| {
                eprintln!("[audio] error del stream: {e}");
                fallo_cb.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| AppError::Audio(format!("no se pudo abrir el stream de salida: {e}")))?;
    stream
        .play()
        .map_err(|e| AppError::Audio(format!("no se pudo arrancar el stream: {e}")))?;

    estado.sample_rate.store(sample_rate, Ordering::Relaxed);
    estado.channels.store(canales as u32, Ordering::Relaxed);
    estado.buffer_frames.store(frames, Ordering::Relaxed);
    estado.buffer_fijo.store(fijo, Ordering::Relaxed);
    if !fijo {
        eprintln!("[audio] este backend no deja fijar el tamaño de buffer; se usa el del sistema");
    }
    estado
        .cache_limite
        .store(CACHE_BYTES as u64, Ordering::Relaxed);
    if let Ok(mut d) = estado.dispositivo.lock() {
        d.clone_from(&nombre_dev);
    }

    Ok(PiezasStream {
        stream,
        cmd_tx,
        basura_rx,
        lat_rx,
        sample_rate,
        dispositivo: nombre_dev,
    })
}

/// Nombre del dispositivo de salida que el sistema considera el predeterminado AHORA.
/// Es una consulta barata (nanosegundos en ALSA, microsegundos en WASAPI), así que se puede
/// hacer varias veces por segundo desde el hilo de control.
fn dispositivo_por_defecto() -> Option<String> {
    cpal::default_host()
        .default_output_device()
        .and_then(|d| d.description().ok())
        .map(|d| d.name().to_string())
}

/// Decide si hay que reabrir la salida, y por qué. Es una función pura para poder probar la
/// lógica sin tarjeta de sonido: es la parte donde un fallo deja la app muda.
///
/// Tres motivos, por orden de probabilidad real:
/// 1. el sistema cambió de salida por defecto (cascos, Bluetooth, interfaz USB, HDMI) y
///    seguimos escribiendo en el dispositivo viejo: el stream late, pero no se oye nada;
/// 2. el backend nos avisó de un error;
/// 3. el callback dejó de correr: el stream está muerto.
fn motivo_para_reconectar(
    latido: u64,
    latido_previo: u64,
    fallo: bool,
    dispositivo_abierto: &str,
    dispositivo_del_sistema: Option<&str>,
) -> Option<&'static str> {
    if let Some(actual) = dispositivo_del_sistema {
        if actual != dispositivo_abierto {
            return Some("el sistema cambió de salida");
        }
    }
    if fallo {
        return Some("el backend dio error");
    }
    if latido == latido_previo {
        return Some("el stream dejó de responder");
    }
    None
}

/// Cada cuánto se comprueba que el stream sigue vivo. A 256 frames y 44.1 kHz el callback
/// corre cada 5,8 ms, así que medio segundo sin un solo latido no es carga: es que está muerto.
const VIGILANCIA: Duration = Duration::from_millis(500);
/// Y no se reconecta más a menudo que esto, para no entrar en bucle si el sistema no tiene
/// ninguna salida utilizable.
const ESPERA_ENTRE_RECONEXIONES: Duration = Duration::from_secs(2);

fn bucle(mut motor: Motor, rx: Receiver<Peticion>, estado: Arc<Estado>) {
    let mut ultimo_latido = motor.latidos.load(Ordering::Relaxed);
    let mut ultima_vigilancia = Instant::now();
    let mut ultima_reconexion = Instant::now() - ESPERA_ENTRE_RECONEXIONES;

    loop {
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(Peticion::Apagar) => break,
            Ok(Peticion::Reconectar) => {
                if let Err(e) = reconectar(&mut motor, &estado) {
                    eprintln!("[audio] no se pudo reconectar: {e}");
                }
                ultima_reconexion = Instant::now();
                ultimo_latido = motor.latidos.load(Ordering::Relaxed);
            }
            Ok(p) => manejar(&mut motor, p),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        // Vigilancia del stream. Al enchufar unos cascos, conectar un Bluetooth o cambiar de
        // salida, el stream puede quedarse sin servirse: deja de latir y no vuelve solo. Antes
        // de esto, la única forma de recuperar el sonido era reiniciar la aplicación.
        if ultima_vigilancia.elapsed() >= VIGILANCIA {
            ultima_vigilancia = Instant::now();
            let latido = motor.latidos.load(Ordering::Relaxed);
            let motivo = motivo_para_reconectar(
                latido,
                ultimo_latido,
                motor.fallo.load(Ordering::Relaxed),
                &motor.dispositivo,
                dispositivo_por_defecto().as_deref(),
            );
            ultimo_latido = latido;

            if let Some(motivo) = motivo {
                if ultima_reconexion.elapsed() >= ESPERA_ENTRE_RECONEXIONES {
                    eprintln!("[audio] {motivo}; reconectando");
                    if let Err(e) = reconectar(&mut motor, &estado) {
                        eprintln!("[audio] no se pudo reconectar: {e}");
                    }
                    ultima_reconexion = Instant::now();
                    ultimo_latido = motor.latidos.load(Ordering::Relaxed);
                }
            }
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
            motor.gain = g;
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
        Peticion::Reconectar | Peticion::Apagar => {}
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

#[cfg(test)]
mod tests {
    use super::motivo_para_reconectar;

    #[test]
    fn un_stream_sano_no_se_toca() {
        // Late y el sistema sigue en el mismo sitio: reconectar aquí sonaría como un corte.
        assert!(motivo_para_reconectar(120, 100, false, "Altavoces", Some("Altavoces")).is_none());
    }

    #[test]
    fn cambiar_la_salida_del_sistema_obliga_a_reabrir() {
        // Este es el caso real: el stream late tan campante, pero escribe en los altavoces
        // mientras el sonido debería ir a los cascos. Sin esto, la app se queda muda hasta
        // que la reinicias.
        let m = motivo_para_reconectar(120, 100, false, "Altavoces", Some("Cascos Bluetooth"));
        assert_eq!(m, Some("el sistema cambió de salida"));
    }

    #[test]
    fn un_error_del_backend_obliga_a_reabrir() {
        let m = motivo_para_reconectar(120, 100, true, "Altavoces", Some("Altavoces"));
        assert_eq!(m, Some("el backend dio error"));
    }

    #[test]
    fn un_callback_que_deja_de_correr_obliga_a_reabrir() {
        let m = motivo_para_reconectar(100, 100, false, "Altavoces", Some("Altavoces"));
        assert_eq!(m, Some("el stream dejó de responder"));
    }

    #[test]
    fn sin_dispositivo_por_defecto_no_se_inventa_un_cambio() {
        // Si no se puede consultar el sistema (o no hay ninguno), el nombre no dice nada:
        // solo mandan el latido y el error. Lo contrario sería reconectar en bucle.
        assert!(motivo_para_reconectar(120, 100, false, "Altavoces", None).is_none());
        assert_eq!(
            motivo_para_reconectar(100, 100, false, "Altavoces", None),
            Some("el stream dejó de responder")
        );
    }

    #[test]
    fn el_cambio_de_salida_manda_sobre_los_demas_motivos() {
        // Importa para el mensaje del log: si además cambió la salida, eso es lo que hay
        // que contar, porque es lo que explica el silencio.
        let m = motivo_para_reconectar(100, 100, true, "Altavoces", Some("HDMI"));
        assert_eq!(m, Some("el sistema cambió de salida"));
    }
}
