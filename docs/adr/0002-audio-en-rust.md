# ADR-0002 — El audio se reproduce en Rust, no en el WebView

**Fecha:** 2026-08-18 · **Estado:** aceptada

## Contexto

La app tiene que reproducir un sample en cuanto se enfoca una fila, con latencia imperceptible y
sin clics, miles de veces por sesión. La alternativa evidente era usar la Web Audio API dentro del
WebView, que ya está ahí y es cómoda.

## Decisión

La reproducción vive en Rust: `cpal` para la salida y `symphonia` para decodificar, con una caché
LRU de buffers decodificados y prefetch de los vecinos de la selección.

## Por qué no Web Audio

1. **Latencia y jitter.** En WebKitGTK el tamaño de buffer no se controla y la latencia real ronda
   los 40-100 ms, con variabilidad. Nuestro presupuesto entero es de 25 ms.
2. **El archivo tendría que cruzar el puente.** Reproducir un WAV desde el WebView implica leerlo
   en Rust y pasarlo por IPC, o exponer un servidor local de archivos. Ambas cosas añaden copias y
   latencia justo en el camino crítico.
3. **`decodeAudioData` no es cacheable de forma controlada.** No podemos gestionar un LRU en bytes
   ni hacer prefetch predecible; el GC del WebView decide por nosotros.
4. **Formatos.** `symphonia` cubre WAV, AIFF, FLAC, MP3, OGG/Vorbis, AAC y ALAC de forma uniforme.
   El soporte de AIFF en WebKit es irregular, y AIFF abunda en las librerías de samples.
5. **Medibilidad.** En Rust podemos instrumentar el callback y publicar un histograma real de
   latencia. En el WebView solo podemos suponer.

## Consecuencias

- El código dentro de `src-tauri/src/audio/` es **código de tiempo real** y tiene sus propias
  reglas (sin allocs, sin locks, sin I/O, sin panics en el callback). Están en CLAUDE.md y las
  vigila la skill `samplecurator-audio-rt`.
- El cabezal de reproducción se calcula en el front por interpolación (`startedAt` + `duration`)
  para no cruzar el puente 60 veces por segundo.
- Hay que resamplear cuando el archivo y el device no coinciden (`rubato`). Se hace al decodificar,
  no en el callback.
- El motor se puede testear sin tarjeta de sonido: el grafo escribe a un buffer en memoria.
