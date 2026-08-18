import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Este test monta la aplicación entera con el núcleo Rust simulado. No comprueba píxeles:
 * comprueba que el árbol se construye sin romperse, que el arranque encadena las llamadas
 * correctas y que la interfaz reacciona a lo que devuelven.
 */
vi.mock("../lib/ipc", () => ({
  appInfo: vi.fn(async () => ({
    version: "0.1.0",
    dbPath: "/tmp/x.db",
    audio: null,
    audioError: null,
  })),
  fuentes: vi.fn(async () => [
    { id: 1, path: "/musica/samples", addedAt: 0, total: 3, analyzed: 3 },
  ]),
  pagina: vi.fn(async () => ({
    rows: [
      {
        id: 10,
        filename: "KICK_808.wav",
        relPath: "kicks/KICK_808.wav",
        ext: "wav",
        size: 1000,
        durationMs: 420,
        sampleRate: 44100,
        channels: 1,
        analyzed: true,
        status: "pending",
        rating: 0,
        duplicate: false,
        destination: null,
      },
    ],
    total: 1,
    offset: 0,
  })),
  estadisticas: vi.fn(async () => ({
    total: 3,
    pending: 3,
    kept: 0,
    rejected: 0,
    moved: 0,
    analyzed: 3,
    duplicates: 0,
  })),
  ultimoProyecto: vi.fn(async () => ({
    id: 1,
    name: "sesión",
    destRoot: "/musica/libreria",
    mode: "move",
    createdAt: 0,
  })),
  abrirProyecto: vi.fn(async () => ({})),
  destinos: vi.fn(async () => [
    {
      id: 5,
      projectId: 1,
      name: "Kicks",
      relPath: "Kicks",
      hotkey: "1",
      color: "dest-1",
      sortOrder: 0,
      count: 12,
    },
  ]),
  resumenPapelera: vi.fn(async () => ({ files: 0, bytes: 0 })),
  progresoSesion: vi.fn(async () => ({ done: 1, total: 3 })),
  ultimaPosicion: vi.fn(async () => null),
  posicionDe: vi.fn(async () => null),
  analisisPendiente: vi.fn(async () => 0),
  alSoltarCarpetas: vi.fn(async () => () => {}),
  reproducir: vi.fn(async () => ({
    sampleId: 10,
    startedAtMs: 0,
    durationMs: 420,
    startOffsetMs: 0,
    looping: false,
  })),
  picos: vi.fn(async () => new Int8Array(2000)),
  prefetch: vi.fn(async () => {}),
  recordarPosicion: vi.fn(async () => {}),
  esAppError: (e: unknown) => typeof e === "object" && e !== null && "kind" in e,
  // El resto del contrato, en silencio: si un componente llama a algo que no está aquí,
  // el mock falla en voz alta y nos enteramos.
  settingsGet: vi.fn(async () => null),
  settingsSet: vi.fn(async () => {}),
  renombrar: vi.fn(async (_id: number, n: string) => n),
  exportarDecisiones: vi.fn(async () => "/musica/libreria/library.json"),
  destinosSugeridos: vi.fn(async () => ["Kicks", "Snares"]),
  elegirCarpeta: vi.fn(async () => null),
  anadirFuente: vi.fn(async () => ({ id: 2, path: "/x", addedAt: 0, total: 0, analyzed: 0 })),
  crearProyecto: vi.fn(async () => ({})),
  crearDestino: vi.fn(async () => ({})),
  borrarDestino: vi.fn(async () => {}),
  cambiarModo: vi.fn(async () => {}),
  valorar: vi.fn(async () => {}),
  enviar: vi.fn(async () => ({})),
  rechazar: vi.fn(async () => ({})),
  conservar: vi.fn(async () => ({})),
  deshacer: vi.fn(async () => ({})),
  rehacer: vi.fn(async () => ({})),
  vaciarPapelera: vi.fn(async () => 0),
  revelarEnElExplorador: vi.fn(async () => {}),
  detalle: vi.fn(async () => ({})),
  parar: vi.fn(async () => {}),
  buscarEn: vi.fn(async () => {}),
  ganancia: vi.fn(async () => {}),
  bucle: vi.fn(async () => {}),
}));

import { useLibraryStore } from "../features/library/store";
import * as ipc from "../lib/ipc";
import { App } from "./App";

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useLibraryStore.setState({
      fuentes: [],
      total: 0,
      paginas: new Map(),
      cargando: new Set(),
      foco: 0,
      seleccion: new Set(),
      ancla: null,
    });
  });

  it("se monta y pinta las tres zonas y el transporte", async () => {
    render(<App />);
    expect(screen.getByText("Carpetas")).toBeDefined();
    expect(screen.getByText("Filtro")).toBeDefined();
    expect(screen.getByText("Destinos")).toBeDefined();
    expect(screen.getByText("revisados")).toBeDefined();
  });

  it("al arrancar carga fuentes, lista, sesión y destinos", async () => {
    render(<App />);
    await waitFor(() => expect(ipc.fuentes).toHaveBeenCalled());
    await waitFor(() => expect(ipc.pagina).toHaveBeenCalled());
    await waitFor(() => expect(ipc.ultimoProyecto).toHaveBeenCalled());
    await waitFor(() => expect(ipc.destinos).toHaveBeenCalled());
  });

  it("muestra el cubo de destino con su tecla y su contador", async () => {
    render(<App />);
    await waitFor(() => expect(screen.getByText("Kicks")).toBeDefined());
    expect(screen.getByText("1")).toBeDefined();
    expect(screen.getByText("12")).toBeDefined();
  });

  it("avisa cuando el motor de audio no ha podido arrancar", async () => {
    vi.mocked(ipc.appInfo).mockResolvedValueOnce({
      version: "0.1.0",
      dbPath: "/tmp/x.db",
      audio: null,
      audioError: "no hay dispositivo de salida",
    });
    render(<App />);
    await waitFor(() =>
      expect(screen.getByText(/Sin audio: no hay dispositivo de salida/)).toBeDefined(),
    );
  });

  it("F2 abre el renombrado en la propia barra de transporte, sin modal", async () => {
    render(<App />);
    await waitFor(() => expect(screen.getByText("KICK_808.wav")).toBeDefined());

    fireEvent.keyDown(window, { key: "F2" });
    const campo = (await screen.findByLabelText("Nuevo nombre del archivo")) as HTMLInputElement;
    expect(campo.value).toBe("KICK_808.wav");

    fireEvent.change(campo, { target: { value: "KICK_grave.wav" } });
    fireEvent.keyDown(campo, { key: "Enter" });
    await waitFor(() => expect(ipc.renombrar).toHaveBeenCalledWith(10, "KICK_grave.wav", 1));
  });

  it("la tecla T cambia de tema tocando solo el atributo del documento", async () => {
    render(<App />);
    await waitFor(() => expect(ipc.fuentes).toHaveBeenCalled());
    fireEvent.keyDown(window, { key: "t" });
    expect(document.documentElement.dataset.theme).toBe("light");
    fireEvent.keyDown(window, { key: "t" });
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(ipc.settingsSet).toHaveBeenCalledWith("tema", "dark");
  });

  it("con la biblioteca vacía abre el asistente en vez de dejar una pantalla muerta", async () => {
    vi.mocked(ipc.fuentes).mockResolvedValueOnce([]);
    render(<App />);
    await waitFor(() => expect(screen.getByText("Preparar la sesión")).toBeDefined());
  });
});
