/**
 * Inspector de metadatos (Fase 9).
 *
 * Etiquetas, notas y valoración. Todo esto vive en el índice: los archivos de audio no se
 * tocan nunca. Escribir etiquetas dentro del `.wav` obligaría a reescribir los archivos del
 * usuario, y los DAW mayormente las ignoran en samples.
 */
import { create } from "zustand";
import { useUiStore } from "../../app/uiStore";
import type { SampleDetail } from "../../bindings";
import * as ipc from "../../lib/ipc";
import { esAppError } from "../../lib/ipc";
import { log } from "../../lib/log";
import { filaEn, useLibraryStore } from "../library/store";

interface MetaState {
  modo: boolean;
  detalle: SampleDetail | null;
  catalogo: Array<[string, number]>;

  alternarModo: () => Promise<void>;
  cargarDe: (sampleId: number) => Promise<void>;
  refrescarCatalogo: () => Promise<void>;
  anadirEtiqueta: (nombre: string) => Promise<void>;
  quitarEtiqueta: (nombre: string) => Promise<void>;
  guardarNotas: (texto: string) => Promise<void>;
  valorar: (estrellas: number) => Promise<void>;
}

function idEnFoco(): number | null {
  const lib = useLibraryStore.getState();
  return filaEn(lib, lib.foco)?.id ?? null;
}

export const useMetaStore = create<MetaState>((set, get) => ({
  modo: false,
  detalle: null,
  catalogo: [],

  async alternarModo() {
    const modo = !get().modo;
    set({ modo });
    if (!modo) return;
    await get().refrescarCatalogo();
    const id = idEnFoco();
    if (id !== null) await get().cargarDe(id);
  },

  async cargarDe(sampleId) {
    try {
      set({ detalle: await ipc.detalle(sampleId) });
    } catch (e) {
      log.warn("no se pudo leer el detalle", e);
      set({ detalle: null });
    }
  },

  async refrescarCatalogo() {
    try {
      set({ catalogo: await ipc.catalogoTags() });
    } catch (e) {
      log.warn("no se pudo leer el catálogo de etiquetas", e);
    }
  },

  async anadirEtiqueta(nombre) {
    const id = idEnFoco() ?? get().detalle?.row.id ?? null;
    if (id === null || nombre.trim() === "") return;
    try {
      await ipc.ponerTag(id, nombre);
      await get().cargarDe(id);
      void get().refrescarCatalogo();
    } catch (e) {
      useUiStore
        .getState()
        .avisar("error", esAppError(e) ? e.message : "No se pudo poner la etiqueta");
    }
  },

  async quitarEtiqueta(nombre) {
    const id = get().detalle?.row.id ?? null;
    if (id === null) return;
    try {
      await ipc.quitarTag(id, nombre);
      await get().cargarDe(id);
      void get().refrescarCatalogo();
    } catch (e) {
      log.warn("no se pudo quitar la etiqueta", e);
    }
  },

  async guardarNotas(texto) {
    const id = get().detalle?.row.id ?? null;
    if (id === null) return;
    try {
      await ipc.ponerNotas(id, texto);
      await get().cargarDe(id);
    } catch (e) {
      useUiStore
        .getState()
        .avisar("error", esAppError(e) ? e.message : "No se pudieron guardar las notas");
    }
  },

  /** La valoración se parchea también en la lista, para que la fila lo enseñe al momento. */
  async valorar(estrellas) {
    const id = idEnFoco() ?? get().detalle?.row.id ?? null;
    if (id === null) return;
    try {
      await ipc.valorar(id, estrellas);
      useLibraryStore.getState().parchear(id, { rating: estrellas });
      await get().cargarDe(id);
    } catch (e) {
      log.warn("no se pudo valorar", e);
    }
  },
}));
