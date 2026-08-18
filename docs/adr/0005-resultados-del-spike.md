# ADR-0005 — Resultados del spike de Fase 0: GO

**Fecha:** 2026-08-18 · **Estado:** aceptada · **Decisión:** **GO** (con un punto abierto)

## Contexto

La Fase 0 existía para responder una pregunta antes de construir nada: ¿se puede, en esta
máquina y con este stack, oír un sample a menos de 25 ms de pulsar la tecla, y escanear 50.000
archivos en menos de 60 s? El código de medida está en `spike/` y es desechable.

**Entorno medido:** ThinkPad · Ubuntu 24.04 · kernel 7.0.0-28 · 14 GB RAM · 16 hilos ·
PipeWire 1.0.5 (vía pipewire-pulse) · Rust 1.97.1 · disco NVMe.

**Biblioteca sintética:** 50.000 archivos, 4,2 GB, en 400 carpetas (50 packs × 8 categorías).
48.000 WAV + 500 FLAC + 500 MP3 + 500 OGG + 500 AIFF. Generarla: 6,2 s + 18,8 s de conversión.

---

## 1. Latencia de reproducción

### Camino de software (tecla → buffer de audio escrito)

| Buffer solicitado | p50 | p95 | p99 | max |
|---|---|---|---|---|
| 128 frames | 0,77 ms | **1,21 ms** | 1,28 ms | 1,31 ms |
| 256 frames | 1,44 ms | **2,59 ms** | 2,68 ms | 2,72 ms |
| 512 frames | 2,71 ms | **5,11 ms** | 5,49 ms | 5,52 ms |
| *por defecto del device* | 22,60 ms | **42,05 ms** | 42,54 ms | 42,75 ms |

**El hallazgo más importante del spike está en la última fila.** Dejar que cpal use el tamaño de
buffer por defecto del dispositivo cuesta 42 ms de p95 — casi el doble del presupuesto entero,
antes siquiera de contar el hardware. La app **debe** pedir `BufferSize::Fixed(256)`
explícitamente. Con 256 el margen es holgado y no hubo ni un xrun en ~15.000 callbacks.

### Con decodificación en el momento (sin caché de la app)

Buffer 256, decodificando el archivo al pulsar. Se midió dos veces con resultados muy distintos,
y la diferencia es informativa:

| Pasada | p50 | p95 | max | Estado de la caché de página del SO |
|---|---|---|---|---|
| Aislada (n=100) | 10,53 ms | **12,35 ms** | 13,02 ms | biblioteca poco tocada |
| Dentro de `all` (n=120) | 1,44 ms | **2,83 ms** | 4,23 ms | tras leer 2.000 archivos en los benches previos |

Es decir: **cuando el archivo ya está en la caché de página del sistema, decodificar sale casi
gratis (~0,2 ms sobre el camino base); cuando hay que ir al disco, cuesta ~9,8 ms.** Ambas cifras
están dentro del presupuesto de 40 ms, pero la relevante para el diseño es la primera, porque el
caso real del triaje es exactamente ese: recorrer por primera vez una biblioteca que el SO no ha
tocado nunca.

Eso es lo que compran la caché LRU y el prefetch de vecinos: hasta **9,8 ms de p95** en el
recorrido inicial, que es la diferencia entre "instantáneo" y "se nota". No es una optimización
opcional; es parte del diseño.

### Punto abierto: el offset constante de hardware

Las cifras de arriba miden hasta que el programa escribe el buffer. Falta el trozo constante
—PipeWire + DAC— que no he conseguido medir en esta máquina:

- cpal, sobre el backend ALSA, reporta `playback == callback`: el adelanto de reproducción sale
  siempre 0, así que su API no sirve para esto aquí.
- El micrófono del portátil devuelve un fondo de 0,42 a escala completa en silencio (entrada
  saturada o mala conversión s32→f32 en el puente ALSA): el loopback acústico no es fiable.
- Capturar el monitor del sink vía `PULSE_SOURCE` entrega ceros aunque el stream esté
  correctamente enganchado al sink por defecto (verificado con `pactl list sink-inputs`).

Lo que sí sabemos: PipeWire negocia `node.latency = 64/44100` = **1,45 ms** de cuanto para este
stream, lo que apunta a una ruta de baja latencia. Con 2,59 ms de software y un cuanto de 1,45 ms,
el total quedaría muy por debajo de 25 ms salvo que el buffer de hardware sea anormalmente grande.

**Cómo se cierra:** por oído, que es el criterio real del producto. `spike play --lib DIR` con los
altavoces encendidos: si al pulsar ↓ el sonido se siente simultáneo a la tecla, el presupuesto se
cumple. Y se vuelve a verificar en la Fase 3 con la app real (tarea 3.10, histograma en vivo).

## 2. Clics al cambiar de sample

| Medida | Salto máximo entre muestras consecutivas |
|---|---|
| Salto natural del material (seno 660 Hz @ 48 kHz) | 0,0864 |
| Con fade de 5 ms | **0,0864** |
| Sin fade (control negativo) | 0,9960 |

Con el fade de 5 ms el salto máximo es **idéntico al natural del material**: no queda ni una
discontinuidad, ni siquiera cambiando de sample cada 42 ms. El control negativo (0,996) demuestra
que el detector mide lo que dice medir.

## 3. Decodificación

n=600 archivos, mezcla de formatos:

| Formato | p50 | p95 | max |
|---|---|---|---|
| aiff | 0,06 ms | 0,17 ms | 0,19 ms |
| flac | 0,20 ms | 0,52 ms | 0,56 ms |
| mp3 | 0,43 ms | 1,06 ms | 1,20 ms |
| ogg | 0,84 ms | 1,50 ms | 1,64 ms |
| wav | 0,29 ms | 1,09 ms | 9,35 ms |
| **total** | **0,29 ms** | **1,09 ms** | 9,35 ms |

Symphonia cubre los cinco formatos sin sorpresas y con coste despreciable. AIFF —crítico en
librerías de samples— funciona.

Remuestreo 48.000 → 44.100 Hz con interpolación lineal: p50 0,22 ms, p95 1,38 ms (+217 % sobre
decodificar, pero se hace **una vez** al cargar, nunca en el callback). La app usará `rubato`
por calidad; esto establece el suelo de coste.

## 4. Disco

Escaneo e indexado de 50.000 archivos, **con las cachés de página tiradas** (`drop_caches`):

| Etapa | Tiempo |
|---|---|
| Recorrido del árbol (jwalk, paralelo) | 0,46 s |
| Inserción en SQLite (lotes de 1.000 en transacción) | 0,07 s |
| **Total** | **0,52 s** (95.903 archivos/s) |
| Índice resultante | 6,2 MB |

En caliente: 0,19 s. El presupuesto era 60 s: hay **115× de margen**.

Análisis (decodificar + 1.000 buckets de picos + hash blake3), 3.000 archivos en frío con 16
hilos: 0,22 s → 0,07 ms por archivo → **0,1 min extrapolado a 50.000**. El presupuesto era
10 minutos: hay **100× de margen**.

---

## Decisión

**GO.** La premisa se sostiene con holgura. Todo lo que depende de nuestro código está dentro de
presupuesto, y con margen suficiente para que el único trozo no medido (el offset de hardware)
tenga que ser absurdamente grande para tumbarlo.

## Constantes calibradas para la app

| Constante | Valor | Por qué |
|---|---|---|
| `BufferSize` | **`Fixed(256)`** | El default del device cuesta 42 ms de p95. No negociable. |
| Fade al cambiar de sample | **5 ms** | Elimina la discontinuidad por completo (medido). |
| Caché LRU + prefetch de vecinos | **obligatorio** | Ahorra 9,8 ms de p95. Sin esto no hay producto. |
| Hilos de análisis | 16 (todos) | El análisis va tan sobrado que puede competir por CPU sin molestar. |
| Lote de inserción SQLite | 1.000 en transacción | 0,07 s para 50.000 filas. |

## Presupuestos que se pueden apretar

Los objetivos de disco resultaron conservadores por dos órdenes de magnitud. Se actualizan en
`docs/PERFORMANCE.md`: escaneo de 50.000 ≤ **5 s** (era 60 s) y análisis completo ≤ **1 min**
(era 10 min). Apretarlos ahora evita que una regresión de 20× pase desapercibida más adelante.

## Consecuencias para el diseño

1. La arquitectura de `audio/` del ADR-0002 queda validada tal cual: hilo de control que
   decodifica y prepara `Arc<AudioBuffer>`, callback que solo mezcla, `rtrb` entre medias.
2. Se confirma un detalle que el spike descubrió por las malas: los `Arc` que dejan de sonar **no
   pueden soltarse en el callback** (soltar el último provoca una liberación de memoria en el
   hilo de tiempo real). El spike los devuelve al hilo de control por un ring buffer de basura;
   la app hará lo mismo.
3. El margen de disco permite subir la calidad del análisis (más buckets, loudness EBU R128 real
   en vez de RMS) sin tocar el presupuesto.
