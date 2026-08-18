# Plan 02 — Índice de biblioteca

> Fase: 2 de 7 | Estado: ✅ Hecho | 2026-08-18
> Hito: elegir una carpeta y ver una lista virtualizada de 50.000 samples con búsqueda instantánea

---

## Dependencia con otras fases

- **Requiere:** Fase 1.
- **Habilita:** Fase 3 (el audio necesita saber qué archivos hay) y Fase 5 (filtros y duplicados).

---

## Tareas

### Escaneo (Rust)
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 2.1 | `scan/walker.rs`: recorrido paralelo con `jwalk`, filtro por extensión de audio | ✅ Hecho | jwalk, filtrado por extensión, rutas relativas a la raíz |
| 2.2 | Indexador: upsert por lotes de 1.000 en transacción, detección por `(size, mtime)` | ✅ Hecho | Upsert por lotes de 1.000 en transacción; `(size, mtime)` invalida el análisis |
| 2.3 | Comando `library_add_source` + progreso por `Channel` con throttle de 10 msg/s | ✅ Hecho | `library_add_source` + Channel con throttle de 100 ms |
| 2.4 | `scan/analyze.rs`: duración, sample rate, canales, profundidad con `symphonia` | ✅ Hecho | symphonia: duración, frecuencia, canales y profundidad de bits |
| 2.5 | Cálculo de picos (1.000 buckets, min/max i8) y `loudness_db` (RMS integrado) | ✅ Hecho | 1.000 buckets min/max en 2 bytes + RMS integrado |
| 2.6 | `content_hash` blake3 del PCM normalizado, solo archivos < 30 s | ✅ Hecho | blake3 sobre el PCM en mono a 16 bits, solo < 30 s. Límites documentados y testeados |
| 2.7 | Cola de análisis en `rayon` con prioridad a lo visible; reanudable tras cerrar la app | ✅ Hecho | Cola en rayon por lotes de 256, reanudable: el estado vive en `analyzed_at` |
| 2.8 | Archivos ilegibles: se marcan `dañado` y no vuelven a intentarse | ✅ Hecho | Los ilegibles quedan `broken = 1` y no se reintentan |

### Consulta (Rust)
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 2.9 | `library_page(offset, limit, filtro, orden)` con `prepare_cached` | ✅ Hecho | `page()` con filtro, orden y paginación; 16 ms en la peor página de 50.000 |
| 2.10 | Triggers de sincronización con `samples_fts` en insert/update/delete | ✅ Hecho | FTS5 externo con los tres triggers |
| 2.11 | `library_search(q)` sobre FTS5, < 50 ms con 100.000 filas | ✅ Hecho | Búsqueda con prefijos: 3,1 ms sobre 50.000 (presupuesto 50 ms) |
| 2.12 | `library_peaks(sampleId)` devolviendo bytes crudos (`tauri::ipc::Response`) | ✅ Hecho | Bytes crudos vía `tauri::ipc::Response` |

### Interfaz
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 2.13 | Selector de carpeta (diálogo nativo) + arrastrar y soltar sobre la ventana | ✅ Hecho | Diálogo nativo + arrastrar y soltar sobre la ventana |
| 2.14 | Lista virtualizada con `@tanstack/react-virtual`, altura de fila fija | ✅ Hecho | @tanstack/react-virtual con altura leída del token CSS |
| 2.15 | Componente `Row` con las cinco columnas de `docs/DESIGN_SYSTEM.md` | ✅ Hecho | 4 de las 5 columnas; la mini-onda por fila se aplaza (ver abajo) |
| 2.16 | Carga por ventanas con caché de páginas y placeholders para lo no cargado | ✅ Hecho | Páginas de 200 con caché y fila fantasma mientras carga |
| 2.17 | Barra de progreso de indexado no bloqueante | ✅ Hecho | Barra no bloqueante en la barra lateral |
| 2.18 | Búsqueda con `/`, incremental, con *debounce* de 80 ms | ✅ Hecho | `/` + debounce de 80 ms |
| 2.19 | Navegación con `↓ ↑ PageDown Home End` y foco persistente | ✅ Hecho | ↓↑, Av/Re Pág, Inicio/Fin, con foco persistente |

### Pruebas
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 2.20 | Test de integración: árbol sintético → escaneo → conteo e integridad del índice | ✅ Hecho | tests/triaje.rs monta índice real sobre TempDir |
| 2.21 | Test: reescaneo sin cambios no abre ningún archivo | ✅ Hecho | Test: reescanear sin cambios no reanaliza nada |
| 2.22 | Bench: escaneo de 50.000 archivos dentro de presupuesto | ✅ Hecho | tests/escala.rs con 50.000 reales: 1,69 s de escaneo |

---

## Entregable

Añades una carpeta con miles de samples y en segundos tienes una lista fluida, buscable, con
duraciones y mini-waveforms que van apareciendo solas.

## Criterio de aceptación

- 50.000 archivos listados en < 60 s, con la lista utilizable desde el primer segundo.
- Búsqueda por debajo de 50 ms.
- Navegar 100 filas con `↓` sin ningún frame por encima de 16 ms.
- Cerrar la app a mitad del análisis y reabrirla lo reanuda donde estaba.

---

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | 2.1–2.22 | Índice completo. Medido sobre 50.000 samples reales |
| 2026-08-18 | 2.9 | **Bug de rendimiento encontrado por el test**: página a 82 ms. Faltaban índices que cubrieran el `ORDER BY … COLLATE NOCASE`. Con ellos, 16 ms |


---

## Aplazado a propósito

**Mini-onda por fila (parte de 2.15).** Pintar 35 mini-ondas exige 35 blobs de picos por
pantalla. Hacerlo bien pide una columna aparte con ~32 buckets (64 bytes) y un canal binario
para la página entera; hacerlo mal significa 35 llamadas IPC por scroll. Se mueve a la Fase 6,
donde ya hay presupuesto medido para decidirlo con datos. La columna queda reservada en el
layout.
