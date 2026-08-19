//! Comandos del reproductor. Ninguno bloquea: el motor vive en su propio hilo.

use super::Estado;
use crate::codec;
use crate::db::queries;
use crate::domain::*;
use crate::error::Result;
use tauri::State;

/// Reproduce un sample. Devuelve en el acto: la decodificación (si hace falta) ocurre en el
/// hilo del motor, y el prefetch hace que casi siempre ya esté en RAM.
#[tauri::command]
pub async fn player_play(
    estado: State<'_, Estado>,
    sample_id: i64,
    looping: bool,
) -> Result<PlaybackStarted> {
    let audio = estado.audio()?;
    let (ruta, detalle, picos) = estado.db.read(|c| {
        Ok((
            queries::abs_path(c, sample_id)?,
            queries::detail(c, sample_id)?,
            queries::peaks(c, sample_id)?,
        ))
    })?;

    let duracion = detalle.row.duration_ms.unwrap_or(0);
    // En un loop de cuatro compases lo interesante no está en el primer milisegundo.
    let inicio = codec::start_offset_ms(&picos, duracion) as f64;

    audio.play(sample_id, ruta, inicio, looping);
    let _ = estado.db.read(|c| queries::mark_seen(c, sample_id));

    Ok(PlaybackStarted {
        sample_id,
        started_at_ms: crate::audio::ahora_ms(),
        duration_ms: duracion as f64,
        start_offset_ms: inicio,
        looping,
    })
}

#[tauri::command]
pub async fn player_stop(estado: State<'_, Estado>) -> Result<()> {
    estado.audio()?.stop();
    Ok(())
}

#[tauri::command]
pub async fn player_seek(estado: State<'_, Estado>, ms: f64) -> Result<()> {
    estado.audio()?.seek_ms(ms);
    Ok(())
}

#[tauri::command]
pub async fn player_gain(estado: State<'_, Estado>, gain: f32) -> Result<()> {
    estado.audio()?.gain(gain);
    Ok(())
}

#[tauri::command]
pub async fn player_set_loop(estado: State<'_, Estado>, looping: bool) -> Result<()> {
    estado.audio()?.set_looping(looping);
    Ok(())
}

/// Decodifica por adelantado los vecinos de la selección. Es lo que convierte 12 ms de
/// latencia en 2,6 ms cuando el usuario pulsa la flecha.
#[tauri::command]
pub async fn player_prefetch(estado: State<'_, Estado>, sample_ids: Vec<i64>) -> Result<()> {
    let audio = estado.audio()?;
    let rutas = estado.db.read(|c| queries::abs_paths(c, &sample_ids))?;
    audio.prefetch(rutas);
    Ok(())
}

/// Vuelve a abrir el dispositivo de salida. La app lo hace sola cuando detecta que el stream
/// ha dejado de latir; esto es la salida de emergencia por si en algún sistema no basta.
#[tauri::command]
pub async fn player_reconnect(estado: State<'_, Estado>) -> Result<()> {
    estado.audio()?.reconectar();
    Ok(())
}

#[tauri::command]
pub async fn player_info(estado: State<'_, Estado>) -> Result<AudioInfo> {
    Ok(estado.audio()?.info())
}
