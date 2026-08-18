//! Caché LRU de audio ya decodificado, con tope en BYTES (no en número de elementos).
//!
//! Un one-shot de 0,3 s y un loop de 30 s no ocupan lo mismo, así que contar elementos no
//! sirve para acotar la memoria. Solo la tocan hilos de control: el callback de audio jamás.

use crate::codec::AudioBuffer;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Cache {
    limite: usize,
    bytes: usize,
    reloj: u64,
    mapa: HashMap<i64, Entrada>,
}

struct Entrada {
    buf: Arc<AudioBuffer>,
    bytes: usize,
    ultimo_uso: u64,
}

impl Cache {
    pub fn nueva(limite_bytes: usize) -> Self {
        Self {
            limite: limite_bytes,
            bytes: 0,
            reloj: 0,
            mapa: HashMap::new(),
        }
    }

    pub fn obtener(&mut self, id: i64) -> Option<Arc<AudioBuffer>> {
        self.reloj += 1;
        let reloj = self.reloj;
        let e = self.mapa.get_mut(&id)?;
        e.ultimo_uso = reloj;
        Some(Arc::clone(&e.buf))
    }

    pub fn contiene(&self, id: i64) -> bool {
        self.mapa.contains_key(&id)
    }

    pub fn insertar(&mut self, id: i64, buf: Arc<AudioBuffer>) {
        let bytes = buf.bytes();
        // Un sample más grande que la caché entera no se guarda: desalojaría todo lo demás
        // para nada.
        if bytes > self.limite {
            return;
        }
        self.reloj += 1;
        if let Some(anterior) = self.mapa.insert(
            id,
            Entrada {
                buf,
                bytes,
                ultimo_uso: self.reloj,
            },
        ) {
            self.bytes -= anterior.bytes;
        }
        self.bytes += bytes;
        self.desalojar();
    }

    fn desalojar(&mut self) {
        while self.bytes > self.limite && self.mapa.len() > 1 {
            let victima = self
                .mapa
                .iter()
                .min_by_key(|(_, e)| e.ultimo_uso)
                .map(|(k, _)| *k);
            match victima {
                Some(k) => {
                    if let Some(e) = self.mapa.remove(&k) {
                        self.bytes -= e.bytes;
                    }
                }
                None => break,
            }
        }
    }

    /// Saca un sample de la caché: su archivo ya no está donde estaba.
    pub fn quitar(&mut self, id: i64) {
        if let Some(e) = self.mapa.remove(&id) {
            self.bytes -= e.bytes;
        }
    }

    pub fn limpiar(&mut self) {
        self.mapa.clear();
        self.bytes = 0;
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn limite(&self) -> usize {
        self.limite
    }
    pub fn entradas(&self) -> usize {
        self.mapa.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(frames: usize) -> Arc<AudioBuffer> {
        Arc::new(AudioBuffer {
            samples: vec![0.0; frames],
            channels: 1,
            sample_rate: 44_100,
            bit_depth: None,
        })
    }

    #[test]
    fn desaloja_por_bytes_no_por_numero_de_elementos() {
        let mut c = Cache::nueva(4000); // 1000 muestras f32
        c.insertar(1, buf(400));
        c.insertar(2, buf(400));
        assert_eq!(c.entradas(), 2);
        c.insertar(3, buf(400)); // 1200 muestras > 1000: hay que desalojar
        assert!(c.bytes() <= c.limite());
        assert!(c.entradas() < 3);
    }

    #[test]
    fn desaloja_el_menos_usado_recientemente() {
        let mut c = Cache::nueva(4000);
        c.insertar(1, buf(400));
        c.insertar(2, buf(400));
        let _ = c.obtener(1); // 1 pasa a ser el más reciente
        c.insertar(3, buf(400));
        assert!(c.contiene(1), "el recién usado debe sobrevivir");
        assert!(!c.contiene(2), "el más antiguo es el que cae");
    }

    #[test]
    fn un_sample_mas_grande_que_la_cache_no_entra() {
        let mut c = Cache::nueva(1000);
        c.insertar(1, buf(400)); // 1600 bytes > 1000
        assert_eq!(c.entradas(), 0);
        assert_eq!(c.bytes(), 0);
    }

    #[test]
    fn reinsertar_el_mismo_id_no_duplica_la_cuenta() {
        let mut c = Cache::nueva(1_000_000);
        c.insertar(1, buf(100));
        let b1 = c.bytes();
        c.insertar(1, buf(100));
        assert_eq!(c.bytes(), b1);
        assert_eq!(c.entradas(), 1);
    }
}
