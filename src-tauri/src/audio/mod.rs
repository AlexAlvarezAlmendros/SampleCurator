//! Motor de audio. Nada de aquí conoce la base de datos ni Tauri.
//!
//! Reparto de responsabilidades:
//!   `graph`  → código de tiempo real: mezcla, fades, loop. Sin allocs, sin locks, sin I/O.
//!   `cache`  → LRU por bytes de audio ya decodificado. Solo hilos de control.
//!   `engine` → hilo que posee el stream de cpal y traduce peticiones a mandos del grafo.

pub mod cache;
pub mod engine;
pub mod graph;

pub use engine::{ahora_ms, arrancar, AudioHandle};
