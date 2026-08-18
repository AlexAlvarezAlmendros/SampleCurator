import { create } from "zustand";

export type TipoAviso = "info" | "error" | "exito";

export interface Aviso {
  tipo: TipoAviso;
  texto: string;
  /** Contador para que dos avisos iguales seguidos sigan notándose. */
  n: number;
}

export type Tema = "dark" | "light";
export type Densidad = "compacta" | "normal" | "comoda";

/**
 * Alto de fila en píxeles. El token CSS define el valor por defecto; esto es la preferencia
 * del usuario, que manda sobre él. Vive en JS porque el virtualizador necesita el número
 * exacto: si lo leyera del CSS y el CSS cambiara, la lista mediría mal.
 */
export const ALTURA_FILA: Record<Densidad, number> = {
  compacta: 24,
  normal: 28,
  comoda: 34,
};

interface UiState {
  aviso: Aviso | null;
  ayudaAbierta: boolean;
  buscando: boolean;
  asistenteAbierto: boolean;
  renombrando: boolean;
  tema: Tema;
  densidad: Densidad;
  ajustesAbiertos: boolean;
  avisar: (tipo: TipoAviso, texto: string) => void;
  limpiarAviso: () => void;
  alternarAyuda: () => void;
  setBuscando: (v: boolean) => void;
  setAsistente: (v: boolean) => void;
  setRenombrando: (v: boolean) => void;
  alternarTema: () => void;
  aplicarTema: (t: Tema) => void;
  aplicarDensidad: (d: Densidad) => void;
  setAjustes: (v: boolean) => void;
}

let contador = 0;

export const useUiStore = create<UiState>((set, get) => ({
  aviso: null,
  ayudaAbierta: false,
  buscando: false,
  asistenteAbierto: false,
  renombrando: false,
  tema: "dark",
  densidad: "normal",
  ajustesAbiertos: false,
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
  aplicarDensidad: (densidad) => {
    // El token CSS define el valor por defecto; esta es la preferencia del usuario, que manda.
    document.documentElement.style.setProperty("--row-height", `${ALTURA_FILA[densidad]}px`);
    set({ densidad });
  },
  setAjustes: (v) => set({ ajustesAbiertos: v }),
}));
