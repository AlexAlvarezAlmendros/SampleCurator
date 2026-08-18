---
name: samplecurator-audio-rt
description: "Use this skill BEFORE touching anything under src-tauri/src/audio/ in the SampleCurator project (/home/poio/Documentos/GIT/SampleCurator) — the real-time audio engine. Triggers on 'motor de audio', 'reproducción', 'cpal', 'callback de audio', 'latencia', 'clics en el audio', 'decodificar sample', 'caché de audio', 'prefetch', 'fade', 'loop', 'symphonia', 'rubato'. Encodes the hard real-time rules (no allocation, no locks, no I/O, no panics in the callback), the control-thread/callback split, the LRU cache and prefetch design, and how to test audio without a sound card."
metadata:
  version: 1.0.0
---

# Motor de audio — código de tiempo real

`src-tauri/src/audio/` es la parte del proyecto donde un error no se ve: **se oye**, como un clic,
un corte o un retardo de 80 ms que hace que la app "se sienta rara" sin que nadie sepa por qué.
Estas reglas no son estilo, son corrección.

---

## 1. Los dos mundos

```
┌── hilo de control (normal) ──────────┐      ┌── callback de cpal (tiempo real) ──┐
│ decodifica con symphonia             │      │ lee del ring buffer                 │
│ resamplea con rubato                 │ ───► │ mezcla y aplica ganancia            │
│ gestiona la caché LRU                │ rtrb │ aplica la rampa de fade             │
│ hace prefetch de vecinos             │      │ escribe en el buffer de salida      │
│ puede reservar memoria y bloquear    │      │ NO reserva, NO bloquea, NO panica   │
└──────────────────────────────────────┘      └─────────────────────────────────────┘
```

Todo lo caro ocurre **antes**, en el hilo de control. El callback solo consume lo ya preparado.

## 2. Prohibido dentro del callback

| Prohibido | Por qué | Qué hacer |
|---|---|---|
| `Vec::new`, `push`, `vec![]`, `String`, `format!`, `Box::new`, `.collect()`, `.to_vec()` | el asignador puede bloquear milisegundos | preasigna en el hilo de control y pasa `Arc<AudioBuffer>` |
| `Mutex::lock`, `RwLock`, canales bloqueantes | inversión de prioridad → cortes | `rtrb` (SPSC), `triple_buffer`, atómicos |
| `println!`, `log::*`, `dbg!`, cualquier I/O | syscalls impredecibles | acumula en un contador atómico y publícalo desde fuera |
| `unwrap()`, `expect()`, `panic!`, indexación que pueda salirse | un panic aquí mata el stream de audio | `get()`, saturación, valores por defecto |
| `.clone()` de datos de audio | copia oculta | clona el `Arc`, nunca el contenido |
| Bucles de duración no acotada | el callback tiene un presupuesto de microsegundos | trabajo proporcional al tamaño del bloque, y punto |

El hook `check-convenciones.py` detecta estos patrones en `graph.rs` y te los devuelve.

## 3. Reglas de diseño del motor

**El stream se abre una vez.** En el arranque de la app, y se mantiene abierto en silencio hasta
que se cierra. Abrir un device cuesta 50-200 ms: hacerlo por sample destruiría el producto.

**Un solo camino para los mandos.** El hilo de control envía comandos por `rtrb`:
`Play(Arc<AudioBuffer>, start_frame)`, `Stop`, `Seek`, `SetGain`, `SetLoop`, `Mute`. El callback
los consume sin bloquear (`pop()` no bloqueante) al principio de cada bloque.

**Cambiar de sample = fade-out de 5 ms + arranque inmediato.** No se espera a que termine el
anterior, no se corta en seco. La rampa se aplica por muestra con un incremento precalculado.

**Nada de ganancias en escalón.** Cualquier cambio de volumen se interpola en 5-10 ms.

**El estado se publica hacia fuera con atómicos.** Posición actual, sample sonando, xruns: el
callback escribe `AtomicU64`/`AtomicUsize`, el hilo de control los lee cuando quiere. Cero locks.

**La caché es LRU por bytes, no por número de elementos.** Un one-shot de 0,3 s y un loop de 30 s
no ocupan lo mismo. Tope configurable (256 MB por defecto), desalojo por antigüedad de uso.

**El prefetch va detrás de la selección, no delante del usuario.** Al cambiar el foco se encolan
los ±3 vecinos; si el usuario salta lejos, la cola anterior se cancela. Nunca se decodifica la
biblioteca entera "por si acaso".

**Resamplear al decodificar, jamás en el callback.** Si el device va a 48 kHz y el archivo a
44,1, `rubato` actúa una sola vez y el resultado se cachea ya convertido.

**El cabezal no cruza el puente IPC.** El front interpola desde `startedAt` y `duration`.
Si alguna vez hace falta la posición real (drift), se consulta a 4 Hz, no a 60.

## 4. Cómo se prueba sin tarjeta de sonido

El grafo (`graph.rs`) es una función pura sobre buffers: recibe comandos y un buffer de salida, y
lo rellena. Los tests lo ejercitan directamente:

```rust
#[test]
fn cambiar_de_sample_no_produce_discontinuidad() {
    let mut graph = Graph::new(48_000, 2);
    graph.push_command(Cmd::Play(buffer_seno(), 0));
    let mut out = vec![0.0f32; 4096];
    graph.process(&mut out);

    graph.push_command(Cmd::Play(buffer_ruido(), 0));
    graph.process(&mut out);

    // ninguna diferencia entre muestras consecutivas por encima del umbral audible
    assert!(max_delta(&out) < 0.05, "clic detectado al cambiar de sample");
}
```

Qué se testea siempre: fades correctos, ausencia de discontinuidades, ganancia interpolada, loop
sin salto en el punto de bucle, y desalojo correcto de la caché LRU.

Qué se mide (bench, no test): p50/p95/p99 de latencia tecla→primera muestra. **El bench falla si
p95 > 25 ms.**

## 5. Diagnóstico de síntomas

| Síntoma | Causa habitual |
|---|---|
| Clics al cambiar de sample | falta el fade de 5 ms, o se reinicia el stream |
| Micro-cortes aleatorios | alloc o lock dentro del callback |
| Primer disparo lento y el resto rápido | falta el prefetch, o la caché desaloja demasiado pronto |
| Todo va con ~80 ms de retraso | buffer del device demasiado grande, o se está reabriendo el stream |
| El audio se para del todo | un panic en el callback mató el stream |
| Tono agudo o grave | mezcla de sample rates sin resamplear |

## 6. Checklist antes de dar por hecho un cambio en `audio/`

- [ ] El callback no reserva, no bloquea, no hace I/O y no puede panicar
- [ ] Los cambios de estado llegan por `rtrb`; el estado sale por atómicos
- [ ] Toda transición de ganancia tiene rampa
- [ ] La caché respeta su tope en bytes
- [ ] Hay test del grafo con host falso para el comportamiento nuevo
- [ ] `cargo bench` de latencia dentro de presupuesto (p95 < 25 ms)
- [ ] Probado a oído: 20 disparos rápidos seguidos sin un solo clic
