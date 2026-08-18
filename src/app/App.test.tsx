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
  reescanear: vi.fn(async () => {}),
  quitarFuente: vi.fn(async () => {}),
  infoAudio: vi.fn(async () => ({
    sampleRate: 44100,
    channels: 2,
    bufferFrames: 256,
    bufferFixed: true,
    cacheBytes: 1048576,
    cacheLimitBytes: 268435456,
    cacheEntries: 4,
    latencyP50Ms: 1.4,
    latencyP95Ms: 2.6,
    shots: 120,
  })),
  extraerEtiquetas: vi.fn(async () => ({
    processed: 3,
    kind: 2,
    bpm: 1,
    key: 1,
    pitch: 0,
    millis: 12,
  })),
  estadisticasEtiquetas: vi.fn(async () => ({
    fields: [
      {
        field: "kind",
        fromFilename: 3,
        fromUser: 1,
        onlyUser: 0,
        pairs: 1,
        exact: 1,
        close: 0,
        wrong: 0,
        accuracy: 1,
        mirex: 1,
      },
    ],
    labeledSamples: 1,
    target: 200,
  })),
  etiquetasDe: vi.fn(async (id: number) => ({
    sampleId: id,
    kind: null,
    kindSource: null,
    bpm: null,
    bpmSource: null,
    key: null,
    keySource: null,
  })),
  ponerEtiqueta: vi.fn(async () => {}),
  quitarEtiqueta: vi.fn(async () => {}),
  muestraParaEtiquetar: vi.fn(async () => [10]),
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

import { useLabelsStore } from "../features/labels/store";
import { useLibraryStore } from "../features/library/store";
import * as ipc from "../lib/ipc";
import { App } from "./App";
import { useUiStore } from "./uiStore";

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Los stores de zustand viven en el módulo: sin reiniciarlos, un test hereda el modo
    // que dejó encendido el anterior.
    useLabelsStore.setState({ modo: false, etiquetas: null, stats: null, cola: [] });
    useUiStore.setState({ ajustesAbiertos: false, ayudaAbierta: false, densidad: "normal" });
    document.documentElement.style.removeProperty("--row-height");
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

  it("⇧L abre el modo etiquetado y cambia el teclado entero", async () => {
    render(<App />);
    await waitFor(() => expect(screen.getByText("Destinos")).toBeDefined());

    fireEvent.keyDown(window, { key: "L", shiftKey: true });
    await waitFor(() => expect(screen.getByText("Etiquetado")).toBeDefined());
    expect(screen.queryByText("Destinos")).toBeNull();

    // La letra etiqueta en vez de hacer lo que hacía en el triaje.
    await waitFor(() => expect(ipc.etiquetasDe).toHaveBeenCalled());
    fireEvent.keyDown(window, { key: "k" });
    await waitFor(() => expect(ipc.ponerEtiqueta).toHaveBeenCalledWith(10, "kind", "kick"));

    // Y en modo etiquetado NO se dispara una decisión de triaje.
    expect(ipc.enviar).not.toHaveBeenCalled();
    expect(ipc.rechazar).not.toHaveBeenCalled();
  });

  it("Esc sale del modo etiquetado y devuelve los destinos", async () => {
    render(<App />);
    fireEvent.keyDown(window, { key: "L", shiftKey: true });
    await waitFor(() => expect(screen.getByText("Etiquetado")).toBeDefined());
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.getByText("Destinos")).toBeDefined());
  });

  it("desde la barra lateral se puede añadir otra carpeta", async () => {
    render(<App />);
    await waitFor(() => expect(screen.getByText("Carpetas")).toBeDefined());
    fireEvent.click(screen.getByLabelText("Añadir carpeta"));
    await waitFor(() => expect(ipc.elegirCarpeta).toHaveBeenCalled());
  });

  it("reescanear una carpeta llama al backend con su id", async () => {
    render(<App />);
    await waitFor(() => expect(screen.getByText("samples")).toBeDefined());
    fireEvent.click(screen.getByLabelText("Reescanear /musica/samples"));
    await waitFor(() => expect(ipc.reescanear).toHaveBeenCalledWith(1, expect.any(Function)));
  });

  it("quitar una carpeta confirma en la propia fila, sin abrir un diálogo", async () => {
    render(<App />);
    await waitFor(() => expect(screen.getByText("samples")).toBeDefined());

    fireEvent.click(screen.getByLabelText("Quitar /musica/samples"));
    expect(screen.getByText("¿Quitar del índice?")).toBeDefined();
    expect(ipc.quitarFuente).not.toHaveBeenCalled();

    fireEvent.click(screen.getByText("No"));
    expect(screen.queryByText("¿Quitar del índice?")).toBeNull();
    expect(ipc.quitarFuente).not.toHaveBeenCalled();

    fireEvent.click(screen.getByLabelText("Quitar /musica/samples"));
    fireEvent.click(screen.getByText("Sí"));
    await waitFor(() => expect(ipc.quitarFuente).toHaveBeenCalledWith(1));
  });

  it("Ctrl+, abre los ajustes y Esc los cierra", async () => {
    render(<App />);
    await waitFor(() => expect(screen.getByText("Carpetas")).toBeDefined());

    // Por el rol, no por el texto: «Ajustes» aparece dos veces (el botón de la barra
    // lateral y el título del panel).
    fireEvent.keyDown(window, { key: ",", ctrlKey: true });
    await waitFor(() => expect(screen.getByRole("dialog", { name: "Ajustes" })).toBeDefined());
    expect(screen.getByText("Carpetas de samples")).toBeDefined();

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByText("Carpetas de samples")).toBeNull());
  });

  it("la densidad cambia el alto de fila de verdad", async () => {
    render(<App />);
    fireEvent.keyDown(window, { key: ",", ctrlKey: true });
    await waitFor(() => expect(screen.getByText("Compacta")).toBeDefined());

    fireEvent.click(screen.getByText("Compacta"));
    expect(document.documentElement.style.getPropertyValue("--row-height")).toBe("24px");
    expect(ipc.settingsSet).toHaveBeenCalledWith("densidad", "compacta");

    fireEvent.click(screen.getByText("Cómoda"));
    expect(document.documentElement.style.getPropertyValue("--row-height")).toBe("34px");
  });

  it("los ajustes enseñan la latencia que ha medido el motor", async () => {
    render(<App />);
    fireEvent.keyDown(window, { key: ",", ctrlKey: true });
    await waitFor(() => expect(screen.getByText(/p50 1.40 ms/)).toBeDefined());
    expect(screen.getByText(/p95 2.60 ms/)).toBeDefined();
  });

  it("con la biblioteca vacía abre el asistente en vez de dejar una pantalla muerta", async () => {
    vi.mocked(ipc.fuentes).mockResolvedValueOnce([]);
    render(<App />);
    await waitFor(() => expect(screen.getByText("Preparar la sesión")).toBeDefined());
  });
});
