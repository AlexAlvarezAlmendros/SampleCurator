/**
 * Estado de la biblioteca: consulta, páginas cargadas, foco y selección.
 *
 * Las filas se guardan por páginas de 200 y se leen con `filaEn`. Cada fila se suscribe solo
 * a SU dato y a si tiene el foco, así que mover la selección repinta dos filas, no la lista.
 */
import { create } from "zustand";
import type {
  LibraryStats,
  SampleRow,
  ScanProgress,
  SortBy,
  SourceInfo,
  StatusFilter,
} from "../../bindings";
import * as ipc from "../../lib/ipc";
import { log } from "../../lib/log";

export const TAM_PAGINA = 200;

interface LibraryState {
  fuentes: SourceInfo[];
  fuenteActiva: number | null;
  busqueda: string;
  estado: StatusFilter;
  orden: SortBy;
  duracion: "todo" | "oneshots" | "loops";
  minValoracion: number;

  total: number;
  paginas: Map<number, SampleRow[]>;
  cargando: Set<number>;

  foco: number;
  ancla: number | null;
  seleccion: Set<number>;

  progreso: ScanProgress | null;
  stats: LibraryStats | null;

  cargarFuentes: () => Promise<void>;
  anadirCarpeta: () => Promise<void>;
  reescanearFuente: (id: number) => Promise<void>;
  quitarFuenteDelIndice: (id: number) => Promise<void>;
  escaneando: number | null;
  setFuente: (id: number | null) => Promise<void>;
  setBusqueda: (q: string) => Promise<void>;
  setEstado: (e: StatusFilter) => Promise<void>;
  setOrden: (o: SortBy) => Promise<void>;
  setDuracion: (d: "todo" | "oneshots" | "loops") => Promise<void>;
  setMinValoracion: (v: number) => Promise<void>;
  refrescar: (conservarFoco?: boolean) => Promise<void>;
  asegurarRango: (inicio: number, fin: number) => void;
  mover: (delta: number, extender?: boolean) => void;
  irA: (indice: number, extender?: boolean) => void;
  seleccionarTodo: () => void;
  limpiarSeleccion: () => void;
  parchear: (sampleId: number, cambios: Partial<SampleRow>) => void;
  setProgreso: (p: ScanProgress | null) => void;
  refrescarStats: () => Promise<void>;
}

/** Frontera one-shot / loop. Dos segundos separa bien un golpe de un bucle. */
const CORTE_LOOP_MS = 2000;

export function consultaActual(s: LibraryState, offset: number, limit: number): ipc.LibraryQuery {
  return {
    sourceId: s.fuenteActiva,
    search: s.busqueda.trim() === "" ? null : s.busqueda,
    status: s.estado,
    sort: s.orden,
    minDurationMs: s.duracion === "loops" ? CORTE_LOOP_MS : null,
    maxDurationMs: s.duracion === "oneshots" ? CORTE_LOOP_MS : null,
    minRating: s.minValoracion,
    offset,
    limit,
  };
}

/** Fila en un índice absoluto de la lista filtrada, o `undefined` si su página no está cargada. */
export function filaEn(s: LibraryState, indice: number): SampleRow | undefined {
  const pagina = s.paginas.get(Math.floor(indice / TAM_PAGINA));
  return pagina?.[indice % TAM_PAGINA];
}

/** Índices de los samples sobre los que actúa una decisión: la selección, o el foco. */
export function objetivo(s: LibraryState): number[] {
  if (s.seleccion.size > 0) return [...s.seleccion].sort((a, b) => a - b);
  return [s.foco];
}

export function idsDe(s: LibraryState, indices: number[]): number[] {
  const ids: number[] = [];
  for (const i of indices) {
    const f = filaEn(s, i);
    if (f) ids.push(f.id);
  }
  return ids;
}

export const useLibraryStore = create<LibraryState>((set, get) => ({
  fuentes: [],
  fuenteActiva: null,
  busqueda: "",
  estado: "all",
  orden: "path",
  duracion: "todo",
  minValoracion: 0,

  total: 0,
  paginas: new Map(),
  cargando: new Set(),

  foco: 0,
  ancla: null,
  seleccion: new Set(),

  progreso: null,
  stats: null,
  escaneando: null,

  /** Abre el diálogo nativo, añade la carpeta y deja la lista lista para usar. */
  async anadirCarpeta() {
    const ruta = await ipc.elegirCarpeta("Elige una carpeta con samples");
    if (!ruta) return;
    set({ escaneando: -1 });
    try {
      await ipc.anadirFuente(ruta, (p) => set({ progreso: p }));
      await get().cargarFuentes();
      await get().refrescar();
    } catch (e) {
      log.error("no se pudo añadir la carpeta", e);
    } finally {
      set({ escaneando: null });
    }
  },

  /**
   * Vuelve a recorrer una carpeta ya añadida: entra lo nuevo, se actualiza lo que cambió y
   * se poda lo que ya no está en disco. Lo que hayas movido con el triaje NO se toca.
   */
  async reescanearFuente(id) {
    set({ escaneando: id });
    try {
      await ipc.reescanear(id, (p) => set({ progreso: p }));
      await get().cargarFuentes();
      await get().refrescar(true);
    } catch (e) {
      log.error("no se pudo reescanear", e);
    } finally {
      set({ escaneando: null });
    }
  },

  /** Quita la carpeta del ÍNDICE. Los archivos del disco no se tocan. */
  async quitarFuenteDelIndice(id) {
    try {
      await ipc.quitarFuente(id);
      const quedaba = get().fuenteActiva === id;
      await get().cargarFuentes();
      if (quedaba) set({ fuenteActiva: get().fuentes[0]?.id ?? null });
      await get().refrescar();
    } catch (e) {
      log.error("no se pudo quitar la carpeta", e);
    }
  },

  async cargarFuentes() {
    try {
      const fuentes = await ipc.fuentes();
      const actual = get().fuenteActiva;
      const activa = actual ?? fuentes[0]?.id ?? null;
      set({ fuentes, fuenteActiva: activa });
    } catch (e) {
      log.error("no se pudieron cargar las fuentes", e);
    }
  },

  async setFuente(id) {
    set({ fuenteActiva: id });
    await get().refrescar();
  },

  async setBusqueda(q) {
    set({ busqueda: q });
    await get().refrescar();
  },

  async setEstado(e) {
    set({ estado: e });
    await get().refrescar();
  },

  async setOrden(o) {
    set({ orden: o });
    await get().refrescar();
  },

  async setDuracion(d) {
    set({ duracion: d });
    await get().refrescar();
  },

  async setMinValoracion(v) {
    set({ minValoracion: v });
    await get().refrescar();
  },

  async refrescar(conservarFoco = false) {
    const s = get();
    set({ paginas: new Map(), cargando: new Set(), seleccion: new Set(), ancla: null });
    try {
      const p = await ipc.pagina(consultaActual(s, 0, TAM_PAGINA));
      const paginas = new Map<number, SampleRow[]>([[0, p.rows]]);
      const foco = conservarFoco ? Math.min(s.foco, Math.max(0, p.total - 1)) : 0;
      set({ total: p.total, paginas, foco });
    } catch (e) {
      log.error("no se pudo cargar la lista", e);
      set({ total: 0 });
    }
    void get().refrescarStats();
  },

  asegurarRango(inicio, fin) {
    const s = get();
    const primera = Math.max(0, Math.floor(inicio / TAM_PAGINA));
    const ultima = Math.floor(Math.max(0, fin) / TAM_PAGINA);
    for (let p = primera; p <= ultima; p++) {
      if (s.paginas.has(p) || s.cargando.has(p)) continue;
      const cargando = new Set(get().cargando);
      cargando.add(p);
      set({ cargando });
      void ipc
        .pagina(consultaActual(get(), p * TAM_PAGINA, TAM_PAGINA))
        .then((res) => {
          const actual = get();
          const paginas = new Map(actual.paginas);
          paginas.set(p, res.rows);
          const pendientes = new Set(actual.cargando);
          pendientes.delete(p);
          set({ paginas, cargando: pendientes, total: res.total });
        })
        .catch((e) => {
          log.error(`no se pudo cargar la página ${p}`, e);
          const pendientes = new Set(get().cargando);
          pendientes.delete(p);
          set({ cargando: pendientes });
        });
    }
  },

  mover(delta, extender = false) {
    const s = get();
    if (s.total === 0) return;
    const destino = Math.min(Math.max(0, s.foco + delta), s.total - 1);
    get().irA(destino, extender);
  },

  irA(indice, extender = false) {
    const s = get();
    if (s.total === 0) return;
    const destino = Math.min(Math.max(0, indice), s.total - 1);
    if (extender) {
      const ancla = s.ancla ?? s.foco;
      const desde = Math.min(ancla, destino);
      const hasta = Math.max(ancla, destino);
      const seleccion = new Set<number>();
      for (let i = desde; i <= hasta; i++) seleccion.add(i);
      set({ foco: destino, ancla, seleccion });
    } else {
      set({ foco: destino, ancla: null, seleccion: new Set() });
    }
    get().asegurarRango(destino - 20, destino + 40);
  },

  seleccionarTodo() {
    const s = get();
    const seleccion = new Set<number>();
    for (let i = 0; i < s.total; i++) seleccion.add(i);
    set({ seleccion, ancla: 0 });
  },

  limpiarSeleccion() {
    set({ seleccion: new Set(), ancla: null });
  },

  /**
   * Actualiza una fila in situ tras una decisión, sin recargar la página.
   *
   * La fila decidida NO desaparece de la lista aunque el filtro sea "pendientes": si se fuera,
   * todos los índices se moverían bajo los dedos del usuario y deshacer no podría devolverle
   * el foco a donde estaba.
   */
  parchear(sampleId, cambios) {
    const s = get();
    for (const [numero, filas] of s.paginas) {
      const i = filas.findIndex((f) => f.id === sampleId);
      if (i === -1) continue;
      const anterior = filas[i];
      if (!anterior) continue;
      const nuevas = filas.slice();
      nuevas[i] = { ...anterior, ...cambios };
      const paginas = new Map(s.paginas);
      paginas.set(numero, nuevas);
      set({ paginas });
      return;
    }
  },

  setProgreso(p) {
    set({ progreso: p });
  },

  async refrescarStats() {
    try {
      set({ stats: await ipc.estadisticas(get().fuenteActiva) });
    } catch (e) {
      log.error("no se pudieron cargar las estadísticas", e);
    }
  },
}));
