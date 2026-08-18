/**
 * La papelera, ahora visible.
 *
 * Existía desde la Fase 4 —lo rechazado va a `<destino>/.samplecurator-trash/` con su
 * manifiesto— pero era invisible: la única forma de recuperar algo era `Ctrl+Z` en el momento.
 * Aquí se puede escuchar antes de decidir y devolver de una en una.
 */
import { create } from "zustand";
import { useUiStore } from "../../app/uiStore";
import type { TrashEntry } from "../../bindings";
import * as ipc from "../../lib/ipc";
import { esAppError } from "../../lib/ipc";
import { log } from "../../lib/log";
import { useLibraryStore } from "../library/store";
import { usePlayerStore } from "../player/store";
import { useTriageStore } from "./store";

interface TrashState {
  abierta: boolean;
  entradas: TrashEntry[];
  cargando: boolean;
  abrir: () => Promise<void>;
  cerrar: () => void;
  refrescar: () => Promise<void>;
  restaurar: (trashPath: string) => Promise<void>;
  escuchar: (entrada: TrashEntry) => void;
}

export const useTrashStore = create<TrashState>((set, get) => ({
  abierta: false,
  entradas: [],
  cargando: false,

  async abrir() {
    set({ abierta: true });
    await get().refrescar();
  },

  cerrar() {
    set({ abierta: false });
  },

  async refrescar() {
    const proyecto = useTriageStore.getState().proyecto;
    if (!proyecto) return;
    set({ cargando: true });
    try {
      set({ entradas: await ipc.listarPapelera(proyecto.id) });
    } catch (e) {
      log.warn("no se pudo leer la papelera", e);
    } finally {
      set({ cargando: false });
    }
  },

  async restaurar(trashPath) {
    const proyecto = useTriageStore.getState().proyecto;
    if (!proyecto) return;
    try {
      await ipc.restaurarDePapelera(proyecto.id, trashPath);
      useUiStore.getState().avisar("exito", "Devuelto a su carpeta original");
      await get().refrescar();
      await useLibraryStore.getState().refrescar(true);
      void useTriageStore.getState().refrescarPapelera();
    } catch (e) {
      useUiStore.getState().avisar("error", esAppError(e) ? e.message : "No se pudo restaurar");
    }
  },

  /** Se puede escuchar lo que sigue en el índice; lo huérfano no tiene fila que reproducir. */
  escuchar(entrada) {
    if (entrada.sampleId === null) return;
    void usePlayerStore.getState().reproducir(entrada.sampleId);
  },
}));
