/**
 * La ÚNICA puerta de entrada al núcleo Rust.
 *
 * Ningún componente llama a `invoke` directamente: aquí se normalizan los errores, se fijan
 * los tipos (generados en `src/bindings.ts` con ts-rs) y se documenta el contrato.
 * Hay un hook que revisa que no aparezca `invoke(` fuera de este archivo.
 */
import { Channel, invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type {
  AppInfo,
  AudioInfo,
  Destination,
  LibraryPage,
  LibraryQuery,
  LibraryStats,
  PlaybackStarted,
  Project,
  SampleDetail,
  ScanProgress,
  SessionProgress,
  SourceInfo,
  TrashSummary,
  TriageMode,
  TriageResult,
  UndoResult,
} from "../bindings";

export type { LibraryQuery, SampleRow, ScanProgress, SortBy, StatusFilter } from "../bindings";

/** Error de la app tal y como lo emite Rust: `{ kind, message }`. */
export interface AppError {
  kind: string;
  message: string;
}

export function esAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "kind" in e && "message" in e;
}

function normalizar(e: unknown): AppError {
  if (esAppError(e)) return e;
  return { kind: "desconocido", message: String(e) };
}

async function llamar<T>(comando: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(comando, args);
  } catch (e) {
    throw normalizar(e);
  }
}

// ─────────────────────────── app ───────────────────────────

export const appInfo = (): Promise<AppInfo> => llamar("app_info");
export const settingsGet = (key: string): Promise<string | null> => llamar("settings_get", { key });
export const settingsSet = (key: string, value: string): Promise<void> =>
  llamar("settings_set", { key, value });

/** Diálogo nativo de carpeta. Va aquí para que los componentes no hablen con los plugins. */
export async function elegirCarpeta(titulo: string): Promise<string | null> {
  const r = await open({ directory: true, multiple: false, title: titulo });
  return typeof r === "string" ? r : null;
}

export async function revelarEnElExplorador(ruta: string): Promise<void> {
  await revealItemInDir(ruta);
}

/**
 * Carpetas soltadas sobre la ventana. Devuelve la función para dejar de escuchar.
 * Si el entorno no lo soporta, no pasa nada: el botón y la tecla O siguen ahí.
 */
export async function alSoltarCarpetas(cb: (rutas: string[]) => void): Promise<() => void> {
  try {
    return await getCurrentWebview().onDragDropEvent((evento) => {
      if (evento.payload.type === "drop" && evento.payload.paths.length > 0) {
        cb(evento.payload.paths);
      }
    });
  } catch {
    return () => {};
  }
}

// ─────────────────────────── biblioteca ───────────────────────────

export const fuentes = (): Promise<SourceInfo[]> => llamar("library_sources");

export function anadirFuente(
  path: string,
  alProgresar: (p: ScanProgress) => void,
): Promise<SourceInfo> {
  const canal = new Channel<ScanProgress>();
  canal.onmessage = alProgresar;
  return llamar("library_add_source", { path, progreso: canal });
}

export function reescanear(
  sourceId: number,
  alProgresar: (p: ScanProgress) => void,
): Promise<void> {
  const canal = new Channel<ScanProgress>();
  canal.onmessage = alProgresar;
  return llamar("library_rescan", { sourceId, progreso: canal });
}

export const quitarFuente = (sourceId: number): Promise<void> =>
  llamar("library_remove_source", { sourceId });

export const pagina = (query: LibraryQuery): Promise<LibraryPage> =>
  llamar("library_page", { query });

export const posicionDe = (query: LibraryQuery, sampleId: number): Promise<number | null> =>
  llamar("library_index_of", { query, sampleId });

export const estadisticas = (sourceId: number | null): Promise<LibraryStats> =>
  llamar("library_stats", { sourceId });

export const detalle = (sampleId: number): Promise<SampleDetail> =>
  llamar("library_detail", { sampleId });

export const valorar = (sampleId: number, rating: number): Promise<void> =>
  llamar("library_set_rating", { sampleId, rating });

export const analisisPendiente = (): Promise<number> => llamar("library_analysis_pending");

/**
 * Picos de la onda como BYTES CRUDOS: 2 por bucket (mín, máx en i8). 2 KB por sample en
 * binario frente a ~12 KB en JSON, y el canvas los pinta sin parsear nada.
 */
export async function picos(sampleId: number): Promise<Int8Array> {
  const buffer = await llamar<ArrayBuffer>("library_peaks", { sampleId });
  return new Int8Array(buffer);
}

// ─────────────────────────── reproductor ───────────────────────────

export const reproducir = (sampleId: number, looping: boolean): Promise<PlaybackStarted> =>
  llamar("player_play", { sampleId, looping });
export const parar = (): Promise<void> => llamar("player_stop");
export const buscarEn = (ms: number): Promise<void> => llamar("player_seek", { ms });
export const ganancia = (gain: number): Promise<void> => llamar("player_gain", { gain });
export const bucle = (looping: boolean): Promise<void> => llamar("player_set_loop", { looping });
export const prefetch = (sampleIds: number[]): Promise<void> =>
  llamar("player_prefetch", { sampleIds });
export const infoAudio = (): Promise<AudioInfo> => llamar("player_info");

// ─────────────────────────── triaje ───────────────────────────

export const proyectos = (): Promise<Project[]> => llamar("triage_projects");
export const ultimoProyecto = (): Promise<Project | null> => llamar("triage_last_project");
export const crearProyecto = (name: string, destRoot: string, mode: TriageMode): Promise<Project> =>
  llamar("triage_create_project", { name, destRoot, mode });
export const abrirProyecto = (projectId: number): Promise<Project> =>
  llamar("triage_open_project", { projectId });
export const borrarProyecto = (projectId: number): Promise<void> =>
  llamar("triage_delete_project", { projectId });
export const cambiarModo = (projectId: number, mode: TriageMode): Promise<void> =>
  llamar("triage_set_mode", { projectId, mode });

export const destinos = (projectId: number): Promise<Destination[]> =>
  llamar("triage_destinations", { projectId });
export const crearDestino = (
  projectId: number,
  name: string,
  relPath: string,
): Promise<Destination> => llamar("triage_create_destination", { projectId, name, relPath });
export const renombrarDestino = (destId: number, name: string, relPath: string): Promise<void> =>
  llamar("triage_rename_destination", { destId, name, relPath });
export const borrarDestino = (destId: number): Promise<void> =>
  llamar("triage_delete_destination", { destId });
export const destinosSugeridos = (projectId: number): Promise<string[]> =>
  llamar("triage_suggest_destinations", { projectId });

export const enviar = (
  projectId: number,
  destId: number,
  sampleIds: number[],
): Promise<TriageResult> => llamar("triage_send", { projectId, destId, sampleIds });
export const rechazar = (projectId: number, sampleIds: number[]): Promise<TriageResult> =>
  llamar("triage_reject", { projectId, sampleIds });
export const conservar = (projectId: number, sampleIds: number[]): Promise<TriageResult> =>
  llamar("triage_keep", { projectId, sampleIds });
export const renombrar = (
  sampleId: number,
  name: string,
  projectId: number | null,
): Promise<string> => llamar("triage_rename", { sampleId, name, projectId });
export const exportarDecisiones = (projectId: number): Promise<string> =>
  llamar("triage_export", { projectId });
export const deshacer = (): Promise<UndoResult> => llamar("triage_undo");
export const rehacer = (): Promise<UndoResult> => llamar("triage_redo");

export const progresoSesion = (sourceId: number | null): Promise<SessionProgress> =>
  llamar("triage_progress", { sourceId });
export const recordarPosicion = (projectId: number, sampleId: number): Promise<void> =>
  llamar("triage_remember", { projectId, sampleId });
export const ultimaPosicion = (projectId: number): Promise<number | null> =>
  llamar("triage_last_sample", { projectId });

export const resumenPapelera = (projectId: number): Promise<TrashSummary> =>
  llamar("triage_trash_summary", { projectId });
export const vaciarPapelera = (projectId: number): Promise<number> =>
  llamar("triage_empty_trash", { projectId });
