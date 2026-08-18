# Plan 04 — Triaje (MVP)

> Fase: 4 de 7 | Estado: ✅ Hecho | 2026-08-18 · **MVP funcional**
> Hito: **el producto**. `1…9` clasifica, `X` rechaza, `Ctrl+Z` deshace. Una tecla por decisión.

---

## Dependencia con otras fases

- **Requiere:** Fase 3.
- **Habilita:** el uso real. Al cerrar esta fase la app ya resuelve el problema completo.

---

## Tareas

### Operaciones de archivo (Rust) — la parte delicada
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 4.1 | `fileops/journal.rs`: escribir la acción **antes** de tocar el disco, cerrarla después | ✅ Hecho | Journal con `prev_status`, `prev_dest` y `prev_current`: deshacer no reconstruye nada |
| 4.2 | `fileops/mover.rs`: `rename` mismo dispositivo; entre dispositivos copiar → verificar → borrar | ✅ Hecho | rename mismo dispositivo; entre dispositivos copiar → verificar por hash → borrar |
| 4.3 | Resolución de colisiones con sufijo ` (2)`, ` (3)`… Jamás sobrescribir | ✅ Hecho | ` (2)`, ` (3)`… con reservas dentro del lote (tres `kick.wav` a la vez no se pisan) |
| 4.4 | `fileops/trash.rs`: papelera gestionada `<destino>/.samplecurator-trash/` + manifiesto JSONL | ✅ Hecho | Papelera gestionada + manifiesto JSONL con las comillas escapadas |
| 4.5 | Reparación al arrancar: acciones con `done_at IS NULL` se investigan y se resuelven | ✅ Hecho | Reparación al arrancar; ante la duda SIEMPRE se conserva el original |
| 4.6 | Undo/redo por journal, con `batch_id` para que un lote se deshaga de una vez | ✅ Hecho | Deshacer/rehacer por `batch_id` |
| 4.7 | Comandos `triage_send`, `triage_reject`, `triage_keep`, `triage_undo`, `triage_redo` | ✅ Hecho | triage_send / reject / keep / undo / redo |

### Sesiones y destinos
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 4.8 | CRUD de `projects` (origen, raíz de destino, modo mover/copiar) | ✅ Hecho | CRUD de proyectos con modo mover/copiar |
| 4.9 | CRUD de `destinations` con asignación automática de la siguiente tecla libre `1…9` | ✅ Hecho | Destinos con asignación automática de la primera tecla libre 1…9 |
| 4.10 | Importar las subcarpetas existentes del destino como destinos propuestos | ✅ Hecho | `triage_suggest_destinations` propone las subcarpetas que ya existen |
| 4.11 | Reanudar sesión: al abrir, volver al último sample revisado | ✅ Hecho | `last_sample_id` por proyecto; al abrir, la app vuelve donde estabas |
| 4.12 | Contadores por destino, actualizados de forma transaccional | ✅ Hecho | Contador transaccional + recuento desde la verdad al reparar |

### Interfaz
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 4.13 | Asistente de arranque de sesión (origen → destino → destinos → `Enter`), sin modales | ✅ Hecho | Asistente con los tres pasos a la vez, sin encadenar pantallas |
| 4.14 | Panel de destinos con `DestinationBucket` (número, nombre, contador, color de token) | ✅ Hecho | Cubos con número, nombre, contador y color de token |
| 4.15 | Teclas `1…9`, `X`, `Enter`, `F` con avance automático a la fila siguiente | ✅ Hecho | 1…9, X/Supr, Intro, F, con avance automático |
| 4.16 | Actualización optimista: la fila se marca en el frame de la pulsación; revierte si falla | ✅ Hecho | Optimista con reversión: la fila se marca en el frame de la tecla |
| 4.17 | Parpadeo de 80 ms del cubo receptor + incremento del contador | ✅ Hecho | Parpadeo de 80 ms del contador al recibir (se remonta por `key`) |
| 4.18 | `Ctrl+Z` / `Ctrl+⇧+Z` con retorno del foco a la fila afectada y reproducción | ✅ Hecho | Ctrl+Z / Ctrl+⇧+Z devolviendo también el foco al sample afectado |
| 4.19 | Selección múltiple (`⇧↓`, `Ctrl+A`) y envío por lotes con un solo undo | ✅ Hecho | ⇧↓/⇧↑ y Ctrl+A; el lote se deshace de una vez |
| 4.20 | `SessionProgress`: `428 / 3.211` + barra fina | ✅ Hecho | `428 / 3.211` en mono con barra de 2 px |
| 4.21 | Barra inferior de avisos no bloqueantes (fallo al mover, destino inaccesible) | ✅ Hecho | Avisos no bloqueantes que se van solos |
| 4.22 | Vaciar papelera desde ajustes — el único diálogo de confirmación de la app | ✅ Hecho | Único diálogo de confirmación de la app |

### Pruebas — aquí no se escatima
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 4.23 | Integración sobre `TempDir`: mover → verificar destino → undo → verificar origen | ✅ Hecho | tests/triaje.rs: mover → verificar → deshacer → verificar |
| 4.24 | Colisión de nombres: 3 archivos iguales a un mismo destino, ninguno se pierde | ✅ Hecho | Test de colisión triple: los tres archivos llegan y ninguno se sobrescribe |
| 4.25 | Corte simulado entre journal y disco → reparación correcta al arrancar | ✅ Hecho | Dos tests de corte simulado: uno completa y otro descarta, conservando el original |
| 4.26 | Copia entre dispositivos simulada: nunca se borra el origen sin verificar | ✅ Hecho | `mover_entre_dispositivos` extraído y testeado con verificación por hash |
| 4.27 | Undo de un lote de 40 con un solo `Ctrl+Z` | ✅ Hecho | Test de lote de 40 deshecho con una sola llamada |

---

## Entregable

Una sesión real: carpeta de 3.000 samples desordenados → 30 minutos → carpetas limpias por
categoría, la basura en la papelera, y ni un archivo perdido.

## Criterio de aceptación

- El bucle completo se hace **sin tocar el ratón**.
- Ninguna operación de archivo pierde ni sobrescribe datos, ni siquiera matando el proceso a mitad.
- `Ctrl+Z` restaura estado, archivo, contador y foco.
- Decidir 100 samples seguidos no produce ni una caída de frame.

---

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | 4.1–4.27 | Triaje completo. 9 tests de integración sobre archivos reales en TempDir |

