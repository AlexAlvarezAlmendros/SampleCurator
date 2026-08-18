//! Percentiles e histograma ASCII. Sin dependencias.

pub fn percentile(sorted_us: &[f64], p: f64) -> f64 {
    if sorted_us.is_empty() {
        return 0.0;
    }
    let rank = (p / 100.0) * (sorted_us.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        sorted_us[lo]
    } else {
        sorted_us[lo] + (sorted_us[hi] - sorted_us[lo]) * (rank - lo as f64)
    }
}

pub struct Resumen {
    pub n: usize,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub min: f64,
    pub max: f64,
    pub media: f64,
}

pub fn resumir(valores: &[f64]) -> Resumen {
    let mut v = valores.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Resumen {
        n: v.len(),
        p50: percentile(&v, 50.0),
        p95: percentile(&v, 95.0),
        p99: percentile(&v, 99.0),
        min: v.first().copied().unwrap_or(0.0),
        max: v.last().copied().unwrap_or(0.0),
        media: if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 },
    }
}

/// Imprime un resumen con el veredicto frente a un presupuesto (en ms).
pub fn informe(titulo: &str, valores_ms: &[f64], presupuesto_p95_ms: f64) -> bool {
    let r = resumir(valores_ms);
    let ok = r.p95 <= presupuesto_p95_ms;
    println!("\n── {titulo} ──");
    println!(
        "  n={}  min={:.2} ms  p50={:.2} ms  p95={:.2} ms  p99={:.2} ms  max={:.2} ms  media={:.2} ms",
        r.n, r.min, r.p50, r.p95, r.p99, r.max, r.media
    );
    println!(
        "  presupuesto p95 ≤ {:.1} ms  →  {}",
        presupuesto_p95_ms,
        if ok { "✅ DENTRO" } else { "❌ FUERA" }
    );
    histograma(valores_ms);
    ok
}

pub fn histograma(valores: &[f64]) {
    if valores.is_empty() {
        return;
    }
    let r = resumir(valores);
    let bins = 12usize;
    let lo = r.min;
    let hi = if r.max > r.min { r.max } else { r.min + 1.0 };
    let ancho = (hi - lo) / bins as f64;
    let mut cuentas = vec![0usize; bins];
    for v in valores {
        let mut i = ((v - lo) / ancho) as usize;
        if i >= bins {
            i = bins - 1;
        }
        cuentas[i] += 1;
    }
    let pico = cuentas.iter().copied().max().unwrap_or(1).max(1);
    for (i, c) in cuentas.iter().enumerate() {
        let desde = lo + ancho * i as f64;
        let barra = (c * 40 / pico).max(if *c > 0 { 1 } else { 0 });
        println!("  {:>7.2} ms │{:<40}│ {}", desde, "█".repeat(barra), c);
    }
}
