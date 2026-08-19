//! Prueba del canal de actualización, contra la release DE VERDAD.
//!
//! Comprueba de una vez las cuatro cosas que pueden estar mal en nuestro montaje y que no se
//! notan hasta que alguien se queda sin poder actualizar:
//!
//! 1. el endpoint que lleva compilado la app responde y trae un `latest.json` válido;
//! 2. ese fichero tiene una entrada para esta plataforma;
//! 3. el paquete que anuncia se puede descargar;
//! 4. su firma casa con la clave pública que va dentro de la app.
//!
//! El 4 es el que importa: si el par de claves se regenera y alguien olvida actualizar
//! `tauri.conf.json`, las apps instaladas rechazarán en silencio todas las actualizaciones.
//!
//! Ignorada porque necesita red y una release publicada:
//!
//!   cargo test --test actualizacion -- --ignored --nocapture

use std::process::Command;

/// Lee del `tauri.conf.json` real: lo que se prueba es lo que se distribuye, no una copia.
fn config() -> serde_json::Value {
    let texto = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json"))
        .expect("no se pudo leer tauri.conf.json");
    serde_json::from_str(&texto).expect("tauri.conf.json no es JSON válido")
}

fn bajar(url: &str) -> Vec<u8> {
    let salida = Command::new("curl")
        .args(["-sSL", "--fail", url])
        .output()
        .expect("hace falta curl para esta prueba");
    assert!(salida.status.success(), "no se pudo descargar {url}");
    salida.stdout
}

/// Igual que el actualizador: `linux-x86_64`, `windows-x86_64`, `darwin-aarch64`…
fn plataforma() -> String {
    let so = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "darwin"
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else {
        "aarch64"
    };
    format!("{so}-{arch}")
}

#[test]
#[ignore = "necesita red y una release publicada"]
fn el_canal_de_actualizacion_esta_bien_montado() {
    let conf = config();
    let endpoint = conf["plugins"]["updater"]["endpoints"][0]
        .as_str()
        .expect("falta el endpoint del actualizador en tauri.conf.json");
    let pubkey_b64 = conf["plugins"]["updater"]["pubkey"]
        .as_str()
        .expect("falta la clave pública del actualizador en tauri.conf.json");

    // 1 · el endpoint responde con un latest.json válido
    let crudo = bajar(endpoint);
    let latest: serde_json::Value =
        serde_json::from_slice(&crudo).expect("el latest.json publicado no es JSON válido");
    let version = latest["version"]
        .as_str()
        .expect("el latest.json no trae versión");
    println!("endpoint: {endpoint}\núltima versión publicada: {version}");

    // 2 · con entrada para esta plataforma
    let plat = plataforma();
    let entrada = &latest["platforms"][&plat];
    assert!(
        !entrada.is_null(),
        "el latest.json no tiene entrada para «{plat}»: en este sistema nadie podría actualizar"
    );
    let url = entrada["url"].as_str().expect("la entrada no trae url");
    let firma = entrada["signature"]
        .as_str()
        .expect("la entrada no trae firma");

    // 3 · el paquete se descarga
    println!("descargando {url}");
    let paquete = bajar(url);
    assert!(
        paquete.len() > 1_000_000,
        "el paquete descargado son {} bytes: eso no es un instalador",
        paquete.len()
    );

    // 4 · y su firma casa con la clave pública que lleva la app
    let pubkey_txt = String::from_utf8(
        base64_decode(pubkey_b64).expect("la clave pública de la config no es base64"),
    )
    .expect("la clave pública no es texto");
    // El fichero de clave son dos líneas: un comentario y la clave. Aquí interesa la segunda.
    let clave = minisign_verify::PublicKey::from_base64(
        pubkey_txt
            .lines()
            .next_back()
            .expect("la clave pública está vacía"),
    )
    .expect("la clave pública no tiene formato minisign");

    let firma_txt =
        String::from_utf8(base64_decode(firma).expect("la firma no es base64")).expect("firma");
    let firma = minisign_verify::Signature::decode(&firma_txt).expect("firma mal formada");

    clave
        .verify(&paquete, &firma, true)
        .expect("LA FIRMA NO CASA: las apps instaladas rechazarían esta actualización");

    println!(
        "firma verificada contra la clave pública de la app · {} bytes",
        paquete.len()
    );
}

/// Base64 sin dependencias: solo se usa aquí, para el par clave/firma.
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const TABLA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut inv = [255u8; 256];
    for (i, c) in TABLA.iter().enumerate() {
        inv[*c as usize] = i as u8;
    }
    let limpio: Vec<u8> = s
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'=')
        .collect();
    let mut salida = Vec::with_capacity(limpio.len() * 3 / 4);
    for trozo in limpio.chunks(4) {
        let mut acc = 0u32;
        for (i, b) in trozo.iter().enumerate() {
            let v = inv[*b as usize];
            if v == 255 {
                return None;
            }
            acc |= u32::from(v) << (18 - 6 * i);
        }
        let bytes = [(acc >> 16) as u8, (acc >> 8) as u8, acc as u8];
        salida.extend_from_slice(&bytes[..trozo.len() - 1]);
    }
    Some(salida)
}
