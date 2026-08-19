//! Pruebas del motor de audio contra un dispositivo REAL.
//!
//! Van marcadas como ignoradas porque necesitan tarjeta de sonido: en el CI no hay ninguna.
//! Para ejecutarlas:
//!
//!   cargo test --release --test audio -- --ignored --nocapture

use samplecurator_lib::audio;
use std::time::Duration;

#[test]
#[ignore = "necesita un dispositivo de salida real"]
fn el_motor_se_reconecta_sin_reiniciar_la_app() {
    let handle = match audio::arrancar() {
        Ok(h) => h,
        Err(e) => {
            println!("sin dispositivo de salida ({e}); nada que probar");
            return;
        }
    };

    let antes = handle.info();
    println!(
        "al arrancar: {} Hz · {} canales · buffer {} · reconexiones {}",
        antes.sample_rate, antes.channels, antes.buffer_frames, antes.reconnections
    );
    assert!(antes.sample_rate > 0, "el motor debería estar sonando");
    assert_eq!(antes.reconnections, 0);

    // Esto es exactamente lo que hace la app cuando detecta que el stream dejó de latir.
    handle.reconectar();
    std::thread::sleep(Duration::from_millis(1500));

    let despues = handle.info();
    println!(
        "tras reconectar: {} Hz · {} canales · reconexiones {}",
        despues.sample_rate, despues.channels, despues.reconnections
    );
    assert_eq!(
        despues.reconnections, 1,
        "la reconexión debería haberse contado"
    );
    assert!(despues.sample_rate > 0, "y el motor tiene que seguir vivo");
    assert_eq!(
        despues.channels, antes.channels,
        "mismo dispositivo, misma configuración"
    );
}

#[test]
#[ignore = "necesita un dispositivo de salida real"]
fn el_vigilante_no_reconecta_cuando_todo_va_bien() {
    let Ok(handle) = audio::arrancar() else {
        println!("sin dispositivo de salida; nada que probar");
        return;
    };

    // Tres segundos de vida tranquila: el vigilante mira cada 500 ms, así que si fuera a
    // disparar por error ya habría disparado seis veces.
    std::thread::sleep(Duration::from_secs(3));

    let info = handle.info();
    println!("tras 3 s en reposo: reconexiones {}", info.reconnections);
    assert_eq!(
        info.reconnections, 0,
        "un stream sano no puede provocar reconexiones: sonarían como cortes"
    );
}
