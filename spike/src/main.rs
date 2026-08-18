//! SPIKE de SampleCurator — código desechable.
//!
//! Responde a una sola pregunta antes de construir la app:
//! ¿se puede, en ESTA máquina, oír un sample a menos de 25 ms de pulsar la tecla, y escanear
//! 50.000 archivos en menos de 60 s?
//!
//!   cargo run --release -- gen --out DIR [--count 50000] [--convert-pct 4]
//!   cargo run --release -- bench-latency --lib DIR [--shots 200] [--buffer 256] [--cold]
//!   cargo run --release -- bench-decode    --lib DIR [--files 400]
//!   cargo run --release -- bench-retrigger
//!   cargo run --release -- bench-scan      --lib DIR
//!   cargo run --release -- bench-peaks     --lib DIR [--files 1000]
//!   cargo run --release -- play            --lib DIR   (interactivo: ↓ ↑ espacio q)
//!   cargo run --release -- all             --lib DIR

#![allow(dead_code)]

mod decode;
mod engine;
mod gen;
mod scan;
mod stats;

use anyhow::{bail, Result};
use engine::{ahora_ns, Engine, Graph};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const PRESUPUESTO_LATENCIA_MS: f64 = 25.0;
const PRESUPUESTO_LATENCIA_FRIA_MS: f64 = 40.0;
const PRESUPUESTO_ESCANEO_S: f64 = 60.0;
const OBJETIVO_ARCHIVOS: f64 = 50_000.0;

struct Args(Vec<String>);

impl Args {
    fn opt(&self, nombre: &str) -> Option<String> {
        let pos = self.0.iter().position(|a| a == nombre)?;
        self.0.get(pos + 1).cloned()
    }
    fn num(&self, nombre: &str, defecto: usize) -> usize {
        self.opt(nombre)
            .and_then(|v| v.parse().ok())
            .unwrap_or(defecto)
    }
    fn flag(&self, nombre: &str) -> bool {
        self.0.iter().any(|a| a == nombre)
    }
    fn lib(&self) -> Result<PathBuf> {
        match self.opt("--lib") {
            Some(v) => Ok(PathBuf::from(v)),
            None => bail!("falta --lib DIR (biblioteca sintética; créala con `gen`)"),
        }
    }
}

fn main() -> Result<()> {
    let todos: Vec<String> = std::env::args().skip(1).collect();
    let cmd = todos.first().cloned().unwrap_or_else(|| "help".into());
    let args = Args(todos);

    match cmd.as_str() {
        "gen" => {
            let out = args
                .opt("--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./lib-sintetica"));
            gen::generar(&out, args.num("--count", 50_000), args.num("--convert-pct", 4))?;
        }
        "bench-latency" => {
            bench_latencia(&args.lib()?, args.num("--shots", 200), buffer(&args), args.flag("--cold"))?;
        }
        "bench-decode" => {
            bench_decode(&args.lib()?, args.num("--files", 400))?;
        }
        "probe" => {
            probe(args.num("--secs", 8), buffer(&args))?;
        }
        "bench-resample" => {
            bench_resample(&args.lib()?, args.num("--files", 300))?;
        }
        "mic-check" => {
            mic_check(args.num("--secs", 4))?;
        }
        "bench-loopback" => {
            bench_loopback(args.num("--shots", 20), buffer(&args))?;
        }
        "bench-retrigger" => {
            bench_retrigger()?;
        }
        "bench-scan" => {
            bench_scan(&args.lib()?)?;
        }
        "bench-peaks" => {
            bench_peaks(&args.lib()?, args.num("--files", 1000))?;
        }
        "play" => {
            interactivo(&args.lib()?, buffer(&args))?;
        }
        "all" => {
            let lib = args.lib()?;
            let mut veredictos = Vec::new();
            veredictos.push(("escaneo + índice", bench_scan(&lib)?));
            veredictos.push(("análisis (picos + hash)", bench_peaks(&lib, args.num("--files", 1000))?));
            veredictos.push(("decodificación", bench_decode(&lib, args.num("--files", 400))?));
            veredictos.push(("retrigger sin clics", bench_retrigger()?));
            veredictos.push((
                "latencia (caché caliente)",
                bench_latencia(&lib, args.num("--shots", 200), buffer(&args), false)?,
            ));
            veredictos.push((
                "latencia (en frío)",
                bench_latencia(&lib, args.num("--shots", 100), buffer(&args), true)?,
            ));

            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║  VEREDICTO DEL SPIKE                                     ║");
            println!("╚══════════════════════════════════════════════════════════╝");
            for (nombre, ok) in &veredictos {
                println!("  {:<34} {}", nombre, if *ok { "✅" } else { "❌" });
            }
            let go = veredictos.iter().all(|v| v.1);
            println!(
                "\n  → {}",
                if go {
                    "GO: la premisa se sostiene en esta máquina."
                } else {
                    "NO-GO: revisa los apartados en ❌ antes de seguir."
                }
            );
        }
        _ => {
            println!("{}", include_str!("main.rs").lines().take(16).collect::<Vec<_>>().join("\n"));
        }
    }
    Ok(())
}

fn buffer(args: &Args) -> Option<u32> {
    match args.opt("--buffer") {
        Some(v) if v == "default" => None,
        Some(v) => v.parse().ok(),
        None => Some(256),
    }
}

fn elegir(archivos: &[PathBuf], n: usize) -> Vec<PathBuf> {
    if archivos.is_empty() {
        return Vec::new();
    }
    let paso = (archivos.len() / n.max(1)).max(1);
    archivos.iter().step_by(paso).take(n).cloned().collect()
}

// ───────────────────────────── latencia ─────────────────────────────

fn bench_latencia(lib: &Path, disparos: usize, buffer: Option<u32>, frio: bool) -> Result<bool> {
    let archivos = scan::listar_audio(lib);
    if archivos.is_empty() {
        bail!("no hay archivos de audio en {}", lib.display());
    }
    let mut motor = Engine::nuevo(buffer)?;
    motor.gain(0.15); // suena, pero sin atronar: la medida no depende del volumen
    println!(
        "\nDispositivo: {} · {} Hz · {} canales · buffer: {}",
        motor.dispositivo, motor.sample_rate, motor.canales, motor.buffer_frames
    );

    let elegidos = elegir(&archivos, disparos + 10);
    let sr = motor.sample_rate;

    // Calentamiento: 10 disparos que se descartan (primer toque del device, cachés de página).
    let precargados: Vec<Arc<decode::AudioBuffer>> = if frio {
        Vec::new()
    } else {
        elegidos
            .iter()
            .filter_map(|p| decode::decodificar_a(p, sr).ok())
            .collect()
    };

    let intervalo = Duration::from_millis(120);
    let calentamiento = 10.min(elegidos.len());

    for i in 0..calentamiento {
        disparar(&mut motor, &elegidos, &precargados, i, frio, sr);
        std::thread::sleep(intervalo);
        motor.vaciar_basura();
    }
    let _ = motor.recoger_latencias_ms();

    let total = if frio {
        elegidos.len()
    } else {
        precargados.len()
    };
    for i in calentamiento..total {
        disparar(&mut motor, &elegidos, &precargados, i, frio, sr);
        std::thread::sleep(intervalo);
        motor.vaciar_basura();
    }
    std::thread::sleep(Duration::from_millis(200));

    let (adelantos, desconocidos) = motor.recoger_adelantos_ms();
    let ra = stats::resumir(&adelantos);
    println!(
        "\n  Diagnóstico del backend: adelanto de reproducción reportado por cpal\n             n={}  p50={:.2} ms  p95={:.2} ms  max={:.2} ms   ·  sin dato: {}",
        ra.n, ra.p50, ra.p95, ra.max, desconocidos
    );
    if ra.n == 0 || ra.p50 == 0.0 {
        println!(
            "    ⚠️ El backend NO reporta el adelanto real: la cifra de abajo mide el camino\n                 de software (tecla → buffer escrito), NO lo que sale por el altavoz."
        );
    }

    let latencias = motor.recoger_latencias_ms();
    let presupuesto = if frio {
        PRESUPUESTO_LATENCIA_FRIA_MS
    } else {
        PRESUPUESTO_LATENCIA_MS
    };
    let titulo = if frio {
        "Latencia tecla → sonido — EN FRÍO (incluye decodificación)"
    } else {
        "Latencia tecla → sonido — CACHÉ CALIENTE"
    };
    println!("\n  (incluye el adelanto de reproducción que reporta cpal: es lo que se oye,");
    println!("   no solo lo que tarda el programa en escribir el buffer)");
    Ok(stats::informe(titulo, &latencias, presupuesto))
}

fn disparar(
    motor: &mut Engine,
    rutas: &[PathBuf],
    precargados: &[Arc<decode::AudioBuffer>],
    i: usize,
    frio: bool,
    sr: u32,
) {
    if frio {
        let Some(ruta) = rutas.get(i) else { return };
        let t = ahora_ns();
        if let Ok(buf) = decode::decodificar_a(ruta, sr) {
            motor.play_desde(buf, false, t);
        }
    } else if let Some(buf) = precargados.get(i) {
        motor.play(buf.clone(), false);
    }
}

// ───────────────────────────── decodificación ─────────────────────────────

fn bench_decode(lib: &Path, n: usize) -> Result<bool> {
    let archivos = elegir(&scan::listar_audio(lib), n);
    if archivos.is_empty() {
        bail!("no hay archivos en {}", lib.display());
    }
    let mut por_formato: HashMap<String, Vec<f64>> = HashMap::new();
    let mut duracion_total = 0.0f64;

    for p in &archivos {
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("?")
            .to_ascii_lowercase();
        let t0 = Instant::now();
        if let Ok(buf) = decode::decodificar(p) {
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            duracion_total += buf.duracion_s();
            por_formato.entry(ext).or_default().push(ms);
        }
    }

    println!("\n── Coste de decodificación por formato ──");
    let mut formatos: Vec<_> = por_formato.iter().collect();
    formatos.sort_by_key(|(k, _)| (*k).clone());
    let mut todas = Vec::new();
    for (fmt, ms) in formatos {
        let r = stats::resumir(ms);
        println!(
            "  {:<6} n={:<5} p50={:>7.2} ms  p95={:>7.2} ms  max={:>7.2} ms",
            fmt, r.n, r.p50, r.p95, r.max
        );
        todas.extend(ms.iter().copied());
    }
    let r = stats::resumir(&todas);
    println!(
        "  {:<6} n={:<5} p50={:>7.2} ms  p95={:>7.2} ms  max={:>7.2} ms   ({:.1} s de audio)",
        "TOTAL", r.n, r.p50, r.p95, r.max, duracion_total
    );
    // Presupuesto: decodificar un one-shot debe caber holgadamente en el margen de 25 ms.
    let ok = r.p95 <= 15.0;
    println!(
        "  presupuesto p95 ≤ 15 ms (para que quepa en la latencia en frío)  →  {}",
        if ok { "✅ DENTRO" } else { "❌ FUERA" }
    );
    Ok(ok)
}

// ───────────────────────────── retrigger / clics ─────────────────────────────

fn seno(sr: u32, canales: u16, freq: f32, dur: f32) -> Arc<decode::AudioBuffer> {
    let frames = (sr as f32 * dur) as usize;
    let ch = canales as usize;
    let mut s = vec![0.0f32; frames * ch];
    for f in 0..frames {
        let v = (std::f32::consts::TAU * freq * f as f32 / sr as f32).sin();
        for c in 0..ch {
            s[f * ch + c] = v;
        }
    }
    Arc::new(decode::AudioBuffer {
        samples: s,
        channels: canales,
        sample_rate: sr,
        nombre: format!("seno{freq}"),
    })
}

/// Dispara sin parar y mide el salto máximo entre muestras consecutivas.
/// Con senos, un salto muy por encima del natural (TAU·f/sr) es un clic audible.
fn retrigger_max_delta(fade_ms: f32) -> f32 {
    let (sr, ch) = (48_000u32, 2u16);
    let mut g = Graph::nuevo(sr, ch, fade_ms, None);
    let a = seno(sr, ch, 440.0, 1.0);
    let b = seno(sr, ch, 660.0, 1.0);
    let bloque = 256usize;
    let mut out = vec![0.0f32; bloque * ch as usize];
    let mut previa = 0.0f32;
    let mut max_delta = 0.0f32;

    for i in 0..200 {
        if i % 8 == 0 {
            g.limpiar_arranques();
            g.aplicar(engine::Cmd::Play {
                buf: if i % 16 == 0 { a.clone() } else { b.clone() },
                t_send_ns: 0,
                id: i as u64,
                looping: false,
            });
        }
        g.process(&mut out);
        for f in 0..bloque {
            let v = out[f * ch as usize];
            let d = (v - previa).abs();
            if i > 0 && d > max_delta {
                max_delta = d;
            }
            previa = v;
        }
    }
    max_delta
}

fn bench_retrigger() -> Result<bool> {
    // Salto natural de un seno de 660 Hz a 48 kHz entre muestras consecutivas.
    let natural = std::f32::consts::TAU * 660.0 / 48_000.0;
    let con_fade = retrigger_max_delta(5.0);
    let sin_fade = retrigger_max_delta(0.02);

    println!("\n── Retrigger rápido (cambio de sample cada 42 ms) ──");
    println!("  salto natural del material (seno 660 Hz @48k): {natural:.4}");
    println!("  salto máximo CON fade de 5 ms:                 {con_fade:.4}");
    println!("  salto máximo SIN fade (control negativo):      {sin_fade:.4}");
    let ok = con_fade < natural * 3.0;
    println!(
        "  criterio: con fade < 3× el salto natural  →  {}",
        if ok { "✅ SIN CLICS" } else { "❌ HAY CLICS" }
    );
    if sin_fade < natural * 3.0 {
        println!("  ⚠️ el control negativo no detecta clics: la medida no es concluyente");
        return Ok(false);
    }
    Ok(ok)
}

// ───────────────────────────── disco ─────────────────────────────

fn bench_scan(lib: &Path) -> Result<bool> {
    println!("\n── Escaneo e indexado ──");
    let (entradas, s_recorrido) = scan::recorrer(lib);
    let db = std::env::temp_dir().join("samplecurator-spike-index.db");
    let s_indexado = scan::indexar(&entradas, &db)?;
    let total = s_recorrido + s_indexado;
    let n = entradas.len() as f64;
    let extrapolado = if n > 0.0 { total * OBJETIVO_ARCHIVOS / n } else { 0.0 };

    println!("  archivos encontrados: {}", entradas.len());
    println!("  recorrido del árbol : {s_recorrido:.2} s");
    println!("  inserción en SQLite : {s_indexado:.2} s  (lotes de 1.000 en transacción)");
    println!("  total               : {total:.2} s  ({:.0} archivos/s)", n / total.max(1e-9));
    println!("  extrapolado a 50.000: {extrapolado:.1} s");
    println!(
        "  índice en disco     : {:.1} MB",
        std::fs::metadata(&db).map(|m| m.len()).unwrap_or(0) as f64 / 1e6
    );
    let ok = extrapolado <= PRESUPUESTO_ESCANEO_S;
    println!(
        "  presupuesto ≤ {PRESUPUESTO_ESCANEO_S:.0} s para 50.000  →  {}",
        if ok { "✅ DENTRO" } else { "❌ FUERA" }
    );
    Ok(ok)
}

fn bench_peaks(lib: &Path, n: usize) -> Result<bool> {
    println!("\n── Análisis en background: decodificar + picos + hash ──");
    let archivos = elegir(&scan::listar_audio(lib), n);
    let a = scan::analizar(&archivos);
    let por_archivo = a.segundos / a.ok.max(1) as f64;
    let extrapolado_min = por_archivo * OBJETIVO_ARCHIVOS / 60.0;

    println!("  analizados          : {} ({} fallos)", a.ok, a.fallos);
    println!("  tiempo              : {:.2} s  ({} hilos rayon)", a.segundos, rayon::current_num_threads());
    println!("  por archivo         : {:.2} ms", por_archivo * 1000.0);
    println!("  audio decodificado  : {:.2} GB", a.bytes_audio as f64 / 1e9);
    println!("  extrapolado a 50.000: {extrapolado_min:.1} min");
    let ok = extrapolado_min <= 10.0;
    println!(
        "  presupuesto ≤ 10 min para 50.000  →  {}",
        if ok { "✅ DENTRO" } else { "❌ FUERA" }
    );
    Ok(ok)
}

// ───────────────────────────── modo interactivo ─────────────────────────────

fn interactivo(lib: &Path, buffer: Option<u32>) -> Result<()> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    let archivos = scan::listar_audio(lib);
    if archivos.is_empty() {
        bail!("no hay archivos en {}", lib.display());
    }
    let mut motor = Engine::nuevo(buffer)?;
    motor.gain(0.5);
    let sr = motor.sample_rate;
    let mut cache: HashMap<usize, Arc<decode::AudioBuffer>> = HashMap::new();
    let mut sel = 0usize;
    let mut latencias: Vec<f64> = Vec::new();

    println!(
        "Dispositivo: {} · {} Hz · {} canales · buffer {}",
        motor.dispositivo, motor.sample_rate, motor.canales, motor.buffer_frames
    );
    println!("{} samples.  ↓/↑ navegar (suena solo) · espacio repetir · q salir\n", archivos.len());

    enable_raw_mode()?;
    let resultado = (|| -> Result<()> {
        loop {
            // prefetch de vecinos: al llegar, el buffer ya está decodificado
            for d in -3i64..=3 {
                let i = sel as i64 + d;
                if i >= 0 && (i as usize) < archivos.len() {
                    let i = i as usize;
                    if !cache.contains_key(&i) {
                        if let Ok(b) = decode::decodificar_a(&archivos[i], sr) {
                            cache.insert(i, b);
                        }
                    }
                }
            }
            if cache.len() > 64 {
                let lejanos: Vec<usize> = cache
                    .keys()
                    .copied()
                    .filter(|k| k.abs_diff(sel) > 8)
                    .collect();
                for k in lejanos {
                    cache.remove(&k);
                }
            }
            motor.vaciar_basura();

            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down | KeyCode::Char('j') => {
                        sel = (sel + 1).min(archivos.len() - 1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        sel = sel.saturating_sub(1);
                    }
                    KeyCode::Char(' ') => {}
                    _ => continue,
                }
                if let Some(buf) = cache.get(&sel) {
                    motor.play(buf.clone(), false);
                } else if let Ok(b) = decode::decodificar_a(&archivos[sel], sr) {
                    motor.play(b.clone(), false);
                    cache.insert(sel, b);
                }
                std::thread::sleep(Duration::from_millis(12));
                latencias.extend(motor.recoger_latencias_ms());
                let ultima = latencias.last().copied().unwrap_or(0.0);
                print!(
                    "\r[{:>5}/{}] {:<48} {:>6.1} ms   ",
                    sel + 1,
                    archivos.len(),
                    archivos[sel]
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    ultima
                );
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
        }
        Ok(())
    })();
    disable_raw_mode()?;
    println!();
    if !latencias.is_empty() {
        stats::informe("Latencia medida en la sesión interactiva", &latencias, PRESUPUESTO_LATENCIA_MS);
    }
    resultado
}


// ───────────────────────────── latencia real de salida ─────────────────────────────

/// Mantiene un tono en bucle N segundos para poder interrogar a PipeWire desde fuera
/// (`pactl list sink-inputs`), que sí sabe cuánto buffer hay por delante.
fn probe(segundos: usize, buffer: Option<u32>) -> Result<()> {
    let mut motor = Engine::nuevo(buffer)?;
    motor.gain(0.08);
    println!(
        "Dispositivo: {} · {} Hz · {} canales · buffer {}",
        motor.dispositivo, motor.sample_rate, motor.canales, motor.buffer_frames
    );
    let tono = seno(motor.sample_rate, motor.canales, 220.0, 0.5);
    motor.play(tono, true);
    println!("Sonando {segundos} s — interroga a PipeWire ahora.");
    for _ in 0..segundos {
        std::thread::sleep(Duration::from_secs(1));
        motor.vaciar_basura();
    }
    Ok(())
}

/// Un click: ataque instantáneo, 8 ms, fácil de detectar en el micro.
fn click(sr: u32, canales: u16) -> Arc<decode::AudioBuffer> {
    let frames = (sr as f32 * 0.008) as usize;
    let ch = canales as usize;
    let mut s = vec![0.0f32; frames * ch];
    for f in 0..frames {
        let env = 1.0 - (f as f32 / frames as f32);
        let v = (std::f32::consts::TAU * 2000.0 * f as f32 / sr as f32).sin() * env;
        for c in 0..ch {
            s[f * ch + c] = v;
        }
    }
    Arc::new(decode::AudioBuffer {
        samples: s,
        channels: canales,
        sample_rate: sr,
        nombre: "click".into(),
    })
}

/// Loopback acústico: altavoz → aire → micro. Es la ÚNICA medida end-to-end de verdad.
/// Lo que sale incluye salida + vuelo por el aire + ENTRADA, así que es una cota superior
/// de la latencia de salida, no la latencia de salida exacta.
fn bench_loopback(disparos: usize, buffer: Option<u32>) -> Result<bool> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let entrada = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("no hay dispositivo de entrada"))?;
    let cfg_in = entrada.default_input_config()?;
    if cfg_in.sample_format() != cpal::SampleFormat::F32 {
        bail!("la entrada no acepta f32 ({:?})", cfg_in.sample_format());
    }
    let sr_in = cfg_in.sample_rate().0;
    let ch_in = cfg_in.channels() as usize;
    println!(
        "\nEntrada: {} · {} Hz · {} canales",
        entrada.name().unwrap_or_else(|_| "?".into()),
        sr_in,
        ch_in
    );

    let (mut pico_tx, mut pico_rx) = rtrb::RingBuffer::<(u64, f32)>::new(8192);
    let mut cfg: cpal::StreamConfig = cfg_in.into();
    cfg.buffer_size = cpal::BufferSize::Fixed(256);

    let stream_in = entrada.build_input_stream(
        &cfg,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let t = ahora_ns();
            let mut pico = 0.0f32;
            let mut idx = 0usize;
            for (i, s) in data.iter().enumerate() {
                let a = s.abs();
                if a > pico {
                    pico = a;
                    idx = i;
                }
            }
            let desfase_ns = (idx / ch_in.max(1)) as u64 * 1_000_000_000 / sr_in.max(1) as u64;
            let _ = pico_tx.push((t + desfase_ns, pico));
        },
        |e| eprintln!("error de entrada: {e}"),
        None,
    )?;
    stream_in.play()?;

    let mut motor = Engine::nuevo(buffer)?;
    motor.gain(0.9);
    let clic = click(motor.sample_rate, motor.canales);

    // El arranque del stream escupe bloques basura: se descarta el primer segundo entero.
    std::thread::sleep(Duration::from_millis(1000));
    while pico_rx.pop().is_ok() {}
    // Ruido de fondo real: 800 ms, y se usa la MEDIANA, no el máximo (un golpe puntual
    // en la mesa no puede dejar el umbral inalcanzable).
    std::thread::sleep(Duration::from_millis(800));
    let mut fondos = Vec::new();
    while let Ok((_, p)) = pico_rx.pop() {
        fondos.push(p as f64);
    }
    let rf = stats::resumir(&fondos);
    let fondo = rf.p50 as f32;
    let umbral = (fondo * 8.0).clamp(0.02, 0.6);
    println!(
        "  ruido de fondo: p50={:.4} p95={:.4} max={:.4} (n={}) · umbral: {umbral:.4}",
        rf.p50, rf.p95, rf.max, rf.n
    );
    if rf.p50 > 0.5 {
        println!("  ⚠️ La entrada está saturada o devuelve basura: la medida no será fiable.");
    }
    println!("  van a sonar {disparos} clicks por el altavoz…");

    let mut medidas = Vec::new();
    let mut perdidos = 0usize;
    for _ in 0..disparos {
        while pico_rx.pop().is_ok() {}
        let t_send = motor.play(clic.clone(), false);
        let limite = Instant::now() + Duration::from_millis(400);
        let mut detectado = None;
        while Instant::now() < limite && detectado.is_none() {
            while let Ok((t, p)) = pico_rx.pop() {
                if p > umbral && t > t_send {
                    detectado = Some(t);
                    break;
                }
            }
            std::thread::sleep(Duration::from_micros(500));
        }
        match detectado {
            Some(t) => medidas.push((t - t_send) as f64 / 1_000_000.0),
            None => perdidos += 1,
        }
        std::thread::sleep(Duration::from_millis(250));
        motor.vaciar_basura();
    }

    if medidas.is_empty() {
        println!(
            "\n  ❌ No se detectó ningún click. Comprueba que los altavoces suenan y que el\n     \
             micrófono no está silenciado (o que no hay auriculares puestos)."
        );
        return Ok(false);
    }
    println!("  detectados {} de {} ({perdidos} perdidos)", medidas.len(), disparos);
    println!("  ⚠️ Esta cifra incluye salida + aire + ENTRADA: es una COTA SUPERIOR de la");
    println!("     latencia de salida, no la latencia de salida exacta.");
    Ok(stats::informe(
        "Loopback acústico (tecla → altavoz → micro)",
        &medidas,
        PRESUPUESTO_LATENCIA_MS,
    ))
}


/// Diagnóstico rápido de la entrada: ¿qué está oyendo realmente el micrófono?
fn mic_check(segundos: usize) -> Result<()> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let entrada = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("no hay dispositivo de entrada"))?;
    let cfg_in = entrada.default_input_config()?;
    println!(
        "Entrada: {} · {:?} · {} Hz · {} canales",
        entrada.name().unwrap_or_else(|_| "?".into()),
        cfg_in.sample_format(),
        cfg_in.sample_rate().0,
        cfg_in.channels()
    );
    if cfg_in.sample_format() != cpal::SampleFormat::F32 {
        bail!("la entrada no acepta f32");
    }
    let (mut tx, mut rx) = rtrb::RingBuffer::<f32>::new(8192);
    let cfg: cpal::StreamConfig = cfg_in.into();
    let stream = entrada.build_input_stream(
        &cfg,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut pico = 0.0f32;
            for s in data {
                let a = s.abs();
                if a > pico {
                    pico = a;
                }
            }
            let _ = tx.push(pico);
        },
        |e| eprintln!("error de entrada: {e}"),
        None,
    )?;
    stream.play()?;
    for s in 0..segundos {
        std::thread::sleep(Duration::from_millis(1000));
        let mut v = Vec::new();
        while let Ok(p) = rx.pop() {
            v.push(p as f64);
        }
        let r = stats::resumir(&v);
        println!(
            "  segundo {}: n={:<4} p50={:.4} p95={:.4} max={:.4}",
            s + 1,
            r.n,
            r.p50,
            r.p95,
            r.max
        );
    }
    Ok(())
}


/// Coste de remuestrear: compara decodificar tal cual vs decodificar + llevar a 44.100 Hz.
fn bench_resample(lib: &Path, n: usize) -> Result<bool> {
    let archivos = elegir(&scan::listar_audio(lib), n * 4);
    let mut solo_decode = Vec::new();
    let mut con_resample = Vec::new();
    let mut iguales = 0usize;

    for p in &archivos {
        let Ok(buf) = decode::decodificar(p) else { continue };
        if buf.sample_rate == 44_100 {
            iguales += 1;
            continue;
        }
        // archivo a 48 kHz: medimos las dos rutas
        let t0 = Instant::now();
        let b = decode::decodificar(p)?;
        solo_decode.push(t0.elapsed().as_secs_f64() * 1000.0);

        let t1 = Instant::now();
        let r = decode::remuestrear_lineal(b, 44_100);
        con_resample.push(t1.elapsed().as_secs_f64() * 1000.0);
        // comprobación de que el resultado tiene la duración correcta
        debug_assert!((r.duracion_s() - buf.duracion_s()).abs() < 0.01);
        if con_resample.len() >= n {
            break;
        }
    }

    println!("\n── Coste del remuestreo (48.000 → 44.100 Hz, interpolación lineal) ──");
    if con_resample.is_empty() {
        println!("  no hay archivos con frecuencia distinta en la muestra ({iguales} coincidían)");
        return Ok(true);
    }
    let rd = stats::resumir(&solo_decode);
    let rr = stats::resumir(&con_resample);
    println!("  decodificar        : p50={:.2} ms  p95={:.2} ms  (n={})", rd.p50, rd.p95, rd.n);
    println!("  remuestrear        : p50={:.2} ms  p95={:.2} ms", rr.p50, rr.p95);
    println!("  sobrecoste total   : +{:.0} % sobre decodificar", 100.0 * rr.p50 / rd.p50.max(1e-9));
    let ok = rr.p95 < 5.0;
    println!(
        "  presupuesto p95 ≤ 5 ms (se hace una sola vez, en el hilo de control)  →  {}",
        if ok { "✅ DENTRO" } else { "❌ FUERA" }
    );
    println!("  nota: la app usará rubato (calidad); esto mide el suelo de coste. Ver tarea 3.7.");
    Ok(ok)
}
