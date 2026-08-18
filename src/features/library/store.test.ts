import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SampleRow } from "../../bindings";

vi.mock("../../lib/ipc", () => ({
  pagina: vi.fn(),
  estadisticas: vi.fn(async () => ({
    total: 0,
    pending: 0,
    kept: 0,
    rejected: 0,
    moved: 0,
    analyzed: 0,
    duplicates: 0,
  })),
  fuentes: vi.fn(async () => []),
}));

import * as ipc from "../../lib/ipc";
import { TAM_PAGINA, filaEn, idsDe, objetivo, useLibraryStore } from "./store";

function fila(id: number): SampleRow {
  return {
    id,
    filename: `s${id}.wav`,
    relPath: `pack/s${id}.wav`,
    ext: "wav",
    size: 1000,
    durationMs: 500,
    sampleRate: 44100,
    channels: 1,
    analyzed: true,
    status: "pending",
    rating: 0,
    duplicate: false,
    destination: null,
  };
}

function paginaDe(desde: number, cuantas: number, total: number) {
  return {
    rows: Array.from({ length: cuantas }, (_, i) => fila(desde + i)),
    total,
    offset: desde,
  };
}

describe("store de la biblioteca", () => {
  beforeEach(() => {
    vi.mocked(ipc.pagina).mockReset();
    useLibraryStore.setState({
      total: 0,
      paginas: new Map(),
      cargando: new Set(),
      foco: 0,
      ancla: null,
      seleccion: new Set(),
      busqueda: "",
      estado: "all",
      orden: "path",
      fuenteActiva: 1,
    });
  });

  it("carga la primera página y fija el total", async () => {
    vi.mocked(ipc.pagina).mockResolvedValue(paginaDe(0, TAM_PAGINA, 5000));
    await useLibraryStore.getState().refrescar();
    const s = useLibraryStore.getState();
    expect(s.total).toBe(5000);
    expect(filaEn(s, 0)?.id).toBe(0);
    expect(filaEn(s, 199)?.id).toBe(199);
    expect(filaEn(s, 200)).toBeUndefined(); // esa página aún no está cargada
  });

  it("el foco nunca se sale de la lista", async () => {
    vi.mocked(ipc.pagina).mockResolvedValue(paginaDe(0, 10, 10));
    await useLibraryStore.getState().refrescar();
    useLibraryStore.getState().mover(-5);
    expect(useLibraryStore.getState().foco).toBe(0);
    useLibraryStore.getState().mover(999);
    expect(useLibraryStore.getState().foco).toBe(9);
  });

  it("extender con shift selecciona el rango entre el ancla y el destino", async () => {
    vi.mocked(ipc.pagina).mockResolvedValue(paginaDe(0, 20, 20));
    await useLibraryStore.getState().refrescar();
    useLibraryStore.getState().irA(5);
    useLibraryStore.getState().mover(3, true);
    const s = useLibraryStore.getState();
    expect(s.foco).toBe(8);
    expect([...s.seleccion].sort((a, b) => a - b)).toEqual([5, 6, 7, 8]);
    expect(idsDe(s, objetivo(s))).toEqual([5, 6, 7, 8]);
  });

  it("sin selección, la decisión actúa solo sobre el foco", async () => {
    vi.mocked(ipc.pagina).mockResolvedValue(paginaDe(0, 20, 20));
    await useLibraryStore.getState().refrescar();
    useLibraryStore.getState().irA(7);
    const s = useLibraryStore.getState();
    expect(objetivo(s)).toEqual([7]);
  });

  it("parchear cambia una fila y NO toca la identidad de las demás", async () => {
    vi.mocked(ipc.pagina).mockResolvedValue(paginaDe(0, 20, 20));
    await useLibraryStore.getState().refrescar();
    const antes = filaEn(useLibraryStore.getState(), 3);

    useLibraryStore.getState().parchear(5, { status: "moved", destination: "Kicks" });

    const s = useLibraryStore.getState();
    expect(filaEn(s, 5)?.status).toBe("moved");
    expect(filaEn(s, 5)?.destination).toBe("Kicks");
    // la identidad de las vecinas se conserva: si cambiara, `memo` no serviría de nada y
    // se repintarían las treinta filas visibles en cada decisión
    expect(filaEn(s, 3)).toBe(antes);
  });

  it("la fila decidida sigue en la lista aunque el filtro sea 'pendientes'", async () => {
    vi.mocked(ipc.pagina).mockResolvedValue(paginaDe(0, 20, 20));
    useLibraryStore.setState({ estado: "pending" });
    await useLibraryStore.getState().refrescar();
    useLibraryStore.getState().parchear(4, { status: "rejected" });
    const s = useLibraryStore.getState();
    expect(s.total).toBe(20);
    expect(filaEn(s, 4)?.status).toBe("rejected");
  });
});
