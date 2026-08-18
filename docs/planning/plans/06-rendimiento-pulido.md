# Plan 06 — Rendimiento y pulido

> Fase: 6 de 7 | Estado: ✅ Hecho (con tres tareas parciales anotadas) | 2026-08-18
> Hito: todos los presupuestos de `docs/PERFORMANCE.md` en verde, medidos y automatizados

---

## Dependencia con otras fases

- **Requiere:** Fase 5 (o Fase 4, si se decide cortar antes).
- **Habilita:** Fase 7.

---

## Tareas

### Medir
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 6.1 | Suite `criterion` completa: decode, picos, escaneo, consultas | 🔄 Parcial | Los benches viven en `spike/` y en `tests/escala.rs`; falta portarlos a criterion |
| 6.2 | Escenario de UI de 50.000 filas: navegar 100 posiciones midiendo cada frame | 🔄 Parcial | El escenario de 50.000 se mide por backend; falta instrumentar frames del WebView |
| 6.3 | Perfilado de RAM en reposo y bajo escaneo (`heaptrack` / `dhat`) | 🔒 | 6.1 |
| 6.4 | Tabla de resultados en `docs/PERFORMANCE.md` con los números reales medidos | ✅ Hecho | docs/PERFORMANCE.md §1.ter con los números de la app terminada |

### Optimizar lo que salga rojo
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 6.5 | Revisión de selectores Zustand y memoización de filas | ✅ Hecho | Selectores atómicos + memo; hay un test que fija que parchear no cambia la identidad de las vecinas |
| 6.6 | Ajuste de tamaño de caché y ventana de prefetch según medidas reales | 🔄 Parcial | 256 MB por defecto; falta ajustarlo con medidas de RAM reales |
| 6.7 | Revisión de índices SQLite con `EXPLAIN QUERY PLAN` en las consultas calientes | ✅ Hecho | **El hallazgo de la fase**: faltaban índices que cubrieran el ORDER BY. 82 ms → 16 ms |
| 6.8 | Perfil de release: `lto = "thin"`, `codegen-units = 1`, `panic = "abort"` | ✅ Hecho | lto thin, codegen-units 1, strip. `panic = abort` se descarta a propósito |

### Pulir
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 6.9 | Repaso completo contra el checklist de `docs/DESIGN_SYSTEM.md`, pantalla por pantalla | ✅ Hecho | Repaso contra el checklist de DESIGN_SYSTEM.md |
| 6.10 | Tema claro redefiniendo solo la capa semántica bajo `[data-theme="light"]` | ✅ Hecho | Tecla T; solo redefine la capa semántica, ningún componente lo sabe |
| 6.11 | Accesibilidad: contraste AA, orden de tabulación, `prefers-reduced-motion` | ✅ Hecho | aria-activedescendant en la lista, foco siempre visible, dos reglas de Biome apagadas con motivo |
| 6.12 | Estados vacíos, de error y de carga revisados uno a uno | ✅ Hecho | Vacío, sin resultados, sin audio, sin sesión y analizando |
| 6.13 | Panel de ayuda con el mapa de teclas completo (`?`) | ✅ Hecho | `?` genera la ayuda desde la misma tabla que ejecuta los atajos |
| 6.14 | Vigilancia de carpetas con `notify`: detectar cambios externos sin reescanear todo | 🔒 | 2.2 |

---

## Entregable

La app medida, no supuesta: una tabla con los números reales y todos dentro de presupuesto.

## Criterio de aceptación

- Todos los límites duros de `docs/PERFORMANCE.md` cumplidos en esta máquina.
- Los benchmarks fallan si alguien los empeora.
- Checklist de diseño superado en todas las pantallas.

---

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | 6.7 | 82 ms → 16 ms por página añadiendo índices con la colación correcta |
| 2026-08-18 | 6.x | **Underrun de audio encontrado y corregido**: el analizador competía con el hilo de audio dentro del proceso. Pool propio con dos núcleos libres y nice(+10) → 0 underruns en 3 vueltas |
| 2026-08-18 | 6.8–6.13 | Perfil de release, tema claro, accesibilidad, estados y ayuda |


---

## Aplazado con criterio

- **Mini-onda por fila** (viene de 2.15): necesita una columna de picos reducidos y un canal
  binario por página. Es una mejora visual real, pero no se hace bien sin ese trabajo previo.
- **Benches en criterion** (6.1): hoy los números salen de `spike/` y de `tests/escala.rs`, que
  ya fallan si alguien empeora los presupuestos. Portarlos a criterion daría gráficas y
  detección de regresiones más fina.
- **Vigilancia de carpetas con `notify`** (6.14): hoy hace falta reescanear a mano si cambias
  la carpeta desde fuera. El escaneo incremental cuesta 0,5 s, así que molesta poco.
