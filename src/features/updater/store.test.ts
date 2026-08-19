import { beforeEach, describe, expect, it, vi } from "vitest";
import { useUiStore } from "../../app/uiStore";
import type { UpdateInfo } from "../../bindings";

vi.mock("../../lib/ipc", async () => {
  const real = await vi.importActual<typeof import("../../lib/ipc")>("../../lib/ipc");
  return {
    ...real,
    buscarActualizacion: vi.fn(async () => null),
    instalarActualizacion: vi.fn(async () => {}),
    abrirEnlace: vi.fn(async () => {}),
  };
});

import * as ipc from "../../lib/ipc";
import { PAGINA_DESCARGAS, useUpdaterStore } from "./store";

const NUEVA: UpdateInfo = {
  version: "0.3.0",
  currentVersion: "0.2.2",
  notes: "Cosas nuevas",
  canInstall: true,
};

describe("actualizador", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useUpdaterStore.setState({
      info: null,
      estado: "reposo",
      descargado: 0,
      total: 0,
      descartado: false,
    });
    useUiStore.getState().limpiarAviso();
    vi.mocked(ipc.buscarActualizacion).mockResolvedValue(null);
    vi.mocked(ipc.instalarActualizacion).mockResolvedValue(undefined);
    vi.mocked(ipc.abrirEnlace).mockResolvedValue(undefined);
  });

  it("sin versión nueva no molesta a nadie cuando la comprobación es silenciosa", async () => {
    await useUpdaterStore.getState().buscar(true);
    expect(useUpdaterStore.getState().info).toBeNull();
    expect(useUiStore.getState().aviso).toBeNull();
  });

  it("sin versión nueva sí contesta cuando la pides tú", async () => {
    await useUpdaterStore.getState().buscar();
    expect(useUiStore.getState().aviso?.texto).toContain("última versión");
  });

  it("un fallo al comprobar en silencio no se le echa en cara al usuario", async () => {
    // Pasa en desarrollo, donde no hay endpoint: avisar aquí sería ruido puro.
    vi.mocked(ipc.buscarActualizacion).mockRejectedValue({ kind: "update", message: "sin red" });
    await useUpdaterStore.getState().buscar(true);
    expect(useUiStore.getState().aviso).toBeNull();
    expect(useUpdaterStore.getState().estado).toBe("reposo");
  });

  it("con versión nueva la guarda y la deja lista para instalar", async () => {
    vi.mocked(ipc.buscarActualizacion).mockResolvedValue(NUEVA);
    await useUpdaterStore.getState().buscar(true);
    expect(useUpdaterStore.getState().info?.version).toBe("0.3.0");
    expect(useUpdaterStore.getState().estado).toBe("disponible");
  });

  it("instalar informa del progreso de la descarga", async () => {
    vi.mocked(ipc.buscarActualizacion).mockResolvedValue(NUEVA);
    vi.mocked(ipc.instalarActualizacion).mockImplementation(async (alProgresar) => {
      alProgresar({ downloaded: 500, total: 1000, done: false });
    });
    await useUpdaterStore.getState().buscar(true);
    await useUpdaterStore.getState().instalar();
    expect(useUpdaterStore.getState().descargado).toBe(500);
    expect(useUpdaterStore.getState().total).toBe(1000);
  });

  it("si la instalación falla lo dice y deja volver a intentarlo", async () => {
    vi.mocked(ipc.buscarActualizacion).mockResolvedValue(NUEVA);
    vi.mocked(ipc.instalarActualizacion).mockRejectedValue({
      kind: "update",
      message: "no se pudo instalar la actualización: firma inválida",
    });
    await useUpdaterStore.getState().buscar(true);
    await useUpdaterStore.getState().instalar();
    expect(useUiStore.getState().aviso?.tipo).toBe("error");
    expect(useUpdaterStore.getState().estado).toBe("disponible");
  });

  it("instalada por paquete del sistema, lleva a la descarga en vez de instalar", async () => {
    // Reemplazar los archivos de un .deb a mano dejaría al gestor de paquetes mintiendo.
    vi.mocked(ipc.buscarActualizacion).mockResolvedValue({ ...NUEVA, canInstall: false });
    await useUpdaterStore.getState().buscar(true);
    await useUpdaterStore.getState().instalar();
    expect(ipc.instalarActualizacion).not.toHaveBeenCalled();
    expect(ipc.abrirEnlace).toHaveBeenCalledWith(PAGINA_DESCARGAS);
  });
});
