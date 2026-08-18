# Arquitectura

> Documento vivo. Toda decisión estructural nueva se registra además como ADR en `docs/adr/`.

## 1. Forma general

SampleCurator es **un binario nativo** con dos mundos bien separados:

```
┌─────────────────────────── Proceso Tauri ────────────────────────────┐
│                                                                       │
│   WebView (React 19 + TS)                Núcleo Rust                  │
│   ───────────────────────                ───────────                  │
│   · pinta listas y waveforms      IPC    · escanea el disco           │
│   · captura el teclado          ◄─────►  · decodifica audio           │
│   · mantiene estado de UI      invoke    · reproduce (cpal)           │
│   · CERO lógica de negocio     channel   · indexa en SQLite           │
│                                 events   · mueve/borra archivos       │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

La regla que lo ordena todo: **el WebView no calcula nada caro**. No decodifica audio, no lee
directorios, no calcula picos de onda, no ordena 100.000 registros. Pide datos ya masticados y
los pinta. Todo lo pesado vive en Rust, donde hay hilos de verdad y SIMD.

Por qué Tauri y no Electron ni una UI nativa: [ADR-0001](adr/0001-tauri-como-shell.md).
Por qué el audio va en Rust y no en Web Audio: [ADR-0002](adr/0002-audio-en-rust.md).

## 2. Módulos del núcleo Rust

```
src-tauri/src/
├── lib.rs              builder de Tauri, estado global, invoke_handler
├── error.rs            AppError (thiserror) + serialización a { kind, message }
├── domain/             tipos puros: Sample, Destination, Peaks, TriageAction…
├── ipc/                un archivo por área: library, player, triage, settings
├── audio/
│   ├── engine.rs       hilo de control + stream cpal (se abre una vez)
│   ├── graph.rs        mezcla, fade, ganancia, loop  ← código de tiempo real
│   ├── decode.rs       symphonia → AudioBuffer (f32 intercalado)
│   ├── cache.rs        LRU por bytes de buffers decodificados
│   └── resample.rs     rubato, solo si el device y el archivo difieren
├── scan/
│   ├── walker.rs       recorrido paralelo del árbol (jwalk)
│   ├── analyze.rs      duración, canales, picos, loudness, hash de contenido
│   └── watcher.rs      (fase tardía) notify: detectar cambios en disco
├── db/
│   ├── migrations.rs   versionadas, idempotentes
│   ├── queries.rs      SQL preparado y cacheado
│   └── models.rs       filas ↔ tipos de dominio
└── fileops/
    ├── mover.rs        mover/copiar con verificación
    ├── trash.rs        papelera gestionada + manifiesto
    └── journal.rs      registro de acciones → undo/redo
```

Dependencias permitidas (ver también CLAUDE.md):

```
ipc → domain, db, audio, scan, fileops
audio → domain          (no conoce db ni tauri)
scan → domain, db       fileops → domain, db       db → domain       domain → nada
```

## 3. Los tres caminos calientes

### 3.1 Escaneo e indexado

```
walker (jwalk, N hilos)      ── ruta, tamaño, mtime ──►  canal
                                                          │
indexador (1 hilo, tx SQLite en lotes de 1000)  ◄─────────┘
                                                          │
                                     samples nuevos/cambiados
                                                          ▼
analizador (rayon)  ── decodifica cabecera + calcula picos + hash ──► UPDATE
```

- El listado aparece en la UI **antes** de que termine el análisis: primero se ven los nombres,
  luego van llegando duraciones y waveforms. Nunca hay una pantalla de carga bloqueante.
- Detección de cambios por `(tamaño, mtime)`. Si coinciden con lo indexado, ni se abre el archivo.
- El progreso viaja por un `tauri::ipc::Channel` con *throttle* de ~10 mensajes/s. Jamás un
  mensaje por archivo: eso ahogaría el WebView.

### 3.2 Reproducción instantánea

Este es el corazón del producto. La secuencia al pulsar `↓`:

```
tecla ↓
  │
  ├─► UI: mueve la selección y repinta la fila            (< 8 ms)
  │
  └─► invoke("player_play", { sampleId })
        │
        ├─ ¿el buffer está en la caché LRU?  ── sí ──► se envía al graph  ── suena (< 25 ms)
        │                                     no  ──► decodifica ahora (samples cortos:
        │                                              2-10 ms) y suena
        │
        └─► prefetch: decodifica en background los ±3 vecinos de la nueva selección
```

Claves de que se sienta instantáneo:

1. **El stream de salida nunca se cierra.** Abrirlo cuesta 50-200 ms; se abre al arrancar la app.
2. **Prefetch de vecinos.** Al navegar en secuencia, el siguiente sample ya está decodificado.
3. **Caché LRU en bytes** (256 MB por defecto). Un sample de 2 s estéreo a 44,1 kHz ocupa
   ~700 KB en f32: caben ~350 samples. En triaje se recorre en orden, así que el *hit rate* es alto.
4. **Fade de 5 ms** al cambiar de sample: sin clic, y sin esperar a que termine el anterior.
5. El cabezal de reproducción **no se transmite por IPC**. El front sabe `startedAt` y `duration`
   y lo interpola con `requestAnimationFrame`. Un evento IPC por frame mataría el rendimiento.

### 3.3 Triaje (mover/rechazar) con undo

```
tecla 3
  │
  └─► invoke("triage_send", { sampleId, destinationId })
        │
        1. INSERT en `actions` (estado: pending)  ← se registra ANTES de tocar el disco
        2. rename() si es el mismo dispositivo; si no, copy + verificar + remove
        3. UPDATE actions SET done_at = now, UPDATE samples SET status
        4. la UI avanza a la siguiente fila (optimista: ya avanzó en el paso 0)
```

- Si el proceso muere entre 1 y 3, al arrancar se detecta la acción `pending` y se repara.
- `Ctrl+Z` lee la última acción cerrada y la invierte; el journal permite deshacer también
  operaciones de sesiones anteriores.
- Nada se borra de verdad. Rechazar = mover a `<destino>/.samplecurator-trash/` con un manifiesto
  JSONL que guarda la ruta original. Vaciar la papelera es una acción explícita del usuario.

## 4. Contrato IPC

- Los comandos y sus tipos se definen en Rust y **se generan** hacia TypeScript con
  `tauri-specta` → `src/bindings.ts`. Ese archivo se versiona: cualquier cambio de contrato
  aparece en el diff del PR.
- `src/lib/ipc.ts` es la única puerta de entrada desde el front: envuelve los bindings, normaliza
  errores y añade tipos de conveniencia. Ningún componente llama a `invoke()`.
- Tres mecanismos, tres usos:
  - `invoke` → petición/respuesta (cargar página de la lista, reproducir, mover).
  - `Channel<T>` → flujos de progreso de vida corta (escaneo, análisis).
  - eventos globales → cambios de estado que interesan a varias vistas (biblioteca actualizada).
- Los picos de waveform se devuelven como **bytes crudos** (`tauri::ipc::Response`), 2 bytes por
  bucket (min/max en i8). 1.000 buckets = 2 KB. En JSON serían ~12 KB y un parseo por sample.

## 5. Estado en el frontend

| Qué | Dónde | Por qué |
|---|---|---|
| Selección, modo, filtros, panel activo | Zustand | síncrono, sin proveedor, selectores atómicos |
| Páginas de la lista, metadatos | caché propia sobre `ipc.ts` | los datos son inmutables una vez cargados |
| Estado del transporte (reproduciendo, `startedAt`) | Zustand | lo leen la waveform y el transporte |
| Posición del cabezal | ref + rAF, **fuera de React** | 60 fps sin re-render |

La lista usa `@tanstack/react-virtual` con filas de **altura fija**: permite saltar a cualquier
posición sin medir. Cada fila es `memo` y recibe solo props primitivas.

## 6. Arranque

```
main() → lib.rs
  ├─ abre/migra SQLite en app_data_dir()
  ├─ repara acciones `pending` del journal
  ├─ arranca el motor de audio (stream cpal abierto y en silencio)
  ├─ crea la ventana (la UI ya puede pintar: los datos llegan por IPC)
  └─ lanza el re-escaneo incremental de las carpetas conocidas en background
```

Presupuesto: ventana visible en < 400 ms, lista utilizable en < 1,2 s.

## 7. Qué NO hacemos (y por qué)

| Tentación | Decisión |
|---|---|
| Web Audio API para reproducir | Latencia y jitter del WebView; sin control del buffer. Ver ADR-0002 |
| Guardar los samples dentro de una base de datos | Los archivos son del usuario y deben seguir siendo suyos, navegables desde cualquier DAW |
| ORM en Rust (diesel/sea-orm) | SQL a mano con `rusqlite`: 15 consultas, todas calientes, todas medibles |
| Redux / máquina de estados global grande | El estado real vive en Rust; el front solo tiene estado de UI |
| Animaciones de layout | Cualquier cosa que reflowee la lista rompe el presupuesto de 16 ms |
| Auto-clasificación por IA en el MVP | Fase 5+. Primero el bucle manual tiene que ser perfecto |
