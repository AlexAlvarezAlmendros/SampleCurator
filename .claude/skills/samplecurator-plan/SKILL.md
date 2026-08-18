---
name: samplecurator-plan
description: "Use this skill at the START of every development request in the SampleCurator project (/home/poio/Documentos/GIT/SampleCurator). Triggers when the user asks to build, implement, create, fix, refactor, or develop any feature, command, module, component, migration, spike or test in the SampleCurator codebase (app de escritorio Tauri+Rust+React para triar librerías de samples de audio con una tecla por decisión). Manages the project roadmap and phased plans stored in docs/planning/: crea el plan cuando no existe y hace seguimiento del estado de las tareas."
metadata:
  version: 1.0.0
---

# SampleCurator — Workflow de planificación y seguimiento

Eres la capa de planificación de **SampleCurator**. Toda petición de desarrollo DEBE pasar por
este workflow antes de escribir código.

Contexto: app de escritorio (Tauri 2 + Rust + React 19 + SQLite) para recorrer una carpeta de
samples desordenados, escucharlos al instante y clasificarlos con **una tecla por decisión**.
Los dos objetivos que mandan sobre todo: **latencia percibida cero** y **cero pérdida de datos**.
Convenciones en `CLAUDE.md`; arquitectura en `docs/ARCHITECTURE.md`.

---

## Orden de ejecución obligatorio

1. **Lee el roadmap** — `docs/planning/ROADMAP.md`
   - Si no existe → créalo con la plantilla de abajo.
   - Si existe → identifica la fase actual y el trabajo pendiente.

2. **Identifica el plan relevante** — `docs/planning/plans/NN-nombre.md`
   - Mapea la petición del usuario a la fase correcta.
   - Si el plan no existe → créalo con la plantilla.
   - Si existe → léelo para saber qué está hecho, en curso y bloqueado.

3. **Comprueba el gate de la Fase 0.** El spike de latencia es un GO/NO-GO de todo el proyecto:
   no empieces la Fase 1 ni posteriores si no hay GO registrado en `00-spike-latencia.md`
   (o el usuario lo decide explícitamente saltándoselo).

4. **Actualiza el estado antes de escribir código:**
   - Marca la tarea como `🔄 En curso`.
   - Confirma al usuario: "Empiezo la tarea X del plan Y. Dependencias satisfechas: […]. Bloquea: […]".

5. **Ejecuta el trabajo** siguiendo TODAS las convenciones de `CLAUDE.md`. Aplica además:
   - `samplecurator-audio-rt` si tocas `src-tauri/src/audio/`
   - `samplecurator-comando-ipc` si añades o cambias un comando Tauri
   - `samplecurator-componente` si creas un componente React

6. **Actualiza el plan al terminar:**
   - Tareas completadas → `✅ Hecho` con nota breve.
   - Tareas recién desbloqueadas → `⬜ Listo`.
   - Fila nueva en el registro de avance con la fecha.
   - Reporta: "Completado: […]. Siguientes tareas listas: […]".

---

## Estructura de carpetas

```
docs/
  planning/
    ROADMAP.md                       ← roadmap global, una fila por fase
    plans/
      00-spike-latencia.md           ← Fase 0: GATE GO/NO-GO (latencia y escaneo)
      01-fundaciones.md              ← Fase 1: scaffold, tokens, bindings IPC
      02-indice-biblioteca.md        ← Fase 2: escaneo, SQLite, lista virtualizada
      03-motor-audio.md              ← Fase 3: cpal, caché, prefetch, waveform
      04-triaje.md                   ← Fase 4: MVP — destinos, undo, papelera
      05-calidad-de-vida.md          ← Fase 5: duplicados, filtros, BPM/tono
      06-rendimiento-pulido.md       ← Fase 6: presupuestos verdes, tema claro
      07-empaquetado.md              ← Fase 7: AppImage, .deb, actualizador
      08-clasificacion-automatica.md ← Fase 8: tipo, BPM y tonalidad (GATE en 8.0)
```

---

## Plantilla de ROADMAP.md

Créala solo si no existe (el repo ya trae una; respeta su formato):

```markdown
# SampleCurator — Roadmap del proyecto

> Última actualización: YYYY-MM-DD

## Fases

| # | Fase | Estado | Plan | Hito |
|---|------|--------|------|------|
| 0 | Spike de latencia | ⬜ Pendiente | [00-spike-latencia.md](plans/00-spike-latencia.md) | Tecla → sonido < 25 ms (GATE) |

## Foco actual

**Fase N — …**

## Grafo de dependencias

(qué fase habilita cuál)

## Leyenda de estados

| Icono | Significado |
|-------|-------------|
| ⬜ Listo / Pendiente | Sin bloqueos |
| 🔄 En curso | Trabajando ahora |
| ✅ Hecho | Completado |
| 🔒 Bloqueado | Espera a otra tarea |
| ❌ Cancelado | Fuera de alcance |
```

## Plantilla de plan de fase

```markdown
# Plan NN — [Nombre de la fase]

> Fase: N de 7 | Estado: 🔄 En curso | Iniciado: YYYY-MM-DD
> Hito del roadmap: [descripción]

## Dependencia con otras fases

- **Requiere:** [planes previos, o "nada"]
- **Habilita:** [qué desbloquea]

## Tareas

### [Área — p. ej. Núcleo Rust]

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| N.1 | [descripción] | ⬜ Listo | [dependencia] |

## Entregable

[qué se puede hacer al terminar]

## Criterio de aceptación

[cómo se sabe que está hecha — con números si hay presupuesto asociado]

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| YYYY-MM-DD | [tarea] | [resultado] |
```

---

## Reglas de seguimiento de dependencias

- Una tarea está `⬜ Listo` solo si todas sus dependencias están `✅ Hecho`.
- Una tarea está `🔒 Bloqueado` si alguna dependencia no lo está.
- Nunca marques `🔄 En curso` una tarea con dependencias abiertas.
- Al cerrar una tarea, actualiza en el acto las que desbloqueaba.
- **Cross-fase:** la Fase N no empieza hasta cumplir el hito de la N-1.
- **Fase 4 es el corte útil:** al terminarla la app ya resuelve el problema. Las fases 5-8 son
  mejora. Si el usuario quiere parar ahí, es una decisión legítima, no una fase incompleta.
- **La Fase 8 tiene su propio gate (8.0):** no se escribe ni un estimador de BPM o tonalidad
  hasta que exista el conjunto de evaluación y se haya medido cuánto miente la referencia débil.

## Reglas propias de este proyecto

- **Todo lo que toque archivos del usuario necesita test sobre `TempDir` antes de darse por hecho.**
  Una tarea de `fileops` sin test de undo no se marca `✅ Hecho`.
- **Toda tarea con presupuesto de rendimiento se cierra con el número medido**, no con una
  impresión. El número va en el registro de avance.
- **Nada que estime algo (BPM, tonalidad, tipo) se marca `✅ Hecho` sin su cifra de acierto
  medida contra el conjunto de evaluación**, ni sin comprobar que se calla cuando la pregunta no
  aplica: un kick no tiene tonalidad y un hi-hat no tiene tempo.
- Antes de añadir una dependencia nueva (crate o paquete npm), justifícala en el plan de la fase.

---

## Cómo reportar el estado al usuario

```
📋 Plan: 04-triaje.md (Fase 4 — Triaje)
✅ Hecho (3): 4.1 journal, 4.2 mover, 4.3 colisiones
🔄 En curso (1): 4.4 papelera gestionada
⬜ Siguientes listas (2): 4.8 sesiones, 4.14 panel de destinos
🔒 Bloqueadas (14)
```
