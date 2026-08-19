/**
 * Estado del actualizador.
 *
 * La comprobación es silenciosa al arrancar —nadie abre esta app para leer avisos— y ruidosa
 * cuando la pides tú desde Ajustes, porque entonces sí esperas una respuesta.
 */
import { create } from "zustand";
import { useUiStore } from "../../app/uiStore";
import type { UpdateInfo } from "../../bindings";
import * as ipc from "../../lib/ipc";
import { log } from "../../lib/log";

/** Adonde se manda a quien no puede actualizarse desde la app (instalación por paquete). */
export const PAGINA_DESCARGAS =
  "https://github.com/AlexAlvarezAlmendros/SampleCurator/releases/latest";

export type EstadoActualizacion =
  | "reposo"
  | "buscando"
  | "disponible"
  | "descargando"
  | "instalando";

interface UpdaterState {
  info: UpdateInfo | null;
  estado: EstadoActualizacion;
  descargado: number;
  total: number;
  /** Se oculta el aviso al descartarlo; vuelve a aparecer en el siguiente arranque. */
  descartado: boolean;
  buscar: (silencioso?: boolean) => Promise<void>;
  instalar: () => Promise<void>;
  descartar: () => void;
}

export const useUpdaterStore = create<UpdaterState>((set, get) => ({
  info: null,
  estado: "reposo",
  descargado: 0,
  total: 0,
  descartado: false,

  async buscar(silencioso = false) {
    if (get().estado === "buscando") return;
    set({ estado: "buscando" });
    try {
      const info = await ipc.buscarActualizacion();
      set({ info, estado: info ? "disponible" : "reposo", descartado: false });
      if (!info && !silencioso) {
        useUiStore.getState().avisar("exito", "Ya tienes la última versión");
      }
    } catch (e) {
      set({ estado: "reposo" });
      // En desarrollo no hay endpoint que valga: fallar aquí es normal y no es noticia.
      if (silencioso) log.warn("no se pudo comprobar si hay versión nueva", e);
      else {
        useUiStore
          .getState()
          .avisar("error", ipc.esAppError(e) ? e.message : "No se pudo comprobar la versión");
      }
    }
  },

  async instalar() {
    const info = get().info;
    if (!info) return;

    // Instalada por paquete del sistema: la app no puede reemplazarse a sí misma sin dejar al
    // gestor de paquetes mintiendo, así que lleva a la descarga y que decida el usuario.
    if (!info.canInstall) {
      await ipc.abrirEnlace(PAGINA_DESCARGAS).catch((e) => log.warn("no se pudo abrir", e));
      return;
    }

    set({ estado: "descargando", descargado: 0, total: 0 });
    try {
      await ipc.instalarActualizacion((p) => {
        if (p.done) set({ estado: "instalando" });
        else set({ descargado: p.downloaded, total: p.total });
      });
      // Si llegamos aquí sin reinicio, la instalación terminó pero el reinicio no ocurrió.
      set({ estado: "instalando" });
    } catch (e) {
      set({ estado: "disponible" });
      useUiStore
        .getState()
        .avisar("error", ipc.esAppError(e) ? e.message : "No se pudo instalar la actualización");
    }
  },

  descartar() {
    set({ descartado: true });
  },
}));
