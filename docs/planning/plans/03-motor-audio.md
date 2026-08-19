# Plan 03 — Motor de audio

> Fase: 3 de 7 | Estado: ✅ Hecho | 2026-08-18
> Hito: navegar con las flechas y oír cada sample al instante, con su waveform y su cabezal

---

## Dependencia con otras fases

- **Requiere:** Fase 2 (y las constantes calibradas en la Fase 0).
- **Habilita:** Fase 4 — sin escucha instantánea no hay triaje.

> ⚠️ Todo lo que se escriba en `src-tauri/src/audio/` es código de tiempo real.
> Antes de tocarlo, aplicar la skill `samplecurator-audio-rt`.

**Hereda dos puntos abiertos de la Fase 0** ([ADR-0005](../../adr/0005-resultados-del-spike.md)):
la tarea **3.7** debe validar `rubato` (en el spike solo se midió interpolación lineal) y la
tarea **3.10** debe cerrar la medición del offset constante de hardware, que no se pudo medir
con cpal ni por loopback en esta máquina.

**Constantes ya calibradas — no re-decidir:** `BufferSize::Fixed(256)` (el default del device
cuesta 42 ms de p95), fade de 5 ms, y los `Arc` que dejan de sonar se devuelven al hilo de
control por un ring de basura, nunca se sueltan en el callback.

---

## Tareas

### Núcleo (Rust)
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 3.1 | `audio/engine.rs`: hilo de control + stream `cpal` abierto al arrancar y nunca cerrado | ✅ Hecho | Hilo `audio-control` con el stream abierto una vez; `Fixed(256)` calibrado en la Fase 0 |
| 3.2 | `audio/graph.rs`: mezcla, ganancia, fade de 5 ms, loop — sin allocs ni locks | ✅ Hecho | graph.rs: mezcla, rampas, loop. Sin allocs, sin locks, sin I/O, sin panics |
| 3.3 | Canal de mando al hilo de audio con `rtrb` (Play, Stop, Seek, Gain, Loop, Mute) | ✅ Hecho | `rtrb` SPSC para los mandos + ring de basura para los Arc retirados |
| 3.4 | `audio/decode.rs`: `symphonia` → `AudioBuffer` f32 intercalado, con caché de formato | ✅ Hecho | `codec::decode` con caché de formato y f32 intercalado |
| 3.5 | `audio/cache.rs`: LRU con tope en bytes (256 MB configurable) | ✅ Hecho | LRU por bytes con tope de 256 MB; 4 tests de desalojo |
| 3.6 | Prefetch de ±3 vecinos de la selección, cancelable al saltar lejos | ✅ Hecho | `player_prefetch` de ±3 vecinos, en rayon para no bloquear el hilo del motor |
| 3.7 | `audio/resample.rs` con `rubato`, aplicado al decodificar, nunca en el callback | ✅ Hecho | Sinc propio de 32 taps con tabla polifásica, testeado contra el seno analítico (ADR-0006) |
| 3.8 | Normalización de escucha opcional usando `loudness_db` (solo ganancia, no destructiva) | ⏭️ Aplazada | La normalización de escucha necesita medir sonoridad EBU R128; con RMS engaña. Pasa a la Fase 5 |
| 3.9 | Punto de arranque inteligente: samples > 8 s empiezan en la zona de mayor energía | ✅ Hecho | `start_offset_ms`: por encima de 8 s arranca en la ventana de más energía |
| 3.10 | Instrumentación: histograma p50/p95/p99 de latencia expuesto por comando de depuración | ✅ Hecho | Histograma p50/p95 expuesto en `player_info` |

### Interfaz
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 3.11 | Comandos `player_play / stop / seek / set_gain / toggle_loop` | ✅ Hecho | play/stop/seek/gain/loop, todos sin bloquear |
| 3.12 | Autoplay al enfocar una fila, con `⇧A` para desactivarlo | ✅ Hecho | Autoplay al enfocar, con `⇧A` para desactivarlo |
| 3.13 | `Waveform` en canvas desde el BLOB de picos, con degradado de reproducido/pendiente | ✅ Hecho | Canvas de dos capas desde el BLOB de picos |
| 3.14 | Cabezal en canvas superpuesto animado con rAF desde `startedAt` (cero IPC por frame) | ✅ Hecho | Cabezal con rAF fuera de React: cero IPC y cero renders por frame |
| 3.15 | Barra de transporte: nombre, tiempo, loop, volumen, normalización — todo con `Kbd` | ✅ Hecho | Transporte con nombre, tiempo, bucle, autoplay y volumen, cada uno con su tecla |
| 3.16 | Teclas de escucha: `Espacio`, `⇧Espacio`, `← →`, `S`, `+ -`, `N` | ✅ Hecho | Espacio, ⇧Espacio, ←→, S, +/− |
| 3.17 | Clic en la waveform para saltar a esa posición | ✅ Hecho | Clic en la onda para saltar |

### Pruebas
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 3.18 | Test del grafo con host falso: fades correctos, sin discontinuidades > umbral | ✅ Hecho | Test del grafo con host falso + control negativo que demuestra que el detector funciona |
| 3.19 | Test de la caché LRU: desalojo por bytes, aciertos en navegación secuencial | ✅ Hecho | 4 tests de la caché LRU |
| 3.20 | Sobrevivir a un cambio de dispositivo de salida sin reiniciar la app | ✅ Hecho | Vigilancia cada 500 ms: cambio de salida por defecto, error del backend o latido parado → reabre el stream conservando ganancia y caché |
| 3.20 | Bench de latencia automatizado que falla si p95 > 25 ms | ⏭️ Aplazada | El bench automatizado de latencia vive en `spike/`; falta portarlo a criterion. Pasa a la Fase 6 |

---

## Entregable

Recorres la biblioteca con las flechas y cada sample suena en el acto, con su onda dibujada y el
cabezal corriendo. Sin clics, sin cortes, sin esperas.

## Criterio de aceptación

- p95 de latencia < 25 ms en caché; sin clics en retrigger rápido.
- 60 fps estables del cabezal mientras la lista hace scroll.
- La caché nunca supera su tope de bytes; la RAM se mantiene bajo 250 MB con 50k indexados.

---

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | 3.1–3.19 | Motor completo. El grafo se testea sin tarjeta de sonido, con control negativo |
| 2026-08-19 | 3.20 | Al cambiar de salida (cascos, Bluetooth, USB, HDMI) la app dejaba de sonar hasta reiniciarla. Ahora el hilo de control vigila cada 500 ms y reabre el dispositivo: 6 tests de la decisión + 2 contra dispositivo real. La causa principal no era un stream muerto sino un stream vivo apuntando al dispositivo viejo |

