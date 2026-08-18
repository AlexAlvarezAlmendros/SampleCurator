/**
 * Estado del triaje: proyecto, destinos, progreso y las decisiones.
 *
 * Todas las decisiones son optimistas: la fila se marca en el mismo frame de la pulsación y
 * el foco avanza sin esperar al disco. Si la operación falla, la fila vuelve a su estado y
 * el aviso sale en la barra inferior — para entonces el usuario ya va tres samples más abajo.
 */
import { create } from "zustand";
import { useUiStore } from "../../app/uiStore";
import type {
  Destination,
  Project,
  SessionProgress,
  TrashSummary,
  TriageMode,
} from "../../bindings";
import * as ipc from "../../lib/ipc";
import { esAppError } from "../../lib/ipc";
import { log } from "../../lib/log";
import { consultaActual, filaEn, idsDe, objetivo, useLibraryStore } from "../library/store";

interface TriageState {
  proyecto: Project | null;
  destinos: Destination[];
  progreso: SessionProgress | null;
  papelera: TrashSummary | null;

  cargar: () => Promise<void>;
  crearProyecto: (nombre: string, destRoot: string, modo: TriageMode) => Promise<void>;
  cambiarModo: (modo: TriageMode) => Promise<void>;
  crearDestino: (nombre: string, relPath?: string) => Promise<void>;
  borrarDestino: (id: number) => Promise<void>;
  refrescarDestinos: () => Promise<void>;
  enviarATecla: (tecla: string) => Promise<void>;
  enviarA: (destId: number) => Promise<void>;
  rechazar: () => Promise<void>;
  conservar: () => Promise<void>;
  deshacer: () => Promise<void>;
  rehacer: () => Promise<void>;
  refrescarProgreso: () => Promise<void>;
  refrescarPapelera: () => Promise<void>;
  vaciarPapelera: () => Promise<void>;
}

function avisarError(e: unknown, contexto: string): void {
  const texto = esAppError(e) ? e.message : `${contexto}: ${String(e)}`;
  useUiStore.getState().avisar("error", texto);
  log.warn(contexto, e);
}

/** Índice al que salta el foco después de decidir sobre `indices`. */
function siguienteFoco(indices: number[]): number {
  const ultimo = indices[indices.length - 1] ?? 0;
  return ultimo + 1;
}

export const useTriageStore = create<TriageState>((set, get) => ({
  proyecto: null,
  destinos: [],
  progreso: null,
  papelera: null,

  async cargar() {
    try {
      const proyecto = await ipc.ultimoProyecto();
      set({ proyecto });
      if (proyecto) {
        await ipc.abrirProyecto(proyecto.id);
        await get().refrescarDestinos();
        await get().refrescarPapelera();
      }
      await get().refrescarProgreso();
    } catch (e) {
      avisarError(e, "no se pudo cargar la sesión");
    }
  },

  async crearProyecto(nombre, destRoot, modo) {
    try {
      const proyecto = await ipc.crearProyecto(nombre, destRoot, modo);
      set({ proyecto, destinos: [] });
      await get().refrescarDestinos();
      await get().refrescarPapelera();
    } catch (e) {
      avisarError(e, "no se pudo crear la sesión");
    }
  },

  async cambiarModo(modo) {
    const p = get().proyecto;
    if (!p) return;
    try {
      await ipc.cambiarModo(p.id, modo);
      set({ proyecto: { ...p, mode: modo } });
    } catch (e) {
      avisarError(e, "no se pudo cambiar el modo");
    }
  },

  async crearDestino(nombre, relPath) {
    const p = get().proyecto;
    if (!p) return;
    try {
      await ipc.crearDestino(p.id, nombre, relPath ?? nombre);
      await get().refrescarDestinos();
    } catch (e) {
      avisarError(e, "no se pudo crear el destino");
    }
  },

  async borrarDestino(id) {
    try {
      await ipc.borrarDestino(id);
      await get().refrescarDestinos();
    } catch (e) {
      avisarError(e, "no se pudo borrar el destino");
    }
  },

  async refrescarDestinos() {
    const p = get().proyecto;
    if (!p) return;
    try {
      set({ destinos: await ipc.destinos(p.id) });
    } catch (e) {
      avisarError(e, "no se pudieron cargar los destinos");
    }
  },

  async enviarATecla(tecla) {
    const destino = get().destinos.find((d) => d.hotkey === tecla);
    if (!destino) {
      useUiStore.getState().avisar("info", `No hay ningún destino en la tecla ${tecla}`);
      return;
    }
    await get().enviarA(destino.id);
  },

  async enviarA(destId) {
    const p = get().proyecto;
    if (!p) {
      useUiStore.getState().avisar("info", "Primero elige una carpeta de destino (tecla D)");
      return;
    }
    const lib = useLibraryStore.getState();
    const indices = objetivo(lib);
    const ids = idsDe(lib, indices);
    if (ids.length === 0) return;

    const destino = get().destinos.find((d) => d.id === destId);
    const previos = ids.map((id) => {
      const fila = indices.map((i) => filaEn(lib, i)).find((f) => f?.id === id);
      return { id, status: fila?.status ?? "pending", destination: fila?.destination ?? null };
    });

    // optimista: la fila se marca ya, en el frame de la pulsación
    for (const id of ids) {
      lib.parchear(id, {
        status: p.mode === "copy" ? "kept" : "moved",
        destination: destino?.name ?? null,
      });
    }
    lib.limpiarSeleccion();
    useLibraryStore.getState().irA(siguienteFoco(indices));

    try {
      const r = await ipc.enviar(p.id, destId, ids);
      if (r.destinationCount !== null && r.destinationId !== null) {
        set({
          destinos: get().destinos.map((d) =>
            d.id === r.destinationId ? { ...d, count: r.destinationCount ?? d.count } : d,
          ),
        });
      }
      if (r.affected.length < ids.length) {
        useUiStore
          .getState()
          .avisar("error", `${ids.length - r.affected.length} samples no se pudieron mover`);
      }
      void get().refrescarProgreso();
    } catch (e) {
      for (const previo of previos) {
        lib.parchear(previo.id, { status: previo.status, destination: previo.destination });
      }
      avisarError(e, "no se pudo enviar");
    }
  },

  async rechazar() {
    const p = get().proyecto;
    if (!p) {
      useUiStore.getState().avisar("info", "Primero elige una carpeta de destino (tecla D)");
      return;
    }
    const lib = useLibraryStore.getState();
    const indices = objetivo(lib);
    const ids = idsDe(lib, indices);
    if (ids.length === 0) return;

    for (const id of ids) lib.parchear(id, { status: "rejected", destination: null });
    lib.limpiarSeleccion();
    useLibraryStore.getState().irA(siguienteFoco(indices));

    try {
      await ipc.rechazar(p.id, ids);
      void get().refrescarProgreso();
      void get().refrescarPapelera();
    } catch (e) {
      for (const id of ids) lib.parchear(id, { status: "pending" });
      avisarError(e, "no se pudo rechazar");
    }
  },

  async conservar() {
    const p = get().proyecto;
    if (!p) return;
    const lib = useLibraryStore.getState();
    const indices = objetivo(lib);
    const ids = idsDe(lib, indices);
    if (ids.length === 0) return;

    for (const id of ids) lib.parchear(id, { status: "kept" });
    lib.limpiarSeleccion();
    useLibraryStore.getState().irA(siguienteFoco(indices));

    try {
      await ipc.conservar(p.id, ids);
      void get().refrescarProgreso();
    } catch (e) {
      for (const id of ids) lib.parchear(id, { status: "pending" });
      avisarError(e, "no se pudo conservar");
    }
  },

  /**
   * Deshacer devuelve el archivo, el estado, el contador del destino Y el foco al sample
   * afectado. Un undo que no te devuelve a donde estabas no sirve de nada.
   */
  async deshacer() {
    try {
      const r = await ipc.deshacer();
      await useLibraryStore.getState().refrescar(true);
      await get().refrescarDestinos();
      void get().refrescarProgreso();
      void get().refrescarPapelera();

      if (r.focusSampleId !== null) {
        const lib = useLibraryStore.getState();
        const pos = await ipc.posicionDe(consultaActual(lib, 0, 1), r.focusSampleId);
        if (pos !== null) useLibraryStore.getState().irA(pos);
      }
      useUiStore.getState().avisar("info", `Deshecho: ${r.restored.length} sample(s)`);
    } catch (e) {
      if (esAppError(e) && e.kind === "nothing_to_undo") {
        useUiStore.getState().avisar("info", "No hay nada que deshacer");
        return;
      }
      avisarError(e, "no se pudo deshacer");
    }
  },

  async rehacer() {
    try {
      const r = await ipc.rehacer();
      await useLibraryStore.getState().refrescar(true);
      await get().refrescarDestinos();
      void get().refrescarProgreso();
      useUiStore.getState().avisar("info", `Rehecho: ${r.restored.length} sample(s)`);
    } catch (e) {
      if (esAppError(e) && e.kind === "nothing_to_undo") {
        useUiStore.getState().avisar("info", "No hay nada que rehacer");
        return;
      }
      avisarError(e, "no se pudo rehacer");
    }
  },

  async refrescarProgreso() {
    try {
      set({ progreso: await ipc.progresoSesion(useLibraryStore.getState().fuenteActiva) });
    } catch (e) {
      log.warn("no se pudo cargar el progreso", e);
    }
  },

  async refrescarPapelera() {
    const p = get().proyecto;
    if (!p) return;
    try {
      set({ papelera: await ipc.resumenPapelera(p.id) });
    } catch (e) {
      log.warn("no se pudo leer la papelera", e);
    }
  },

  async vaciarPapelera() {
    const p = get().proyecto;
    if (!p) return;
    try {
      const n = await ipc.vaciarPapelera(p.id);
      useUiStore.getState().avisar("exito", `Papelera vaciada: ${n} archivos`);
      await get().refrescarPapelera();
    } catch (e) {
      avisarError(e, "no se pudo vaciar la papelera");
    }
  },
}));
