//! Migraciones versionadas con `PRAGMA user_version`, cada una en su transacción.
//!
//! Solo hay `up`: el índice se puede reconstruir escaneando de nuevo. Lo que sí duele perder
//! son las decisiones del usuario (proyectos, destinos, journal), y esas se exportan aparte.

use crate::error::Result;
use rusqlite::Connection;

const MIGRACIONES: &[(&str, &str)] = &[
    ("001_esquema_inicial", ESQUEMA_INICIAL),
    ("002_etiquetas", ETIQUETAS),
];

pub fn aplicar(conn: &mut Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, (nombre, sql)) in MIGRACIONES.iter().enumerate() {
        let numero = i as i64 + 1;
        if numero <= version {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)
            .map_err(|e| crate::error::AppError::Db(format!("migración {nombre}: {e}")))?;
        tx.pragma_update(None, "user_version", numero)?;
        tx.commit()?;
    }
    Ok(())
}

const ESQUEMA_INICIAL: &str = r#"
CREATE TABLE sources (
  id         INTEGER PRIMARY KEY,
  path       TEXT    NOT NULL UNIQUE,
  added_at   INTEGER NOT NULL,
  last_scan  INTEGER
);

CREATE TABLE samples (
  id            INTEGER PRIMARY KEY,
  source_id     INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
  rel_path      TEXT    NOT NULL,
  filename      TEXT    NOT NULL,
  ext           TEXT    NOT NULL,
  size          INTEGER NOT NULL,
  mtime         INTEGER NOT NULL,

  duration_ms   INTEGER,
  sample_rate   INTEGER,
  channels      INTEGER,
  bit_depth     INTEGER,
  loudness_db   REAL,
  peaks         BLOB,
  content_hash  BLOB,
  analyzed_at   INTEGER,
  broken        INTEGER NOT NULL DEFAULT 0,

  status        TEXT    NOT NULL DEFAULT 'pending',
  rating        INTEGER NOT NULL DEFAULT 0,
  seen_at       INTEGER,
  dest_id       INTEGER REFERENCES destinations(id) ON DELETE SET NULL,
  current_path  TEXT,

  UNIQUE (source_id, rel_path)
);

CREATE INDEX idx_samples_status   ON samples(status);

-- Índices que CUBREN el ORDER BY, con la misma colación que usa la consulta. Sin esto,
-- pedir la página 245 obliga a SQLite a ordenar las 50.000 filas enteras: medido en 82 ms,
-- por encima del presupuesto de 50 ms (docs/PERFORMANCE.md).
CREATE INDEX idx_samples_orden_path   ON samples(source_id, rel_path COLLATE NOCASE, id);
CREATE INDEX idx_samples_orden_nombre ON samples(source_id, filename COLLATE NOCASE, id);
CREATE INDEX idx_samples_orden_dur    ON samples(source_id, duration_ms, id);
CREATE INDEX idx_samples_pending  ON samples(source_id, id) WHERE analyzed_at IS NULL AND broken = 0;
CREATE INDEX idx_samples_hash     ON samples(content_hash) WHERE content_hash IS NOT NULL;
CREATE INDEX idx_samples_duration ON samples(duration_ms);

CREATE VIRTUAL TABLE samples_fts USING fts5(
  filename,
  rel_path,
  content = 'samples',
  content_rowid = 'id',
  tokenize = "unicode61 remove_diacritics 2"
);

CREATE TRIGGER samples_fts_ai AFTER INSERT ON samples BEGIN
  INSERT INTO samples_fts(rowid, filename, rel_path) VALUES (new.id, new.filename, new.rel_path);
END;
CREATE TRIGGER samples_fts_ad AFTER DELETE ON samples BEGIN
  INSERT INTO samples_fts(samples_fts, rowid, filename, rel_path)
    VALUES ('delete', old.id, old.filename, old.rel_path);
END;
CREATE TRIGGER samples_fts_au AFTER UPDATE OF filename, rel_path ON samples BEGIN
  INSERT INTO samples_fts(samples_fts, rowid, filename, rel_path)
    VALUES ('delete', old.id, old.filename, old.rel_path);
  INSERT INTO samples_fts(rowid, filename, rel_path) VALUES (new.id, new.filename, new.rel_path);
END;

CREATE TABLE projects (
  id          INTEGER PRIMARY KEY,
  name        TEXT    NOT NULL,
  dest_root   TEXT    NOT NULL,
  mode        TEXT    NOT NULL DEFAULT 'move',
  created_at  INTEGER NOT NULL,
  opened_at   INTEGER,
  last_sample_id INTEGER
);

CREATE TABLE destinations (
  id          INTEGER PRIMARY KEY,
  project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name        TEXT    NOT NULL,
  rel_path    TEXT    NOT NULL,
  hotkey      TEXT,
  color       TEXT    NOT NULL DEFAULT 'dest-1',
  sort_order  INTEGER NOT NULL DEFAULT 0,
  count       INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX idx_dest_hotkey ON destinations(project_id, hotkey) WHERE hotkey IS NOT NULL;

CREATE TABLE actions (
  id          INTEGER PRIMARY KEY,
  project_id  INTEGER REFERENCES projects(id) ON DELETE SET NULL,
  sample_id   INTEGER REFERENCES samples(id) ON DELETE SET NULL,
  dest_id     INTEGER REFERENCES destinations(id) ON DELETE SET NULL,
  kind        TEXT    NOT NULL,
  from_path   TEXT    NOT NULL,
  to_path     TEXT    NOT NULL,
  prev_status TEXT    NOT NULL DEFAULT 'pending',
  prev_dest   INTEGER,
  prev_current TEXT,
  created_at  INTEGER NOT NULL,
  done_at     INTEGER,
  undone_at   INTEGER,
  batch_id    TEXT    NOT NULL
);
CREATE INDEX idx_actions_undo    ON actions(batch_id, done_at) WHERE undone_at IS NULL;
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
  value TEXT NOT NULL
);
"#;

/// Migración 002 · el conjunto de evaluación de la Fase 8.
///
/// Una etiqueta es «lo que sabemos de este sample y de dónde lo sabemos». Hay tres orígenes y
/// no valen lo mismo: `filename` es barato y masivo pero miente a veces, `user` es la única
/// verdad sin discusión, y `audio` será lo que estime el clasificador. Guardarlos por separado
/// —en vez de machacar un único campo— es lo que permite medir el acierto de unos contra otros.
const ETIQUETAS: &str = r#"
CREATE TABLE labels (
  id          INTEGER PRIMARY KEY,
  sample_id   INTEGER NOT NULL REFERENCES samples(id) ON DELETE CASCADE,
  field       TEXT    NOT NULL,           -- kind | bpm | key | pitch
  value       TEXT    NOT NULL,
  confidence  REAL    NOT NULL DEFAULT 1.0,
  source      TEXT    NOT NULL,           -- filename | user | audio
  created_at  INTEGER NOT NULL,
  UNIQUE (sample_id, field, source)
);

CREATE INDEX idx_labels_campo  ON labels(field, source);
CREATE INDEX idx_labels_sample ON labels(sample_id);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migra_desde_cero_y_es_idempotente() {
        let mut conn = Connection::open_in_memory().unwrap();
        aplicar(&mut conn).unwrap();
        aplicar(&mut conn).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRACIONES.len() as i64);
        let tablas: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='samples'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tablas, 1);
    }
}
