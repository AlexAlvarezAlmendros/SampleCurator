//! Análisis musical: tipo de sample, tempo y tonalidad.
//!
//! Depende de `domain` y de `codec`, y de nada más: no conoce la base de datos, ni Tauri, ni
//! el sistema de archivos. Eso lo hace testeable sin montar nada.
//!
//! Principio que ordena todo el módulo: **callarse es una respuesta correcta**. Un BPM
//! inventado sobre un kick o una tonalidad inventada sobre un hi-hat ensucian los filtros y
//! hacen que el usuario deje de fiarse de la columna entera. Antes de estimar nada hay que
//! decidir si la pregunta tiene sentido para este sample.

pub mod evaluacion;
pub mod filename;
