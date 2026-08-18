import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SampleRow } from "../../bindings";

vi.mock("../../lib/ipc", async () => {
  const real = await vi.importActual<typeof import("../../lib/ipc")>("../../lib/ipc");
  return {
    ...real,
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
    enviar: vi.fn(),
    rechazar: vi.fn(),
    progresoSesion: vi.fn(async () => ({ done: 0, total: 0 })),
    destinos: vi.fn(async () => []),
    resumenPapelera: vi.fn(async () => ({ files: 0, bytes: 0 })),
  };
});

import * as ipc from "../../lib/ipc";
import { filaEn, useLibraryStore } from "../library/store";
import { useTriageStore } from "./store";

function fila(id: number): SampleRow {
  return {
    id,
    filename: `s${id}.wav`,
    relPath: `p/s${id}.wav`,
    ext: "wav",
    size: 10,
    durationMs: 100,
    sampleRate: 44100,
    channels: 1,
    analyzed: true,
    status: "pending",
    rating: 0,
    duplicate: false,
    destination: null,
  };
}

const DESTINO = {
  id: 7,
  projectId: 1,
  name: "Kicks",
  relPath: "Kicks",
  hotkey: "1",
  color: "dest-1",
  sortOrder: 0,
  count: 0,
};

describe("decisiones de triaje", () => {
  beforeEach(async () => {
    // Sin esto, `mock.calls[0]` sigue siendo la llamada del test anterior.
    vi.clearAllMocks();
    vi.mocked(ipc.pagina).mockResolvedValue({
      rows: Array.from({ length: 10 }, (_, i) => fila(i)),
      total: 10,
      offset: 0,
    });
    useLibraryStore.setState({
      total: 0,
      paginas: new Map(),
      cargando: new Set(),
      foco: 0,
      ancla: null,
      seleccion: new Set(),
      fuenteActiva: 1,
      estado: "all",
      orden: "path",
      busqueda: "",
    });
    await useLibraryStore.getState().refrescar();
    useTriageStore.setState({
      proyecto: {
        id: 1,
        name: "s",
        destRoot: "/tmp/dest",
        mode: "move",
        createdAt: 0,
      },
      destinos: [DESTINO],
      progreso: null,
      papelera: null,
    });
  });

  it("marca la fila y avanza el foco antes de que responda el disco", async () => {
    let resolver: (v: unknown) => void = () => {};
    vi.mocked(ipc.enviar).mockReturnValue(
      new Promise((r) => {
        resolver = r as (v: unknown) => void;
      }) as ReturnType<typeof ipc.enviar>,
    );

    useLibraryStore.getState().irA(2);
    const promesa = useTriageStore.getState().enviarATecla("1");

    // sin esperar a la respuesta, la interfaz ya ha reaccionado
    expect(filaEn(useLibraryStore.getState(), 2)?.status).toBe("moved");
    expect(filaEn(useLibraryStore.getState(), 2)?.destination).toBe("Kicks");
    expect(useLibraryStore.getState().foco).toBe(3);

    resolver({
      batchId: "b1",
      affected: [2],
      destinationId: 7,
      destinationCount: 1,
      kind: "move",
    });
    await promesa;
    expect(useTriageStore.getState().destinos[0]?.count).toBe(1);
  });

  it("si el disco falla, la fila vuelve a su estado anterior", async () => {
    vi.mocked(ipc.enviar).mockRejectedValue({ kind: "io", message: "disco lleno" });
    useLibraryStore.getState().irA(4);
    await useTriageStore.getState().enviarATecla("1");

    const f = filaEn(useLibraryStore.getState(), 4);
    expect(f?.status).toBe("pending");
    expect(f?.destination).toBeNull();
  });

  it("una tecla sin destino asignado no hace nada", async () => {
    useLibraryStore.getState().irA(1);
    await useTriageStore.getState().enviarATecla("9");
    expect(ipc.enviar).not.toHaveBeenCalled();
    expect(filaEn(useLibraryStore.getState(), 1)?.status).toBe("pending");
  });

  it("con selección múltiple decide sobre todas y avanza tras la última", async () => {
    vi.mocked(ipc.enviar).mockResolvedValue({
      batchId: "b",
      affected: [2, 3, 4],
      destinationId: 7,
      destinationCount: 3,
      kind: "move",
    });
    useLibraryStore.getState().irA(2);
    useLibraryStore.getState().mover(2, true); // selecciona 2,3,4
    await useTriageStore.getState().enviarATecla("1");

    expect(vi.mocked(ipc.enviar).mock.calls[0]?.[2]).toEqual([2, 3, 4]);
    expect(useLibraryStore.getState().foco).toBe(5);
    expect(useLibraryStore.getState().seleccion.size).toBe(0);
  });
});
