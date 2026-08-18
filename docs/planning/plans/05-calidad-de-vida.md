# Plan 05 — Calidad de vida

> Fase: 5 de 7 | Estado: 🔄 Parcial | 2026-08-18
> Hito: duplicados, filtros, renombrado, atajos configurables y detección de BPM/tonalidad

---

## Dependencia con otras fases

- **Requiere:** Fase 4 (el bucle básico tiene que estar perfecto antes de adornarlo).
- **Habilita:** nada crítico. Todo aquí es mejora medible del tiempo por decisión.

---

## Tareas

### Encontrar y filtrar
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 5.1 | Detección de duplicados por `content_hash` con chip `dup` en la fila | ✅ Hecho | Chip `dup` en la fila, calculado con un EXISTS sobre el índice de hash |
| 5.2 | Filtro `⇧D` (solo duplicados) y acción "conservar el mejor" (mayor tasa/profundidad) | 🔄 Parcial | El filtro ⇧D está; «conservar el mejor» automático, no |
| 5.3 | Filtros por duración, sample rate, canales, estado y valoración | 🔄 Parcial | Duración y valoración hechos; destino y etiqueta pasan a la [Fase 9](09-metadatos-valoracion-papelera.md) |
| 5.4 | Filtros combinables con chips visibles y `Esc` para limpiar | 🔄 Parcial | Los filtros se combinan y se ven activos; falta el chip que los resuma |
| 5.5 | Orden por nombre, duración, fecha, loudness y aleatorio | ✅ Hecho | Ruta, nombre, duración, tamaño, volumen y recientes (aleatorio no) |
| 5.6 | Paleta de comandos `Ctrl+K` (todas las acciones, con sus teclas) | 🔒 | 1.16 |

### Análisis musical → **movido a la [Fase 8](08-clasificacion-automatica.md)**

Cuatro filas de tabla no daban para lo que esto es. La Fase 8 lo desarrolla con su propio gate,
sus objetivos de acierto numéricos y su conjunto de evaluación.

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 5.7 | Detección de BPM por autocorrelación de la envolvente de energía | ➡️ Movida | 2.5 |
| 5.8 | Detección de tonalidad por chromagram + perfiles Krumhansl | ➡️ Movida | 2.4 |
| 5.9 | Columnas de BPM y tono, con filtro y orden | ➡️ Movida | 5.7, 5.8 |
| 5.10 | Clasificación gruesa por envolvente (one-shot vs loop vs cola larga) | ➡️ Movida | 2.5 |

### Manipulación
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 5.11 | Renombrar en línea (`F2`) con actualización del índice y del journal | ✅ Hecho | F2 renombra EN la barra de transporte, y Ctrl+Z también deshace el renombrado |
| 5.12 | Renombrado por patrón para un lote (`{destino}_{n:03}`) | 🔒 | 5.11 |
| 5.13 | Etiquetas libres con autocompletado, indexadas en FTS | ➡️ Movida | Se hace en la [Fase 9](09-metadatos-valoracion-papelera.md) |
| 5.14 | Comparación A/B entre dos samples con `Ctrl+B` | 🔒 | 3.11 |
| 5.15 | `Ctrl+R`: revelar en el explorador de archivos del sistema | ✅ Hecho | Ctrl+R vía tauri-plugin-opener |

### Gestión de carpetas y ajustes (petición del usuario, 2026-08-18)

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 5.20 | Añadir carpeta de origen desde la barra lateral, sin pasar por el asistente | ✅ Hecho | Botón + en la cabecera de «Carpetas» y en los ajustes; abre el diálogo nativo |
| 5.21 | Reescanear una carpeta ya añadida (detecta lo nuevo y poda lo que ya no está) | ✅ Hecho | ↻ en cada carpeta. El comando existía desde la Fase 2 y estaba huérfano: nada lo llamaba |
| 5.22 | Quitar una carpeta del índice, avisando de que los archivos no se tocan | ✅ Hecho | × con confirmación EN LA PROPIA FILA, sin modal. Avisa de que no se borra nada del disco |
| 5.23 | Panel de ajustes (`Ctrl+,`): carpetas, apariencia, escucha, papelera e información | ✅ Hecho | `Ctrl+,`: carpetas, apariencia, escucha, papelera e información (incluida la latencia real medida por el motor) |
| 5.24 | Densidad de fila configurable (compacta / normal / cómoda) | ✅ Hecho | Compacta 24 px · normal 28 · cómoda 34. Vive en el estado, no en el CSS: el virtualizador necesita el número exacto |

### Personalización
| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 5.16 | Atajos reconfigurables sobre el mapa declarativo, con detección de conflictos | 🔒 | 1.16 |
| 5.17 | Preset "una sola mano" (`Q W E A S D Z X C`) | 🔒 | 5.16 |
| 5.18 | Ajustes: densidad de fila, tamaño de caché, autoplay, normalización | ➡️ Movida | Se concreta en 5.23 y 5.24 |
| 5.19 | Exportar/importar `library.json` con las decisiones del usuario | ✅ Hecho | Ctrl+E y volcado automático al cerrar; escritura atómica. Importar, pendiente |

---

## Entregable

La misma app, pero triando el doble de rápido: los duplicados desaparecen de un filtro, los loops
se ordenan por BPM y cada mano tiene sus teclas.

## Criterio de aceptación

- BPM correcto (±1) en al menos el 85 % de una muestra de loops etiquetados a mano.
- La detección de duplicados no produce falsos positivos en un conjunto de control.
- Ningún filtro tarda más de 50 ms sobre 100.000 samples.

---

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | 5.1, 5.5, 5.11, 5.15, 5.19 | Duplicados, ordenaciones, renombrado, revelar y copia de decisiones |
| 2026-08-18 | 5.2–5.4 | Filtros de duración y valoración; el resto queda apuntado |
| 2026-08-18 | 5.20–5.24 | Gestión de carpetas y panel de ajustes, a petición del usuario tras usar la app de verdad. `library_rescan` y `library_remove_source` llevaban desde la Fase 2 implementados y sin conectar a nada |


---

## Lo que queda, y por qué se queda

| Tarea | Por qué no está |
|---|---|
| 5.6 Paleta de comandos `Ctrl+K` | La pantalla de ayuda (`?`) ya resuelve el descubrimiento, que era el 90 % de su valor |
| 5.7–5.10 BPM, tonalidad y clasificación | **Movidas a la [Fase 8](08-clasificacion-automatica.md)**, que ya está planificada con su gate y sus objetivos medibles |
| 5.12 Renombrado por patrón | Depende de 5.11, que ya está; es media tarde de trabajo cuando haga falta |
| 5.13 Etiquetas | El esquema (`tags`, `sample_tags`) y el FTS ya están listos; falta la interfaz |
| 5.14 Comparar A/B | El motor ya sabe cambiar de sample sin clics: es sobre todo interfaz |
| 5.16–5.17 Atajos reconfigurables | Todo el mapa está en un único archivo declarativo (`src/app/atajos.ts`); hacerlo configurable solo toca ese archivo |
| 5.18 Panel de ajustes | Hoy los ajustes que existen (tema, autoplay, bucle, modo) tienen tecla propia |

Ninguna de estas bloquea el uso real de la app: la Fase 4 ya cerraba el bucle completo.
