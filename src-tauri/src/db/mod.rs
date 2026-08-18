//! Acceso a SQLite. El índice es una caché reconstruible: la verdad son los archivos en disco.
//!
//! Un pool minúsculo hecho a mano en vez de r2d2: una conexión de escritura protegida por
//! Mutex y un saco de conexiones de lectura. Con WAL, lecturas y escrituras no se bloquean,
//! así que la lista sigue respondiendo mientras se indexa.

pub mod migrations;
pub mod queries;
pub mod triage;

use crate::error::{AppError, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Db {
    path: PathBuf,
    write: Mutex<Connection>,
    readers: Mutex<Vec<Connection>>,
}

fn configurar(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous  = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;
         PRAGMA temp_store   = MEMORY;
         PRAGMA cache_size   = -32000;",
    )?;
    Ok(())
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut conn = Connection::open(path)?;
        configurar(&conn)?;
        migrations::aplicar(&mut conn)?;
        Ok(Self {
            path: path.to_path_buf(),
            write: Mutex::new(conn),
            readers: Mutex::new(Vec::new()),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn nueva_lectura(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        configurar(&conn)?;
        Ok(conn)
    }

    /// Ejecuta una lectura. Reutiliza una conexión del pool o abre una nueva.
    pub fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = {
            let mut pool = self
                .readers
                .lock()
                .map_err(|_| AppError::Db("pool de lectura envenenado".into()))?;
            pool.pop()
        };
        let conn = match conn {
            Some(c) => c,
            None => self.nueva_lectura()?,
        };
        let salida = f(&conn);
        if let Ok(mut pool) = self.readers.lock() {
            if pool.len() < 8 {
                pool.push(conn);
            }
        }
        salida
    }

    /// Ejecuta una escritura con la única conexión de escritura.
    pub fn write<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut conn = self
            .write
            .lock()
            .map_err(|_| AppError::Db("conexión de escritura envenenada".into()))?;
        f(&mut conn)
    }
}

pub fn ahora_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
