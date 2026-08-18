# ADR-0003 — SQLite como índice reconstruible

**Fecha:** 2026-08-18 · **Estado:** aceptada

## Contexto

Hay que consultar, filtrar y buscar sobre decenas de miles de samples con respuesta inmediata, y
recordar duraciones, waveforms y decisiones del usuario entre sesiones.

## Decisión

SQLite embebido (`rusqlite`, feature `bundled`) en `app_data_dir()/library.db`, en modo WAL, con
FTS5 para la búsqueda. **El índice es una caché: la verdad son los archivos en disco.**

## Alternativas descartadas

- **JSON / sled / redb.** Un fichero JSON de 50.000 entradas se lee entero en cada arranque y no
  permite filtrar sin cargarlo todo. Las KV embebidas no dan consultas ni búsqueda de texto.
- **Guardar los samples dentro de la base de datos.** Rompe el principio del producto: los
  archivos son del usuario y tienen que seguir siendo navegables desde su DAW y su explorador.
- **Un ORM (diesel, sea-orm).** Son ~15 consultas, todas en el camino caliente. SQL a mano es más
  corto de leer, más fácil de perfilar y no arrastra macros ni tiempos de compilación.

## Consecuencias

- Se puede borrar el `.db` sin perder audio: se reindexa. Eso simplifica las migraciones (solo
  `up`) y elimina toda una clase de miedos.
- Lo que **sí** duele perder son las decisiones (destinos, historial, valoraciones), así que esas
  tablas se exportan a un `library.json` junto a la carpeta destino en cada cierre limpio.
- FTS5 obliga a mantener triggers de sincronización con `samples`. Es la única complejidad
  añadida, y compra búsqueda incremental por debajo de 50 ms.
