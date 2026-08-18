/**
 * Modo etiquetado (Fase 8, gate 8.0).
 *
 * Reutiliza la ergonomía del triaje —una tecla por decisión, con el sample sonando— pero en
 * vez de mover archivos escribe la verdad de referencia. Sin una herramienta rápida, las 200
 * correcciones que necesita la evaluación no se hacen nunca.
 */
import { create } from "zustand";
import { useUiStore } from "../../app/uiStore";
import type { LabelStats, SampleKind, SampleLabels } from "../../bindings";
import * as ipc from "../../lib/ipc";
import { esAppError } from "../../lib/ipc";
import { log } from "../../lib/log";
import { consultaActual, filaEn, useLibraryStore } from "../library/store";

/** Letra de cada clase. Mnemónicas, para que la mano las aprenda en dos minutos. */
export const TECLAS_TIPO: Array<[string, SampleKind, string]> = [
  ["k", "kick", "Bombo"],
  ["s", "snare", "Caja"],
  ["c", "clap", "Palmas"],
  ["h", "hat", "Charles"],
  ["y", "cymbal", "Plato"],
  ["t", "tom", "Tom"],
  ["p", "perc", "Percusión"],
  ["b", "bass", "Bajo"],
  ["n", "synth", "Sintetizador"],
  ["v", "vocal", "Voz"],
  ["f", "fx", "Efecto"],
  ["l", "loop", "Bucle"],
  ["u", "unknown", "No sé"],
];

interface LabelsState {
  modo: boolean;
  etiquetas: SampleLabels | null;
  stats: LabelStats | null;
  /** Muestra estratificada por la que avanza la sesión. */
  cola: number[];
  extrayendo: boolean;

  alternarModo: () => Promise<void>;
  cargarDe: (sampleId: number) => Promise<void>;
  poner: (field: string, value: string) => Promise<void>;
  ponerTipo: (kind: SampleKind) => Promise<void>;
  refrescarStats: () => Promise<void>;
  extraerDeNombres: () => Promise<void>;
  siguienteDeLaCola: () => Promise<void>;
}

export const useLabelsStore = create<LabelsState>((set, get) => ({
  modo: false,
  etiquetas: null,
  stats: null,
  cola: [],
  extrayendo: false,

  async alternarModo() {
    const modo = !get().modo;
    set({ modo });
    if (!modo) return;

    await get().refrescarStats();

    // Lo primero, las etiquetas de lo que el usuario ya tiene delante: entrar en el modo y
    // ver el panel vacío durante un segundo es desconcertante.
    const enFoco = filaEn(useLibraryStore.getState(), useLibraryStore.getState().foco)?.id;
    if (enFoco !== undefined) await get().cargarDe(enFoco);

    try {
      const cola = await ipc.muestraParaEtiquetar(20);
      set({ cola });
      if (cola.length > 0) {
        useUiStore
          .getState()
          .avisar("info", `Modo etiquetado: ${cola.length} samples repartidos por tipo`);
        await get().siguienteDeLaCola();
      }
    } catch (e) {
      log.warn("no se pudo preparar la muestra", e);
    }
  },

  async cargarDe(sampleId) {
    try {
      set({ etiquetas: await ipc.etiquetasDe(sampleId) });
    } catch (e) {
      log.warn("no se pudieron leer las etiquetas", e);
      set({ etiquetas: null });
    }
  },

  async poner(field, value) {
    // La etiqueta se aplica SIEMPRE a lo que está enfocado. Depender de que las etiquetas ya
    // se hubieran cargado hacía que una pulsación rápida tras entrar en el modo se perdiera
    // en silencio, que es la peor forma de fallar.
    const lib = useLibraryStore.getState();
    const id = filaEn(lib, lib.foco)?.id ?? get().etiquetas?.sampleId ?? null;
    if (id === null) return;
    try {
      await ipc.ponerEtiqueta(id, field, value);
      await get().cargarDe(id);
      void get().refrescarStats();
    } catch (e) {
      useUiStore
        .getState()
        .avisar("error", esAppError(e) ? e.message : "No se pudo guardar la etiqueta");
    }
  },

  /** Asigna el tipo y salta al siguiente de la muestra: una tecla, una decisión. */
  async ponerTipo(kind) {
    await get().poner("kind", kind);
    await get().siguienteDeLaCola();
  },

  async siguienteDeLaCola() {
    const { cola } = get();
    const lib = useLibraryStore.getState();
    const actual = get().etiquetas?.sampleId ?? null;
    const desde = actual === null ? -1 : cola.indexOf(actual);
    const siguiente = cola[desde + 1];

    if (siguiente === undefined) {
      // Se acabó la muestra: se sigue avanzando por la lista normal.
      lib.mover(1);
      return;
    }
    try {
      const pos = await ipc.posicionDe(consultaActual(lib, 0, 1), siguiente);
      if (pos !== null) {
        lib.irA(pos);
        return;
      }
    } catch (e) {
      log.warn("no se pudo localizar el siguiente de la muestra", e);
    }
    lib.mover(1);
  },

  async refrescarStats() {
    try {
      set({ stats: await ipc.estadisticasEtiquetas() });
    } catch (e) {
      log.warn("no se pudieron leer las estadísticas de etiquetas", e);
    }
  },

  async extraerDeNombres() {
    set({ extrayendo: true });
    try {
      const r = await ipc.extraerEtiquetas();
      useUiStore
        .getState()
        .avisar(
          "exito",
          `Leídos ${r.processed} nombres: ${r.kind} tipos, ${r.bpm} tempos y ${r.key} tonalidades en ${r.millis} ms`,
        );
      await get().refrescarStats();
    } catch (e) {
      useUiStore
        .getState()
        .avisar("error", esAppError(e) ? e.message : "No se pudo leer los nombres");
    } finally {
      set({ extrayendo: false });
    }
  },
}));
