# Rendimiento — presupuestos y técnicas

Una app de triaje que tarda 200 ms en reproducir es inservible: el usuario deja de usarla a la
tercera sesión sin saber explicar por qué. Por eso el rendimiento aquí no es una optimización
posterior, es un **requisito funcional con números**.

## 1. Presupuestos

| Métrica | Objetivo | Límite duro | Cómo se mide |
|---|---|---|---|
| Tecla → primer sample de audio | 15 ms | **25 ms** | timestamp en el comando IPC → timestamp en el callback de cpal |
| Frame al navegar por la lista | 8 ms | **16 ms** | Performance panel del WebView, escenario de 50k filas |
| Escaneo de 50.000 archivos (listado visible) | 1 s | **5 s** | bench de integración con árbol sintético |
| Análisis completo de 50.000 archivos (background) | 15 s | 1 min | ídem |
| Búsqueda incremental sobre 100.000 samples | 20 ms | **50 ms** | bench de la consulta FTS |
| Arranque en frío hasta ventana visible | 400 ms | 1,2 s | `console.time` desde `main()` |
| RAM en reposo, 50.000 samples indexados | 180 MB | **250 MB** | RSS del proceso |
| Tamaño del `.deb` | 8 MB | 15 MB | salida de `tauri build` |
| Tamaño del AppImage | 80 MB | 120 MB | ídem — empaqueta WebKitGTK entero, por eso pesa 10× más |

> Los presupuestos de disco se apretaron 12× y 10× tras la Fase 0: los originales (60 s y 10 min)
> resultaron ser dos órdenes de magnitud más flojos que la realidad medida, y con ellos una
> regresión de 20× habría pasado desapercibida. Ver [ADR-0005](adr/0005-resultados-del-spike.md).

Si un cambio empeora cualquiera de estos números, no entra. Los benchmarks de `criterion` viven
en `src-tauri/benches/` y se ejecutan antes de cerrar cada fase.

## 1.ter Medido sobre la app terminada (2026-08-18, misma máquina)

50.000 samples reales (4,2 GB: wav, flac, mp3, ogg, aiff), `tests/escala.rs`:

| Métrica | Presupuesto | Medido | |
|---|---|---|---|
| Escaneo + índice de 50.000, cachés tiradas | 5 s | **0,52 s** | ✅ |
| Análisis completo de 50.000 (decode + picos + hash) | 1 min | **18 s** | ✅ |
| Página de 200 filas en la peor posición | 50 ms | **16 ms** | ✅ |
| Búsqueda incremental (4 términos) | 50 ms | **3,1–4,0 ms** | ✅ |
| `.deb` | 15 MB | **6,3 MB** | ✅ |
| AppImage | 120 MB | **78 MB** | ✅ |
| Underruns del stream analizando 50.000 mientras suena | 0 | **0** (3 vueltas) | ✅ |

**Dos cosas que solo aparecieron al medir la app de verdad:**

1. **La página costaba 82 ms**, no 16. Faltaban índices que cubrieran el `ORDER BY … COLLATE
   NOCASE`: SQLite ordenaba las 50.000 filas enteras en cada página. Con
   `idx_samples_orden_path` y sus hermanos, 16 ms. Lo encontró el test de escala, no el ojo.

2. **El analizador provocaba un underrun del audio.** Sus 16 hilos de rayon competían con el
   hilo de audio *dentro del mismo proceso* (con la CPU saturada desde fuera no pasaba: es
   competencia interna, no del sistema). El analizador usa ahora un pool propio con dos núcleos
   menos y `nice(+10)`: 0 underruns en tres vueltas, a cambio de un 7 % menos de velocidad de
   análisis — que sobra, porque va 3× por debajo de su presupuesto.

## 1.bis Medido en la Fase 0 (2026-08-18, ThinkPad · Ubuntu 24.04 · PipeWire 1.0.5)

| Métrica | Presupuesto | Medido | |
|---|---|---|---|
| Tecla → buffer de audio escrito (buffer 256) | 25 ms | **2,59 ms** p95 | ✅ |
| Ídem sin caché, decodificando en el momento | 40 ms | **12,35 ms** p95 en frío · 2,83 ms si el SO ya tiene el archivo | ✅ |
| Ídem con el buffer por defecto del device | 25 ms | **42,05 ms** p95 | ❌ → por eso `Fixed(256)` |
| Salto al cambiar de sample (fade 5 ms) | sin clics | **0,0864** = el natural del material | ✅ |
| Escaneo + índice de 50.000 en frío | 5 s | **0,52 s** | ✅ |
| Análisis de 50.000 (extrapolado, 16 hilos) | 1 min | **~0,1 min** | ✅ |
| Decodificación (wav/aiff/flac/mp3/ogg) | 15 ms | **1,09 ms** p95 | ✅ |

**Pendiente de medir:** el offset constante de hardware (PipeWire + DAC). Detalle y plan de
cierre en [ADR-0005](adr/0005-resultados-del-spike.md) §1.

## 2. Las siete técnicas que sostienen los números

**1. El stream de audio se abre una vez.** Abrir un device ALSA/CoreAudio cuesta 50-200 ms.
Se abre al arrancar la app y se mantiene en silencio. Reproducir = escribir en un ring buffer.

**2. Caché LRU de audio decodificado + prefetch de vecinos.** La decodificación (2-10 ms para un
one-shot) se hace *antes* de que el usuario llegue al sample. Con navegación secuencial, el
acierto de caché es prácticamente del 100 %.

**3. Progreso por lotes, nunca por elemento.** El escaneo emite ~10 mensajes por segundo con
totales acumulados. Un mensaje IPC por archivo saturaría el puente y el WebView pasaría el día
haciendo `JSON.parse`.

**4. Binario crudo para los datos densos.** Los picos de waveform viajan como `Vec<u8>`
(`tauri::ipc::Response`), no como JSON. 2 KB por sample en vez de 12 KB, y cero parseo.

**5. Virtualización con altura de fila fija.** `@tanstack/react-virtual` solo monta las filas
visibles (~35). La altura fija evita medir y permite saltar a cualquier índice en O(1).

**6. El cabezal de reproducción no cruza el puente IPC.** El front recibe `startedAt` y
`duration` una vez y lo interpola con `requestAnimationFrame` fuera de React (ref + canvas).
60 eventos IPC por segundo es la forma más fácil de arruinar esta app.

**7. SQLite en WAL con transacciones por lotes.** Los inserts del escaneo van en tandas de 1.000
dentro de una transacción con `prepare_cached`. Lecturas y escrituras no se bloquean entre sí, así
que la lista sigue respondiendo mientras se indexa.

## 3. Trampas conocidas

| Trampa | Síntoma | Prevención |
|---|---|---|
| Alloc en el callback de audio | clics y micro-cortes aleatorios | prohibido por convención; hook de revisión sobre `src-tauri/src/audio/` |
| Selector de Zustand que devuelve un objeto nuevo | la lista entera se repinta al mover el foco | selectores atómicos, `useShallow` cuando haga falta |
| Callbacks recreados pasados a las filas | `memo` inútil, 35 filas repintadas por tecla | callbacks estables (`useCallback` con deps vacías) o dispatch por id |
| Reanalizar en cada arranque | 40 s de disco al abrir | comparar `(size, mtime)` antes de abrir nada |
| `SELECT *` sobre 100.000 filas | la UI se atraganta al filtrar | paginación por ventana y `LIMIT`/`OFFSET` con índice |
| Fuentes cargadas de red | salto de layout al arrancar | `.woff2` empaquetados y subsetados |
| Animar la selección | sensación de lentitud aunque el frame sea rápido | la selección no se anima, nunca |

## 4. Cómo se verifica

```bash
# Micro-benchmarks del núcleo
cargo bench --manifest-path src-tauri/Cargo.toml

# Escenario sintético: genera un árbol de N WAV y mide escaneo + análisis
cargo test --manifest-path src-tauri/Cargo.toml --release -- --ignored bench_scan_50k

# Latencia de reproducción (imprime el histograma p50/p95/p99)
cargo test --manifest-path src-tauri/Cargo.toml --release -- --ignored bench_play_latency
```

En el frontend, el escenario de referencia es una biblioteca sintética de 50.000 filas: se navega
100 posiciones con `↓` y ningún frame puede pasar de 16 ms.
