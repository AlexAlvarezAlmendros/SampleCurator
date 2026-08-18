import { create } from "zustand";

export type TipoAviso = "info" | "error" | "exito";

export interface Aviso {
  tipo: TipoAviso;
  texto: string;
  /** Contador para que dos avisos iguales seguidos sigan notándose. */
  n: number;
}

export type Tema = "dark" | "light";

interface UiState {
  aviso: Aviso | null;
  ayudaAbierta: boolean;
  buscando: boolean;
  asistenteAbierto: boolean;
  renombrando: boolean;
  tema: Tema;
  avisar: (tipo: TipoAviso, texto: string) => void;
  limpiarAviso: () => void;
  alternarAyuda: () => void;
  setBuscando: (v: boolean) => void;
  setAsistente: (v: boolean) => void;
  setRenombrando: (v: boolean) => void;
  alternarTema: () => void;
  aplicarTema: (t: Tema) => void;
}

let contador = 0;

export const useUiStore = create<UiState>((set, get) => ({
  aviso: null,
  ayudaAbierta: false,
  buscando: false,
  asistenteAbierto: false,
  renombrando: false,
  tema: "dark",
  avisar: (tipo, texto) => set({ aviso: { tipo, texto, n: ++contador } }),
  limpiarAviso: () => set({ aviso: null }),
  alternarAyuda: () => set((s) => ({ ayudaAbierta: !s.ayudaAbierta })),
  setBuscando: (v) => set({ buscando: v }),
  setAsistente: (v) => set({ asistenteAbierto: v }),
  setRenombrando: (v) => set({ renombrando: v }),
  alternarTema: () => {
    const tema: Tema = get().tema === "dark" ? "light" : "dark";
    get().aplicarTema(tema);
  },
  // El tema solo toca la capa semántica de tokens: ningún componente sabe en cuál está.
  aplicarTema: (tema) => {
    document.documentElement.dataset.theme = tema;
    set({ tema });
  },
}));
