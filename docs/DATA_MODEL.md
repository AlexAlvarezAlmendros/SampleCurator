# Modelo de datos

Motor: **SQLite** (`rusqlite` con `bundled`), en `app_data_dir()/library.db`, modo WAL.

Principio rector: **el índice es una caché reconstruible**. La verdad son los archivos en disco.
Si el `.db` desaparece, se vuelve a escanear y no se pierde ni un sonido. Lo único que sí duele
perder son las decisiones del usuario (destinos, valoraciones, historial): por eso esas tablas se
exportan a un `library.json` junto a la carpeta destino en cada cierre limpio.

## Esquema

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous  = NORMAL;
PRAGMA foreign_keys = ON;

-- Carpetas de origen que el usuario ha añadido a la biblioteca
CREATE TABLE sources (
  id          INTEGER PRIMARY KEY,
  path        TEXT NOT NULL UNIQUE,
  added_at    INTEGER NOT NULL,          -- epoch ms
  last_scan   INTEGER
);

-- Un archivo de audio conocido
CREATE TABLE samples (
  id            INTEGER PRIMARY KEY,
  source_id     INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  rel_path      TEXT    NOT NULL,        -- relativa a sources.path
  filename      TEXT    NOT NULL,
  ext           TEXT    NOT NULL,        -- wav, flac, aiff, mp3, ogg…
  size          INTEGER NOT NULL,
  mtime         INTEGER NOT NULL,        -- epoch ms; con size decide si hay que reanalizar

  -- rellenado por el analizador en background (NULL = pendiente)
  duration_ms   INTEGER,
  sample_rate   INTEGER,
  channels      INTEGER,
  bit_depth     INTEGER,
  loudness_db   REAL,                    -- RMS integrado, para normalizar la escucha
  peaks         BLOB,                    -- 2 bytes/bucket (min,max i8), ~1000 buckets
  content_hash  BLOB,                    -- blake3 de las muestras PCM (detección de duplicados)
  analyzed_at   INTEGER,

  -- decisiones del usuario
  status        TEXT    NOT NULL DEFAULT 'pending',  -- pending|kept|rejected|moved
  rating        INTEGER NOT NULL DEFAULT 0,          -- 0-5
  seen_at       INTEGER,                             -- última vez que sonó en triaje

  UNIQUE (source_id, rel_path)
);

CREATE INDEX idx_samples_status   ON samples(status);
CREATE INDEX idx_samples_pending  ON samples(source_id, id) WHERE analyzed_at IS NULL;
CREATE INDEX idx_samples_hash     ON samples(content_hash) WHERE content_hash IS NOT NULL;
CREATE INDEX idx_samples_duration ON samples(duration_ms);

-- Búsqueda incremental
CREATE VIRTUAL TABLE samples_fts USING fts5(
  filename, rel_path, tags,
  content = '', tokenize = "unicode61 remove_diacritics 2"
);

-- Sesión de triaje: origen + raíz de destino + modo
CREATE TABLE projects (
  id          INTEGER PRIMARY KEY,
  name        TEXT NOT NULL,
  dest_root   TEXT NOT NULL,
  mode        TEXT NOT NULL DEFAULT 'move',   -- move | copy
  created_at  INTEGER NOT NULL,
  opened_at   INTEGER
);

-- Los "cubos" a los que se envía con 1..9
CREATE TABLE destinations (
  id          INTEGER PRIMARY KEY,
  project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,         -- "Kicks", "Snares", "FX"
  rel_path    TEXT NOT NULL,         -- relativa a projects.dest_root
  hotkey      TEXT,                  -- '1'..'9'
  color       TEXT,                  -- nombre de token, nunca un hex
  sort_order  INTEGER NOT NULL DEFAULT 0,
  count       INTEGER NOT NULL DEFAULT 0,
  UNIQUE (project_id, hotkey)
);

-- Journal de operaciones de archivo: la base del undo y de la reparación al arrancar
CREATE TABLE actions (
  id            INTEGER PRIMARY KEY,
  project_id    INTEGER REFERENCES projects(id) ON DELETE SET NULL,
  sample_id     INTEGER REFERENCES samples(id) ON DELETE SET NULL,
  kind          TEXT NOT NULL,       -- move | copy | reject | restore | rename
  from_path     TEXT NOT NULL,
  to_path       TEXT NOT NULL,
  created_at    INTEGER NOT NULL,    -- se escribe ANTES de tocar el disco
  done_at       INTEGER,             -- NULL = quedó a medias → reparar al arrancar
  undone_at     INTEGER,
  batch_id      TEXT                 -- agrupa una operación múltiple en un solo undo
);

CREATE INDEX idx_actions_undo    ON actions(project_id, done_at DESC) WHERE undone_at IS NULL;
CREATE INDEX idx_actions_pending ON actions(id) WHERE done_at IS NULL;

CREATE TABLE tags (
  id    INTEGER PRIMARY KEY,
  name  TEXT NOT NULL UNIQUE
);

CREATE TABLE sample_tags (
  sample_id INTEGER NOT NULL REFERENCES samples(id) ON DELETE CASCADE,
  tag_id    INTEGER NOT NULL REFERENCES tags(id)    ON DELETE CASCADE,
  PRIMARY KEY (sample_id, tag_id)
);

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL           -- JSON
);
```

## Decisiones y sus porqués

**`peaks` como BLOB, no como JSON.** Un waveform de 1.000 buckets son 2 KB en binario y ~12 KB en
JSON, con su parseo. Se envía al front como bytes crudos y se pinta directamente en el canvas.

**`content_hash` sobre las muestras PCM, no sobre el archivo.** Hashear el PCM decodificado
(mezclado a mono y cuantizado a 16 bits) detecta lo que un hash exacto sí puede detectar: **el
mismo audio en distinto envoltorio**. El mismo kick como WAV y como FLAC, con otros metadatos, o
duplicado con otro nombre en tres packs distintos, cae en el mismo hash. Es el caso que de verdad
molesta al ordenar librerías descargadas.

Lo que **no** detecta, y conviene no prometer: dos exportaciones del mismo master con dither
distinto, o con 0,1 dB de diferencia de nivel. Eso es parecido perceptual y necesita una huella
acústica, no un hash — está en la Fase 5, no aquí. Hay un test que fija justamente este límite
(`el_hash_no_sobrevive_a_un_dither_distinto`), para que nadie lo confunda más adelante.

Se calcula solo para archivos < 30 s: el resto casi nunca son duplicados de pack y hashearlos
multiplicaría el coste del análisis sin ganar nada.

**`(size, mtime)` como testigo de cambio.** Reanalizar 50.000 archivos en cada arranque es
inaceptable; comparar dos enteros es gratis.

**Índice parcial `idx_samples_pending`.** La cola del analizador se consulta constantemente:
`WHERE analyzed_at IS NULL` con índice parcial es O(log n) sin engordar el índice general.

**El journal se escribe antes de tocar el disco.** Es la única forma de que un corte de luz a
mitad de un `move` sea reparable. `done_at IS NULL` al arrancar = investigar y arreglar.

**`batch_id`.** Enviar 40 samples seleccionados a un destino debe deshacerse con **un** `Ctrl+Z`,
no con cuarenta.

**FTS5 con `content=''`.** Tabla externa sin contenido duplicado: el índice pesa lo mínimo y las
filas viven solo en `samples`. Se sincroniza con triggers en las tres operaciones.

## Migraciones

Numeradas y versionadas en `src-tauri/src/db/migrations.rs`, aplicadas dentro de una transacción
y registradas en `PRAGMA user_version`. Cada migración es `up` únicamente: si algo va mal, se
borra el `.db` y se reindexa — es una caché. Nunca hay que escribir un `down` para datos que se
pueden regenerar, pero las tablas de decisiones (`projects`, `destinations`, `actions`, `tags`,
`sample_tags`) se exportan antes de cualquier migración destructiva.
