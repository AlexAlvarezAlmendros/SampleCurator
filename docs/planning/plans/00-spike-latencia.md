# Plan 00 — Spike de latencia y escaneo (GATE)

> Fase: 0 de 7 | Estado: ✅ Hecho | 2026-08-18 | Veredicto: **GO** (ver [ADR-0005](../../adr/0005-resultados-del-spike.md))
> Hito: tecla → sonido < 25 ms (p95) y escaneo de 50.000 archivos < 60 s

---

## Por qué esta fase existe

Todo el valor del producto depende de una sensación: pulsas una flecha y **ya está sonando**.
Si eso no se consigue en esta máquina y con este stack, ninguna cantidad de buena UI lo arregla.
Se comprueba primero, con código desechable, antes de construir nada.

Este código vive en `spike/` y **no** se convierte en la app: se lee, se miden los números y se
tira. Lo que sobrevive son las conclusiones y las constantes calibradas (tamaño de buffer, tamaño
de caché, número de hilos de escaneo).

## Dependencia con otras fases

- **Requiere:** nada.
- **Habilita:** todas. Sin GO explícito no se empieza la Fase 1.

---

## Tareas

### Preparación
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 0.1 | Instalar toolchain Rust (rustup, stable) y dependencias de sistema de Tauri | ✅ Hecho | rustup 1.97.1 · webkit2gtk 2.52.3, ALSA 1.2.11 y cc 13.3 ya estaban |
| 0.2 | `spike/` como crate binario independiente (no es la app) | ✅ Hecho | `spike/` con `[workspace]` propio: no entrará en el workspace de la app |
| 0.3 | Generador de biblioteca sintética: N archivos WAV/FLAC/MP3 en árbol de carpetas | ✅ Hecho | 50.000 archivos · 4,2 GB · 6,2 s + 18,8 s de conversión con ffmpeg |

### Medición de audio
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 0.4 | Abrir stream `cpal` una vez y reproducir un buffer f32 desde un ring buffer SPSC | ✅ Hecho | stream abierto una vez + `rtrb` SPSC + ring de basura para los Arc |
| 0.5 | Decodificar con `symphonia` a `Vec<f32>` intercalado (WAV, AIFF, FLAC, MP3, OGG) | ✅ Hecho | wav, aiff, flac, mp3, ogg — los cinco sin sorpresas |
| 0.6 | Medir latencia tecla→primera muestra: p50/p95/p99 sobre 200 disparos | ✅ Hecho | p95 = 2,59 ms (buffer 256). Falta el offset de hardware: ver ADR-0005 |
| 0.7 | Medir coste de decodificación por formato y duración (one-shot vs loop de 8 s) | ✅ Hecho | p50 0,29 ms · p95 1,09 ms sobre 600 archivos mezclados |
| 0.8 | Probar retrigger rápido (10 disparos/s) y verificar ausencia de clics con fade de 5 ms | ✅ Hecho | con fade de 5 ms el salto máximo == el natural del material; control negativo 0,996 |
| 0.9 | Comprobar resampling con `rubato` cuando el device va a 48 kHz y el archivo a 44,1 | ⚠️ Parcial | Lineal medido (p95 1,38 ms). La validación de rubato pasa a la tarea 3.7 |

### Medición de disco
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 0.10 | Escaneo paralelo con `jwalk` sobre 50.000 archivos: medir tiempo y RAM | ✅ Hecho | 0,46 s para 50.000 en frío (drop_caches), 95.903 archivos/s |
| 0.11 | Inserción en SQLite por lotes de 1.000 en transacción: medir tiempo total | ✅ Hecho | 0,07 s para 50.000 filas en lotes de 1.000 |
| 0.12 | Cálculo de picos (1.000 buckets) sobre 1.000 archivos con `rayon`: extrapolar a 50k | ✅ Hecho | 0,07 ms/archivo con 16 hilos → 0,1 min extrapolado a 50.000 |

### Decisión
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 0.13 | Escribir `docs/adr/0005-resultados-spike.md` con los números reales | ✅ Hecho | docs/adr/0005-resultados-del-spike.md |
| 0.14 | **GO / NO-GO** y calibración de constantes (buffer, hilos, tamaño de caché) | ✅ Hecho | **GO** · Fixed(256) obligatorio, fade 5 ms, caché+prefetch obligatorios |

---

## Entregable

Un ejecutable de consola que reproduce samples de una carpeta con las flechas y escupe un
histograma de latencias, más un informe con los números de escaneo.

## Criterio de aceptación

- Latencia tecla→sonido **p95 < 25 ms** con el buffer en caché, y **p95 < 40 ms** en frío.
- Retrigger rápido sin clics audibles.
- 50.000 archivos listados e insertados en SQLite en **< 60 s**.
- Picos de 50.000 archivos calculados en **< 10 min** en background.

**Si la latencia p95 supera 40 ms en caché:** parar y evaluar alternativas (otro backend de audio,
buffer más pequeño, PipeWire directo) antes de seguir. Ese es el sentido del gate.

---

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | 0.1–0.3 | Toolchain instalado y biblioteca sintética de 50.000 archivos (4,2 GB) generada |
| 2026-08-18 | 0.4–0.8 | Motor de audio del spike: p95 2,59 ms de software con buffer 256; sin clics con fade de 5 ms |
| 2026-08-18 | 0.6 | Hallazgo crítico: con el buffer por defecto del device el p95 sube a 42 ms → `Fixed(256)` es obligatorio |
| 2026-08-18 | 0.6 | Descubierto también: soltar un `Arc<AudioBuffer>` en el callback libera memoria en el hilo de tiempo real → ring de basura hacia el hilo de control |
| 2026-08-18 | 0.9 | Remuestreo lineal 48k→44,1k: p95 1,38 ms. Validación de rubato movida a 3.7 |
| 2026-08-18 | 0.10–0.12 | Disco en frío: 0,52 s para escanear e indexar 50.000; análisis 0,1 min extrapolado |
| 2026-08-18 | 0.13–0.14 | ADR-0005 escrito. **GO** con un punto abierto: el offset constante de hardware |

---

## Punto abierto que hereda la Fase 3

El trozo constante de latencia (PipeWire + DAC) no se pudo medir en esta máquina: cpal reporta
adelanto 0, el micrófono devuelve basura (fondo de 0,42 a escala completa) y la captura del
monitor entrega ceros. Se cierra de dos maneras:

1. **Por oído, ahora:** `spike play --lib DIR` con los altavoces encendidos. Si al pulsar ↓ el
   sonido se siente simultáneo, el presupuesto se cumple — que es el criterio real del producto.
2. **Con instrumentos, en la Fase 3:** la tarea 3.10 (histograma de latencia en vivo) mide sobre
   la app real; si hace falta, con una grabación externa del altavoz.
