/**
 * Estado del reproductor.
 *
 * El cabezal NO vive aquí: se interpola con requestAnimationFrame desde `iniciadoEn` y
 * `duracionMs` dentro del canvas. Un evento IPC (o un re-render de React) por frame sería
 * la forma más fácil de arruinar esta app.
 */
import { create } from "zustand";
import * as ipc from "../../lib/ipc";
import { log } from "../../lib/log";

const MAX_PICOS_EN_CACHE = 64;

interface PlayerState {
  sonando: number | null;
  /** `performance.now()` del momento en que se pidió la reproducción. */
  iniciadoEn: number;
  duracionMs: number;
  offsetMs: number;
  bucle: boolean;
  volumen: number;
  silenciado: boolean;
  autoplay: boolean;
  normalizar: boolean;
  picos: Int8Array | null;
  picosDe: number | null;

  reproducir: (sampleId: number) => Promise<void>;
  parar: () => Promise<void>;
  repetir: () => Promise<void>;
  alternarBucle: () => Promise<void>;
  alternarSilencio: () => Promise<void>;
  alternarAutoplay: () => void;
  alternarNormalizar: () => void;
  ajustarVolumen: (delta: number) => Promise<void>;
  ponerVolumen: (v: number) => Promise<void>;
  saltarRelativo: (segundos: number) => Promise<void>;
  saltarA: (fraccion: number) => Promise<void>;
  prefetch: (ids: number[]) => void;
}

const cachePicos = new Map<number, Int8Array>();

function guardarPicos(id: number, datos: Int8Array): void {
  if (cachePicos.size >= MAX_PICOS_EN_CACHE) {
    const primera = cachePicos.keys().next();
    if (!primera.done) cachePicos.delete(primera.value);
  }
  cachePicos.set(id, datos);
}

export const usePlayerStore = create<PlayerState>((set, get) => ({
  sonando: null,
  iniciadoEn: 0,
  duracionMs: 0,
  offsetMs: 0,
  bucle: false,
  volumen: 0.9,
  silenciado: false,
  autoplay: true,
  normalizar: false,
  picos: null,
  picosDe: null,

  async reproducir(sampleId) {
    const s = get();
    // El reloj se toma ANTES de la llamada: el cabezal debe arrancar con la tecla, no con
    // la respuesta.
    const iniciadoEn = performance.now();

    const enCache = cachePicos.get(sampleId);
    if (enCache) set({ picos: enCache, picosDe: sampleId });
    else if (s.picosDe !== sampleId) set({ picos: null, picosDe: sampleId });

    try {
      const r = await ipc.reproducir(sampleId, s.bucle);
      set({
        sonando: sampleId,
        iniciadoEn,
        duracionMs: r.durationMs,
        offsetMs: r.startOffsetMs,
      });
    } catch (e) {
      log.warn("no se pudo reproducir", e);
      set({ sonando: null });
    }

    if (!enCache) {
      try {
        const datos = await ipc.picos(sampleId);
        guardarPicos(sampleId, datos);
        if (get().picosDe === sampleId) set({ picos: datos });
      } catch (e) {
        log.warn("no se pudieron cargar los picos", e);
      }
    }
  },

  async parar() {
    await ipc.parar().catch((e) => log.warn("no se pudo parar", e));
    set({ sonando: null });
  },

  async repetir() {
    const id = get().sonando;
    if (id !== null) await get().reproducir(id);
  },

  async alternarBucle() {
    const bucle = !get().bucle;
    set({ bucle });
    await ipc.bucle(bucle).catch((e) => log.warn("no se pudo cambiar el bucle", e));
  },

  async alternarSilencio() {
    const silenciado = !get().silenciado;
    set({ silenciado });
    await ipc
      .ganancia(silenciado ? 0 : get().volumen)
      .catch((e) => log.warn("no se pudo silenciar", e));
  },

  alternarAutoplay() {
    set({ autoplay: !get().autoplay });
  },

  alternarNormalizar() {
    set({ normalizar: !get().normalizar });
  },

  async ajustarVolumen(delta) {
    const volumen = Math.min(2, Math.max(0, Number((get().volumen + delta).toFixed(2))));
    set({ volumen, silenciado: false });
    await ipc.ganancia(volumen).catch((e) => log.warn("no se pudo ajustar el volumen", e));
  },

  /** Volumen absoluto, para el deslizador. Hasta 150 % porque hay samples grabados bajísimos. */
  async ponerVolumen(v) {
    const volumen = Math.min(1.5, Math.max(0, Number(v.toFixed(2))));
    set({ volumen, silenciado: false });
    await ipc.ganancia(volumen).catch((e) => log.warn("no se pudo ajustar el volumen", e));
  },

  async saltarRelativo(segundos) {
    const s = get();
    if (s.sonando === null || s.duracionMs === 0) return;
    const transcurrido = performance.now() - s.iniciadoEn + s.offsetMs;
    const destino = Math.min(Math.max(0, transcurrido + segundos * 1000), s.duracionMs);
    set({ iniciadoEn: performance.now(), offsetMs: destino });
    await ipc.buscarEn(destino).catch((e) => log.warn("no se pudo saltar", e));
  },

  async saltarA(fraccion) {
    const s = get();
    if (s.sonando === null || s.duracionMs === 0) return;
    const destino = Math.min(Math.max(0, fraccion * s.duracionMs), s.duracionMs);
    set({ iniciadoEn: performance.now(), offsetMs: destino });
    await ipc.buscarEn(destino).catch((e) => log.warn("no se pudo saltar", e));
  },

  prefetch(ids) {
    if (ids.length === 0) return;
    void ipc.prefetch(ids).catch(() => {
      /* el prefetch es best-effort: si falla, se decodifica al reproducir */
    });
  },
}));

/** Posición del cabezal en ms. La lee el canvas en cada frame, fuera de React. */
export function posicionCabezal(): number {
  const s = usePlayerStore.getState();
  if (s.sonando === null || s.duracionMs === 0) return 0;
  const t = performance.now() - s.iniciadoEn + s.offsetMs;
  if (s.bucle && s.duracionMs > 0) return t % s.duracionMs;
  return Math.min(t, s.duracionMs);
}
