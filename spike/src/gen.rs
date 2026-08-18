//! Generador de biblioteca sintética: un árbol de packs con miles de samples plausibles.
//!
//! Determinista (xorshift sembrado por índice): dos ejecuciones dan la misma biblioteca.

use anyhow::Result;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

const CATEGORIAS: [&str; 8] = [
    "kicks", "snares", "hats", "perc", "fx", "loops", "vox", "bass",
];

struct Rng(u64);

impl Rng {
    fn nuevo(semilla: u64) -> Self {
        Rng(semilla.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn siguiente(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn unidad(&mut self) -> f32 {
        (self.siguiente() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unidad()
    }
    fn indice(&mut self, n: usize) -> usize {
        (self.siguiente() % n.max(1) as u64) as usize
    }
}

struct Plan {
    ruta: PathBuf,
    categoria: usize,
    semilla: u64,
    sample_rate: u32,
    canales: u16,
    bits: u16,
}

/// Sintetiza un sample plausible según la categoría. Devuelve muestras intercaladas en f32.
fn sintetizar(cat: usize, rng: &mut Rng, sr: u32, canales: u16) -> Vec<f32> {
    let nombre = CATEGORIAS[cat];
    let dur = match nombre {
        "kicks" | "snares" | "perc" => rng.entre(0.12, 0.45),
        "hats" => rng.entre(0.05, 0.18),
        "fx" => rng.entre(0.4, 1.2),
        "loops" => rng.entre(1.8, 3.5),
        "vox" => rng.entre(0.3, 0.9),
        _ => rng.entre(0.5, 1.5), // bass
    };
    let frames = (sr as f32 * dur) as usize;
    let ch = canales as usize;
    let mut out = vec![0.0f32; frames * ch];
    let f0 = match nombre {
        "kicks" => rng.entre(45.0, 70.0),
        "bass" => rng.entre(55.0, 110.0),
        "snares" => rng.entre(180.0, 260.0),
        "perc" => rng.entre(300.0, 900.0),
        "hats" => rng.entre(6000.0, 11000.0),
        "vox" => rng.entre(160.0, 320.0),
        "fx" => rng.entre(400.0, 3000.0),
        _ => rng.entre(80.0, 200.0),
    };
    let decaimiento = match nombre {
        "kicks" | "perc" => rng.entre(12.0, 30.0),
        "hats" => rng.entre(40.0, 90.0),
        "snares" => rng.entre(10.0, 25.0),
        "fx" => rng.entre(1.5, 5.0),
        _ => rng.entre(2.0, 6.0),
    };
    let ruido = match nombre {
        "hats" => 0.9,
        "snares" => 0.6,
        "fx" => 0.4,
        _ => 0.05,
    };
    let periodo_loop = if nombre == "loops" { (sr as f32 * rng.entre(0.35, 0.6)) as usize } else { 0 };

    let mut fase = 0.0f32;
    for f in 0..frames {
        let t_local = if periodo_loop > 0 { (f % periodo_loop) as f32 } else { f as f32 };
        let env = (-decaimiento * t_local / sr as f32).exp();
        // barrido descendente en los graves, típico de un kick
        let freq = if nombre == "kicks" { f0 * (1.0 + 2.5 * env) } else { f0 };
        fase += std::f32::consts::TAU * freq / sr as f32;
        if fase > std::f32::consts::TAU {
            fase -= std::f32::consts::TAU;
        }
        let tono = fase.sin();
        let n = rng.unidad() * 2.0 - 1.0;
        let v = ((1.0 - ruido) * tono + ruido * n) * env * 0.8;
        for c in 0..ch {
            let ancho = if ch > 1 && c == 1 { 0.92 } else { 1.0 };
            out[f * ch + c] = (v * ancho).clamp(-1.0, 1.0);
        }
    }
    out
}

fn escribir_wav(plan: &Plan) -> Result<()> {
    let mut rng = Rng::nuevo(plan.semilla);
    let muestras = sintetizar(plan.categoria, &mut rng, plan.sample_rate, plan.canales);
    let spec = hound::WavSpec {
        channels: plan.canales,
        sample_rate: plan.sample_rate,
        bits_per_sample: plan.bits,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&plan.ruta, spec)?;
    if plan.bits == 24 {
        for s in &muestras {
            w.write_sample((s * 8_388_607.0) as i32)?;
        }
    } else {
        for s in &muestras {
            w.write_sample((s * 32_767.0) as i16)?;
        }
    }
    w.finalize()?;
    Ok(())
}

pub fn generar(salida: &Path, total: usize, convert_pct: usize) -> Result<()> {
    let packs = 50usize;
    let por_carpeta = (total / (packs * CATEGORIAS.len())).max(1);
    let t0 = std::time::Instant::now();

    let mut planes: Vec<Plan> = Vec::with_capacity(total);
    let mut idx = 0u64;
    for p in 0..packs {
        for (ci, cat) in CATEGORIAS.iter().enumerate() {
            let dir = salida.join(format!("pack_{p:02}")).join(cat);
            std::fs::create_dir_all(&dir)?;
            for n in 0..por_carpeta {
                let mut rng = Rng::nuevo(idx.wrapping_add(7));
                let sample_rate = if rng.indice(10) == 0 { 48_000 } else { 44_100 };
                let canales = if rng.indice(4) == 0 { 2 } else { 1 };
                let bits = if rng.indice(10) == 0 { 24 } else { 16 };
                planes.push(Plan {
                    ruta: dir.join(format!("{}_{:04}.wav", cat.to_uppercase(), n)),
                    categoria: ci,
                    semilla: idx,
                    sample_rate,
                    canales,
                    bits,
                });
                idx += 1;
            }
        }
    }

    println!("Generando {} archivos en {}…", planes.len(), salida.display());
    let errores: usize = planes
        .par_iter()
        .map(|p| if escribir_wav(p).is_err() { 1 } else { 0 })
        .sum();

    let bytes: u64 = planes
        .par_iter()
        .map(|p| std::fs::metadata(&p.ruta).map(|m| m.len()).unwrap_or(0))
        .sum();

    println!(
        "  {} archivos · {:.2} GB · {:.1} s · {} errores",
        planes.len(),
        bytes as f64 / 1e9,
        t0.elapsed().as_secs_f64(),
        errores
    );

    if convert_pct > 0 {
        convertir(&planes, convert_pct)?;
    }
    Ok(())
}

/// Convierte una fracción a otros formatos con ffmpeg, para que el bench de decodificación
/// no mida solo WAV.
fn convertir(planes: &[Plan], pct: usize) -> Result<()> {
    if std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_err()
    {
        println!("  (ffmpeg no disponible: la biblioteca queda solo en WAV)");
        return Ok(());
    }
    let formatos = ["flac", "mp3", "ogg", "aiff"];
    let paso = (100 / pct.max(1)).max(1);
    let objetivo: Vec<(usize, &Plan)> = planes
        .iter()
        .enumerate()
        .filter(|(i, _)| i % paso == 0)
        .collect();

    println!("  Convirtiendo {} archivos a flac/mp3/ogg/aiff…", objetivo.len());
    let t0 = std::time::Instant::now();
    objetivo.par_iter().for_each(|(i, p)| {
        let fmt = formatos[i % formatos.len()];
        let destino = p.ruta.with_extension(fmt);
        let ok = std::process::Command::new("ffmpeg")
            .args(["-loglevel", "error", "-y", "-i"])
            .arg(&p.ruta)
            .arg(&destino)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            let _ = std::fs::remove_file(&p.ruta);
        }
    });
    println!("  conversión: {:.1} s", t0.elapsed().as_secs_f64());
    Ok(())
}
