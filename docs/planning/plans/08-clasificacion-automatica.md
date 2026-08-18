# Plan 08 — Clasificación automática: tipo, BPM y tonalidad

> Fase: 8 de 8 | Estado: ⬜ Listo | Planificada: 2026-08-18
> Hito: que la lista diga `[kick 96%]`, `[loop 174 BPM]` y `[F min]` **y que esos números sean
> ciertos**, medido contra material real etiquetado, no contra una impresión.

---

## Dependencia con otras fases

- **Requiere:** Fase 2 (el analizador en segundo plano donde esto se engancha) y Fase 4 (el
  bucle de triaje al que esto sirve). Ambas ✅.
- **Habilita:** nada. Es la última fase: si sale mal, la app sigue siendo exactamente lo que ya
  es hoy.

## Dos decisiones tomadas antes de escribir una línea

**1 · El clasificador etiqueta; no mueve nada.** Ni propone destino ni pre-ordena en lote. Añade
columnas, filtros y ordenaciones, y el humano sigue decidiendo cada sample con su tecla. Una
etiqueta equivocada cuesta una mirada; un archivo movido por equivocación cuesta confianza. Si
más adelante los números demuestran que acierta, proponer destino es media tarde de trabajo
encima de esto.

**2 · La verdad de referencia se construye por dos caminos a la vez.** Los nombres de archivo dan
una referencia amplia y gratis (`KICK_808_128bpm_Cmin.wav` se etiqueta solo); unas 200
correcciones a mano miden **cuánto mienten esos nombres** y cubren el material que no dice nada
en el nombre. Sin las dos, los números no valdrían: solo con nombres estaríamos midiendo si
sabemos leer nombres, y solo con 200 muestras no habría con qué medir a escala.

---

## El problema real, que no es "detectar BPM"

Una librería de samples no es una carpeta de canciones, y eso cambia el algoritmo entero:

| | Canción de 4 minutos | Sample de un pack |
|---|---|---|
| Duración | larga, sobra información | 0,2 s a 8 s, a veces un solo golpe |
| ¿Tiene BPM? | siempre | **casi nunca**: un kick no tiene tempo |
| ¿Tiene tonalidad? | casi siempre | **depende**: un hi-hat no tiene nota |
| Cortes | arbitrarios | **el loop suele estar cortado a compás exacto** |
| Nombre | ruido | **suele traer la respuesta escrita** |

De ahí las tres ideas que sostienen esta fase:

1. **Callarse es una respuesta correcta.** Un BPM inventado sobre un kick, o una tonalidad
   inventada sobre un hi-hat, es peor que una celda vacía: ensucia los filtros y hace que el
   usuario deje de fiarse de la columna entera. Antes de estimar nada hay que decidir *si la
   pregunta tiene sentido para este sample*.
2. **La duración de un loop bien cortado es casi la respuesta.** Si un archivo dura 3,692 s y
   asumimos 2 compases de 4/4, sale BPM = 240·2/3,692 = **130,0**. Los candidatos plausibles
   (1, 2, 4, 8 compases) filtrados a un rango musical dejan casi siempre uno solo, y con una
   precisión que ningún seguidor de pulso alcanza sobre 4 segundos de audio.
3. **Tres señales flojas que se confirman entre sí valen más que una fuerte.** Nombre de
   archivo, ajuste por duración y autocorrelación de la envolvente son independientes. Cuando
   coinciden, la confianza es altísima. Cuando no, hay que decirlo en vez de elegir a ciegas.

El error clásico del BPM —dar 87 donde son 174, o al revés— se resuelve precisamente ahí: la
autocorrelación se equivoca de octava con facilidad, pero el ajuste por duración no.

---

## Arquitectura

Un módulo nuevo, `src-tauri/src/music/`, entre `codec` y `scan`:

```
music/
  mod.rs         orquesta: UNA pasada por el audio → todas las características
  spectrum.rs    STFT y espectros de magnitud (lo comparten los tres análisis)
  features.rs    centroide, rolloff, planitud, ZCR, ataque, caída, bandas
  onset.rs       envolvente de energía y flujo espectral
  tempo.rs       BPM: tres señales independientes y su fusión
  chroma.rs      cromagrama
  key.rs         tonalidad (loops) y altura (one-shots tonales)
  kind.rs        tipo de sample a partir de las características
  filename.rs    lo que el nombre del archivo ya nos está diciendo
```

**Regla de dependencias, ampliada** (va a `CLAUDE.md`):

```
music → domain, codec        (no conoce db, ni tauri, ni el sistema de archivos)
scan  → domain, db, codec, music
```

**Una sola pasada.** El analizador ya decodifica cada archivo para los picos, la sonoridad y el
hash. La STFT se calcula **una vez** y de ella salen envolvente, cromagrama y características
espectrales. Decodificar dos veces el mismo archivo sería tirar el presupuesto por la ventana.

**Dependencia nueva a justificar:** `realfft` (sobre `rustfft`). La FFT es lo único aquí que no
merece la pena escribir a mano: es código muy optimizado, muy testeado y con SIMD. `realfft`
aprovecha que nuestra entrada es real y va ~2× más rápido que la FFT compleja genérica. El resto
—cromagrama, perfiles, autocorrelación, heurísticas— es código nuestro y testeable.

---

## Tareas

### 8.0 · GATE: conjunto de evaluación y línea base

Antes de escribir un solo estimador. Si no se puede construir una referencia en la que confiar,
no se puede construir un clasificador en el que confiar.

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 8.0.1 | `music/filename.rs`: extraer del nombre BPM, tonalidad y tipo (`KICK`, `128bpm`, `Cmin`, `Amaj`, `140`, `snr`, `oh`/`ch`…) con su confianza | ⬜ Listo | — |
| 8.0.2 | Migración 002: tabla `labels` (sample_id, campo, valor, origen `filename`/`user`, fecha) | ⬜ Listo | — |
| 8.0.3 | Poblar la referencia débil: pasar el extractor por la biblioteca entera y guardar lo que salga | ⬜ Listo | 8.0.1, 8.0.2 |
| 8.0.4 | Interfaz mínima de corrección: en la fila enfocada, corregir tipo/BPM/tono y que quede como etiqueta `user` | ⬜ Listo | 8.0.2 |
| 8.0.5 | Sesión de etiquetado: **~200 samples reales corregidos a mano** por el usuario, muestreados al azar y estratificados por tipo | ⬜ Listo | 8.0.4 |
| 8.0.6 | `tests/evaluacion.rs`: informe de precisión por campo contra ambas referencias, con matriz de confusión por clase | ⬜ Listo | 8.0.3, 8.0.5 |
| 8.0.7 | **Medir cuánto mienten los nombres**: comparar referencia débil contra las 200 correcciones | ⬜ Listo | 8.0.6 |
| 8.0.8 | **GO / NO-GO** y fijación de objetivos numéricos definitivos a la luz de 8.0.7 | ⬜ Listo | 8.0.7 |

> **El gate.** Si los nombres mienten en más del ~20 % de los casos, la referencia débil no sirve
> como juez y hay que ampliar el etiquetado manual (o reducir el alcance a lo que sí se pueda
> medir) **antes** de invertir en DSP. Ese número no lo sabemos hoy: por eso se mide primero.

### 8.1 · Base espectral

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 8.1.1 | `spectrum.rs`: STFT con ventana Hann de 2048 y salto de 512, sobre la mezcla a mono | 🔒 Bloqueado | 8.0.8 |
| 8.1.2 | `features.rs`: centroide, rolloff 85 %, planitud espectral, ZCR, tiempo de ataque, tiempo de caída a −20 dB, energía en 6 bandas | 🔒 Bloqueado | 8.1.1 |
| 8.1.3 | `onset.rs`: envolvente de energía y flujo espectral con media móvil | 🔒 Bloqueado | 8.1.1 |
| 8.1.4 | **Tonalidad sí/no**: decidir si un sample tiene altura definida (armonicidad + planitud) | 🔒 Bloqueado | 8.1.2 |
| 8.1.5 | Medir el coste de la pasada completa sobre 5.000 archivos reales y extrapolar a 50.000 | 🔒 Bloqueado | 8.1.2, 8.1.3 |

### 8.2 · Tipo de sample

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 8.2.1 | Taxonomía cerrada: `kick`, `snare`, `clap`, `hat`, `cymbal`, `perc`, `tom`, `bass`, `synth`, `vocal`, `fx`, `loop`, `desconocido` | 🔒 Bloqueado | 8.0.8 |
| 8.2.2 | `kind.rs`: reglas sobre las características (árbol de decisión escrito a mano y legible) | 🔒 Bloqueado | 8.1.2, 8.2.1 |
| 8.2.3 | Separar `loop` de one-shot por periodicidad y duración, no solo por duración | 🔒 Bloqueado | 8.1.3 |
| 8.2.4 | Confianza por clase y `desconocido` cuando ninguna regla domina | 🔒 Bloqueado | 8.2.2 |
| 8.2.5 | Evaluar contra el conjunto y publicar la matriz de confusión en el registro | 🔒 Bloqueado | 8.2.4, 8.0.6 |

### 8.3 · BPM

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 8.3.1 | Señal A — ajuste por duración: candidatos de 1/2/4/8 compases dentro de 60-200 BPM, con el error de encaje como confianza | 🔒 Bloqueado | 8.0.8 |
| 8.3.2 | Señal B — autocorrelación de la envolvente de onsets, con peso a favor de los tempos habituales | 🔒 Bloqueado | 8.1.3 |
| 8.3.3 | Señal C — el BPM escrito en el nombre | 🔒 Bloqueado | 8.0.1 |
| 8.3.4 | Fusión: acuerdo entre señales → confianza alta; desacuerdo de octava → resolver con A; sin acuerdo → devolver candidato y confianza baja | 🔒 Bloqueado | 8.3.1-8.3.3 |
| 8.3.5 | **BPM nulo en one-shots**: si no es `loop`, no hay tempo que dar | 🔒 Bloqueado | 8.2.3 |
| 8.3.6 | Evaluar: acierto ±1 BPM, y errores de octava contados aparte | 🔒 Bloqueado | 8.3.4, 8.0.6 |

### 8.4 · Tonalidad y altura

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 8.4.1 | `chroma.rs`: cromagrama de 12 clases con pesado logarítmico y recorte por debajo de 60 Hz y por encima de 5 kHz | 🔒 Bloqueado | 8.1.1 |
| 8.4.2 | `key.rs`: correlación contra 24 perfiles (Krumhansl-Schmuckler, con Temperley como alternativa a comparar) | 🔒 Bloqueado | 8.4.1 |
| 8.4.3 | Confianza = margen entre el mejor perfil y el segundo; relativo mayor/menor tratado como caso especial | 🔒 Bloqueado | 8.4.2 |
| 8.4.4 | **Altura en one-shots tonales**: para un 808 o un stab, la nota (`F#1`) importa más que la tonalidad | 🔒 Bloqueado | 8.1.4 |
| 8.4.5 | **Silencio en lo percusivo**: sin altura definida, no hay tono ni nota | 🔒 Bloqueado | 8.1.4 |
| 8.4.6 | Evaluar con puntuación tipo MIREX (exacto, quinta, relativo, paralelo) además del acierto exacto | 🔒 Bloqueado | 8.4.3, 8.0.6 |

### 8.5 · Integración

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 8.5.1 | Migración 003: `bpm`, `bpm_conf`, `bpm_source`, `key_root`, `key_mode`, `key_conf`, `pitch_hz`, `pitch_note`, `kind`, `kind_conf`, `tonal`, `analysis_version` | 🔒 Bloqueado | 8.2.4, 8.3.4, 8.4.3 |
| 8.5.2 | `analysis_version`: al subir la versión del analizador, solo se reanaliza lo caducado, en segundo plano y reanudable | 🔒 Bloqueado | 8.5.1 |
| 8.5.3 | Enganchar `music` al analizador, respetando la pasada única | 🔒 Bloqueado | 8.5.1 |
| 8.5.4 | Índices para filtrar y ordenar por bpm, tono y tipo sin salirse de los 16 ms por página | 🔒 Bloqueado | 8.5.1 |
| 8.5.5 | Columnas en la fila: tipo con confianza, BPM y tono. Lo dudoso se ve dudoso (atenuado, nunca como hecho) | 🔒 Bloqueado | 8.5.3 |
| 8.5.6 | Filtros por tipo, rango de BPM y tonalidad; ordenar por BPM y por tono | 🔒 Bloqueado | 8.5.4 |
| 8.5.7 | Corregir una etiqueta desde la fila enfocada, y que esa corrección alimente el conjunto de evaluación | 🔒 Bloqueado | 8.0.4, 8.5.5 |
| 8.5.8 | Búsqueda por etiqueta: que `kick 128` en el buscador encuentre lo esperado | 🔒 Bloqueado | 8.5.1 |

### 8.6 · Cierre

| # | Tarea | Estado | Notas |
|---|-------|--------|-------|
| 8.6.1 | Informe final de precisión con los tres campos y sus matrices | 🔒 Bloqueado | 8.2.5, 8.3.6, 8.4.6 |
| 8.6.2 | Medir el coste real del análisis completo sobre 50.000 y anotarlo en `docs/PERFORMANCE.md` | 🔒 Bloqueado | 8.5.3 |
| 8.6.3 | **Decisión documentada: ¿bastan las heurísticas o hace falta aprendizaje automático?** | 🔒 Bloqueado | 8.6.1 |
| 8.6.4 | ADR-0007 con el diseño final, los números y lo que se descartó | 🔒 Bloqueado | 8.6.1-8.6.3 |

---

## Presupuestos

| Métrica | Objetivo | Límite duro |
|---|---|---|
| Análisis completo de 50.000 (todo incluido, hoy 18 s) | 45 s | **60 s** |
| Coste añadido por archivo (14 hilos) | 0,5 ms | 0,84 ms |
| Página de la lista con las columnas nuevas | 16 ms | **50 ms** |
| Filtrar por tipo o rango de BPM sobre 50.000 | 20 ms | 50 ms |

## Objetivos de acierto

Provisionales: los definitivos se fijan en 8.0.8, cuando sepamos cuánto miente la referencia
débil. Se miden **sobre las 200 correcciones manuales**, que es la única verdad sin discusión.

| Campo | Objetivo | Nota |
|---|---|---|
| Tipo (12 clases) | ≥ 85 % global | Con recall por clase publicado: kick y hat deberían pasar del 95 %; `fx` es difuso por naturaleza |
| BPM en loops | ≥ 90 % dentro de ±1 BPM | Los errores de octava se cuentan y se publican aparte, no se esconden en el global |
| Tonalidad en loops tonales | ≥ 70 % exacto · ≥ 0,80 ponderado MIREX | La tonalidad es genuinamente difícil; 70 % exacto sobre material real es un resultado bueno |
| Altura en one-shots tonales | ≥ 90 % de la nota correcta | Más fácil que la tonalidad: hay un fundamental que encontrar |
| **Disciplina del silencio** | ≥ 95 % | Un kick NO puede recibir tonalidad, ni un hat un BPM. Este es el número que más protege la confianza del usuario |

Si un objetivo no se alcanza, la salida **no** es rebajarlo: es dejar ese campo vacío para el
material donde falla y decirlo en el ADR. Una columna que acierta el 70 % y lo admite es útil;
una que acierta el 70 % y aparenta certeza es una trampa.

---

## Entregable

La lista muestra `[kick 96%]`, `[loop 174 BPM]` y `[F min]`, se puede filtrar por tipo, por rango
de BPM y por tonalidad, y ordenar por tempo. Cuando el clasificador no está seguro, se le nota.
Cuando la pregunta no aplica, la celda está vacía.

## Criterio de aceptación

- Los objetivos de acierto de arriba, medidos contra las 200 correcciones y publicados con sus
  matrices de confusión en el registro de avance.
- El análisis completo de 50.000 sigue por debajo de 60 s y la lista por debajo de 16 ms.
- Ni un solo BPM sobre un one-shot ni una tonalidad sobre un hi-hat en el conjunto de evaluación.
- ADR-0007 escrito, incluida la decisión sobre aprendizaje automático.

---

## Lo que esta fase NO hace, y por qué

| Idea | Por qué queda fuera |
|---|---|
| Seguimiento de pulso completo con detección de tiempo fuerte | Sirve para canciones; en loops de 2 compases el ajuste por duración gana por goleada y cuesta mil veces menos |
| Clasificación por género | No ayuda a ordenar una librería: nadie busca "un sample de techno", busca "un kick" |
| Embeddings y búsqueda por parecido sonoro | Es otro producto entero. Si algún día se hace, se apoyaría en las características de 8.1 |
| Separación de fuentes | Fuera de alcance |
| Mover o pre-ordenar según la clasificación | Decisión explícita del usuario: el clasificador etiqueta, el humano decide |

## El punto donde esto se puede torcer

**Que las heurísticas se conviertan en un pozo sin fondo.** Es fácil pasarse semanas afinando
umbrales para arañar un 2 %. Por eso 8.6.3 es una tarea con nombre propio: cuando el informe de
8.6.1 esté sobre la mesa, se decide **con datos** si las reglas bastan o si un modelo pequeño
entrenado con las correcciones del usuario lo hace mejor. Y si se decide entrenar, el conjunto
de datos ya existe desde 8.0.5, que es justo el motivo de haberlo construido primero.

---

## Registro de avance

| Fecha | Tarea | Notas |
|-------|-------|-------|
| 2026-08-18 | — | Plan escrito. Alcance decidido: etiquetar y filtrar, sin mover nada. Validación por doble camino (nombres + 200 correcciones) |
