# SampleCurator — CLAUDE.md

App de escritorio para **triar librerías de samples**: recorrer miles de sonidos sueltos,
escucharlos al instante y, con **una sola tecla**, mandarlos a la carpeta destino correcta
o a la papelera.

Dos objetivos no negociables mandan sobre todo lo demás:

1. **Latencia percibida cero** — de pulsar tecla a oír sonido, < 25 ms. La lista nunca tironea.
2. **Una tecla por decisión** — cero clics en el bucle de triaje, cero modales, todo reversible.

Stack: Tauri 2 (Rust) + React 19 + TypeScript + SQLite · audio nativo con `cpal` + `symphonia`.

---

## Comandos esenciales

```bash
# Instalar
pnpm install

# Dev (Vite + Tauri con hot reload)
pnpm tauri dev

# Build de release (AppImage + .deb)
pnpm tauri build

# TypeScript
pnpm typecheck
pnpm biome check .
pnpm biome check --write .
pnpm vitest

# Rust (siempre desde la raíz, con --manifest-path)
cargo fmt --manifest-path src-tauri/Cargo.toml --all
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test  --manifest-path src-tauri/Cargo.toml
cargo bench --manifest-path src-tauri/Cargo.toml   # criterion: decode, scan, peaks

# Regenerar los tipos TS del backend (ts-rs los exporta al pasar los tests)
cargo test --manifest-path src-tauri/Cargo.toml --lib export
```

---

## Estructura

```
src/                        Frontend React (solo presentación)
  app/                      App.tsx, providers, atajos globales
  features/
    library/                lista virtualizada, búsqueda, filtros
    triage/                 bucle de triaje, destinos, undo
    player/                 transporte, waveform, controles
    settings/               carpetas, atajos, preferencias
      components/ hooks/ store.ts
  components/               primitivas de UI (Button, Kbd, Panel, Row…)
  styles/                   tokens.css · reset.css · global.css
  lib/                      ipc.ts · keymap.ts · format.ts
  bindings.ts               GENERADO por ts-rs — no editar a mano

src-tauri/
  src/
    lib.rs                  builder de Tauri + invoke_handler
    ipc/                    comandos Tauri (capa fina, sin lógica)
    domain/                 tipos puros del dominio (sin deps externas)
    codec/                  decodificar · remuestrear · picos · sonoridad · hash
    music/                  tipo de sample, BPM y tonalidad (Fase 8)
    paths.rs                rutas relativas normalizadas con `/` en todos los sistemas
    audio/                  engine (hilo + cpal) · cache (LRU) · graph (tiempo real)
    scan/                   walker + indexado · analyzer (en segundo plano)
    db/                     rusqlite: migrations · queries · triage (journal)
    fileops/                mover · trash · deshacer/rehacer/reparar
    error.rs
  Cargo.toml
  tauri.conf.json

docs/                       arquitectura, diseño, datos, ADRs
docs/planning/              roadmap y planes por fase (ver skill samplecurator-plan)
```

**Regla de dependencias — nunca romper:**

```
ipc      → domain, db, codec, audio, scan, fileops, music
audio    → domain, codec     (NO conoce db, NO conoce tauri)
scan     → domain, db, codec, music
fileops  → domain, db, paths
music    → domain, codec     (NO conoce db, ni el sistema de archivos)
db       → domain
codec    → domain
paths    → nada
domain   → nada
```

`codec` existe porque decodificar hace falta en dos sitios que no pueden conocerse entre sí:
`audio` (para reproducir) y `scan` (para analizar). Es la única forma de que `audio` siga sin
saber nada de la base de datos.

`domain` no importa nada del proyecto. `audio` jamás toca SQLite ni el sistema de archivos
salvo para leer bytes de un sample. Nada por debajo de `ipc` importa `tauri::`.

En el frontend: `features/*` puede usar `components/` y `lib/`; `components/` nunca importa de
`features/`. Ningún componente llama a `invoke()` directamente — solo a través de `src/lib/ipc.ts`.

---

## Versiones fijas

```
Rust          1.83+ (edición 2021)
Tauri         2.x
React         19.x
TypeScript    5.7+
Vite          6.x
Node.js       22 LTS
pnpm          9.x
Biome         1.x     (no ESLint, no Prettier)
Vitest        2.x     (no Jest)
SQLite        vía rusqlite 0.32+ con feature `bundled`
symphonia     0.5.x   (decodificación)
cpal          0.17.x  (salida de audio)
ts-rs         12.x    (tipos TS generados desde Rust — ver ADR-0006)
zustand       5.x
@tanstack/react-virtual  3.x
```

Sin `ethers`, sin Electron, sin librerías de componentes pesadas. Cada dependencia nueva se
justifica en el plan de fase antes de añadirla.

---

## Reglas de tiempo real (audio) — las más importantes del proyecto

El callback de `cpal` corre en un hilo de prioridad alta con un presupuesto de microsegundos.
Dentro del callback de audio, **prohibido**:

- reservar o liberar memoria (`Vec::new`, `push`, `clone`, `String`, `format!`, `Box`)
- bloquear (`Mutex::lock`, `RwLock`, canales bloqueantes)
- I/O de cualquier tipo, incluido `println!`, `log::*` y `dbg!`
- `unwrap()`, `expect()`, `panic!` — un panic aquí mata el stream de audio

Comunicación con el callback: **solo** ring buffer SPSC (`rtrb`), `triple_buffer` o atómicos.
Los buffers de audio llegan ya decodificados como `Arc<AudioBuffer>` preparados por el hilo de
control; el callback solo lee y mezcla.

Además:

- El stream de salida se abre **una vez** al arrancar y **no se cierra nunca**. Abrir el device
  por sample costaría 50-200 ms — es el error clásico que hace que una app así se sienta lenta.
- Todo cambio de ganancia o parada aplica una rampa de 5-10 ms (fade) para no producir clics.
- Cambiar de sample = fade-out de 5 ms del anterior + arranque inmediato del nuevo, sin esperar.

---

## Convenciones Rust

- `thiserror` para los errores de dominio; los comandos IPC devuelven `Result<T, AppError>` y
  `AppError` serializa a un objeto `{ kind, message }` — nunca un `String` suelto. El frontend
  distingue por `kind`, jamás parseando el texto.
- Nada de `unwrap()`/`expect()` fuera de tests y del arranque (`lib.rs` puede fallar rápido).
- Trabajo pesado (escaneo, decodificación, análisis) **nunca** en el hilo del comando IPC:
  va a un pool `rayon` o a un hilo dedicado, y reporta progreso por `tauri::ipc::Channel`.
- Los datos binarios grandes (peaks de waveform) viajan como bytes crudos con
  `tauri::ipc::Response::new(Vec<u8>)`, nunca como array JSON ni base64.
- SQLite en modo WAL, `synchronous = NORMAL`, una conexión por hilo vía `r2d2` o un `Mutex`
  en el pool de escritura; las lecturas nunca bloquean a las escrituras.
- Todo `INSERT` masivo va dentro de una transacción y con `prepare_cached`.
- Módulos con `mod.rs`; nada de archivos de más de ~400 líneas: si crece, se parte.

## Convenciones TypeScript / React

- `strict`, `exactOptionalPropertyTypes` y `noUncheckedIndexedAccess` activados — no desactivar.
- Los tipos del backend **se generan** con ts-rs a `src/bindings.ts`. No se escriben a mano ni se
  duplican interfaces del dominio. Los `i64` del dominio llevan `#[ts(type = "number")]` a
  propósito: el puente de Tauri serializa a JSON, donde llegan como `number` (ADR-0006).
- Estado de UI en Zustand con selectores atómicos (`useStore(s => s.x)`), nunca objetos nuevos
  en el selector: provoca re-render de toda la lista.
- La lista de samples es virtualizada y sus filas van memoizadas (`memo` + props primitivas).
  Una fila nunca recibe callbacks recreados por render.
- La waveform se pinta en `<canvas>`; el cabezal de reproducción se anima con `requestAnimationFrame`
  interpolando desde `startedAt`/`duration`. **Prohibido** un evento IPC por frame.
- Atajos de teclado: un único listener global declarativo en `src/lib/keymap.ts`. Sin librerías
  de hotkeys, sin `useEffect` con `addEventListener` repartidos por componentes.
- Nada de `any`. Nada de `console.log` en `src/` (usa el logger de `lib/`).

---

## Sistema de diseño (resumen — detalle en `docs/DESIGN_SYSTEM.md`)

- **Dark-first**, alta densidad, contraste alto, un único color de acento.
- Tokens CSS en tres capas: primitivos (`--gray-11`) → semánticos (`--color-text-muted`) →
  de componente (`--row-height`). Los componentes **solo** usan semánticos o de componente.
- **Prohibido** cualquier `#hex`, `rgb()` o `hsl()` literal fuera de `src/styles/tokens.css`.
- CSS Modules (`Componente.module.css`) junto al componente. Sin CSS-in-JS en runtime.
- El anillo de foco es siempre visible: la app se usa con el teclado.
- Movimiento: 80 ms micro-interacción, 140 ms paneles, solo `opacity`/`transform`.
  Respeta `prefers-reduced-motion`.

---

## Rendimiento — presupuestos (se miden, no se estiman)

| Métrica | Presupuesto |
|---|---|
| Tecla → primer sample de audio | < 25 ms |
| Navegar a la siguiente fila (con autoplay) | < 16 ms de frame |
| Escaneo inicial de 50.000 archivos | < 60 s (análisis en background) |
| Búsqueda incremental sobre 100.000 samples | < 50 ms |
| Arranque en frío de la app | < 1,2 s |
| RAM en reposo con 50.000 samples indexados | < 250 MB |

La caché de audio decodificado es LRU con tope en bytes (por defecto 256 MB), y hace prefetch
de los ±3 vecinos de la selección: cuando pulsas la flecha, el sample **ya está en RAM**.

---

## Multiplataforma

El objetivo es Linux y Windows con el mismo código. Tres sitios donde se separan, y cómo:

- **Rutas.** El índice guarda siempre `kicks/snare.wav`, con barra hacia delante, también en
  Windows (`crate::paths`). Para abrir el archivo da igual, pero hace que todo lo que compara
  cadenas —podar, buscar, exportar, el `UNIQUE` de la tabla— funcione igual en los dos.
- **Buffer de audio.** ALSA deja fijarlo (256 frames, 2,6 ms); WASAPI no, y allí manda el del
  sistema. Se **pregunta** al backend antes de construir el stream, porque el callback solo se
  puede mover una vez y un reintento obligaría a rehacerlo entero.
- **Prioridad de hilos.** `nice(+10)` en Unix, `THREAD_PRIORITY_BELOW_NORMAL` en Windows. Misma
  idea: el análisis puede esperar, el audio no.

El CI ejecuta clippy y los tests del núcleo en los dos sistemas, porque la mitad de este código
toca rutas, renombrados y prioridades, que es justo donde se separan.

## Seguridad de los datos del usuario — no negociable

Esta app mueve y borra archivos de la biblioteca personal del usuario. Un bug aquí destruye
trabajo irrecuperable.

- **Nunca** `std::fs::remove_file` sobre un sample del usuario. Los rechazados van a la papelera
  gestionada (`<destino>/.samplecurator-trash/`) con manifiesto para restaurar.
- Toda operación de archivo se escribe **antes** en el journal (`actions`) y se marca completada
  **después**: si la app muere a mitad, el arranque detecta y repara.
- Mover entre dispositivos = copiar + verificar tamaño/hash + borrar origen. Nunca borrar antes
  de verificar la copia.
- Colisión de nombres: sufijo ` (2)`, ` (3)`… Jamás sobrescribir un archivo existente.
- Undo/redo ilimitado dentro de la sesión, y persistido en el journal entre sesiones.
- El índice SQLite es una caché reconstruible: perderlo nunca implica perder audio.

---

## Testing

**Rust**
```
src-tauri/tests/          integración: scan → índice → mover → undo
#[cfg(test)] en módulo    unitarios de decode, peaks, journal, rutas
benches/ (criterion)      decode, cálculo de peaks, escaneo
```
- El motor de audio se testea con un *host* falso: el grafo produce muestras a un buffer en
  memoria, sin device real. Los tests verifican fades, ganancia y ausencia de clics.
- `fileops` se testea siempre sobre `tempfile::TempDir`. Ningún test toca rutas reales.

**Frontend (Vitest + Testing Library)**
- Tests de hooks y del keymap con `renderHook`; el módulo `lib/ipc.ts` se mockea entero.
- No se testean primitivas de UI; se testea la lógica de selección, el keymap y los reducers.

---

## Flujo de trabajo

Toda petición de desarrollo pasa primero por la skill **`samplecurator-plan`**: lee
`docs/planning/ROADMAP.md`, localiza el plan de fase en `docs/planning/plans/`, marca la tarea
en curso, implementa y actualiza el estado al terminar.

Skills del proyecto (en `.claude/skills/`):

| Skill | Cuándo |
|---|---|
| `samplecurator-plan` | Al inicio de cualquier petición de desarrollo |
| `samplecurator-comando-ipc` | Añadir o cambiar un comando Tauri (Rust + bindings + wrapper TS) |
| `samplecurator-componente` | Crear un componente React nuevo |
| `samplecurator-audio-rt` | Tocar cualquier cosa dentro de `src-tauri/src/audio/` |

## Decisiones ya tomadas — no volver a abrirlas sin un ADR nuevo

| Decisión | Dónde |
|---|---|
| `BufferSize::Fixed(256)`; el default del device cuesta 42 ms de p95 | [ADR-0005](docs/adr/0005-resultados-del-spike.md) |
| Fade de 5 ms al cambiar de sample; medido, elimina el clic por completo | ADR-0005 |
| Los `Arc` retirados vuelven al hilo de control por un ring de basura | ADR-0005 |
| Tipos TS con ts-rs, no tauri-specta (v2 no existe como estable) | [ADR-0006](docs/adr/0006-contrato-ipc-y-remuestreo.md) |
| Remuestreo con sinc propio testeado, no rubato (caso offline y de ratio fijo) | ADR-0006 |
| Dos reglas a11y de Biome desactivadas, con motivo, en `biome.json` | ADR-0006 |

## Git

Commits en español, imperativo y con ámbito: `audio: abrir el stream una sola vez al arrancar`.
Sin atribución a la IA en commits ni PRs (hay un hook que lo bloquea).
