import { useEffect, useState } from "react";
import { type Densidad, useUiStore } from "../../../../app/uiStore";
import type { AppInfo, AudioInfo } from "../../../../bindings";
import { Boton } from "../../../../components/Boton";
import { Kbd } from "../../../../components/Kbd";
import { cifra, tamano } from "../../../../lib/format";
import * as ipc from "../../../../lib/ipc";
import { log } from "../../../../lib/log";
import { useLibraryStore } from "../../../library/store";
import { usePlayerStore } from "../../../player/store";
import { useTriageStore } from "../../../triage/store";
import styles from "./SettingsPanel.module.css";

const DENSIDADES: Array<[Densidad, string]> = [
  ["compacta", "Compacta"],
  ["normal", "Normal"],
  ["comoda", "Cómoda"],
];

export function SettingsPanel() {
  const fuentes = useLibraryStore((s) => s.fuentes);
  const escaneando = useLibraryStore((s) => s.escaneando);
  const tema = useUiStore((s) => s.tema);
  const densidad = useUiStore((s) => s.densidad);
  const autoplay = usePlayerStore((s) => s.autoplay);
  const volumen = usePlayerStore((s) => s.volumen);
  const papelera = useTriageStore((s) => s.papelera);

  const [info, setInfo] = useState<AppInfo | null>(null);
  const [audio, setAudio] = useState<AudioInfo | null>(null);
  const [confirmando, setConfirmando] = useState<number | null>(null);
  const [reconectando, setReconectando] = useState(false);

  useEffect(() => {
    ipc
      .appInfo()
      .then(setInfo)
      .catch((e) => log.warn("sin info de la app", e));
    ipc
      .infoAudio()
      .then(setAudio)
      .catch(() => setAudio(null));
    void useTriageStore.getState().refrescarPapelera();
  }, []);

  // La app reconecta sola cuando detecta que la salida dejó de responder. Esto es el botón
  // de emergencia por si en algún sistema la detección no llega: reabre el dispositivo y
  // vuelve a leer la información para que se vea el resultado.
  async function reconectarAudio() {
    setReconectando(true);
    try {
      await ipc.reconectarAudio();
      await new Promise((r) => setTimeout(r, 400));
      setAudio(await ipc.infoAudio());
    } catch (e) {
      log.warn("no se pudo reconectar el audio", e);
    } finally {
      setReconectando(false);
    }
  }

  return (
    <div
      className={styles.fondo}
      onClick={() => useUiStore.getState().setAjustes(false)}
      role="presentation"
    >
      <div
        className={styles.hoja}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Ajustes"
      >
        <header className={styles.cabecera}>
          <h2 className={styles.titulo}>Ajustes</h2>
          <span className={styles.pista}>
            <Kbd>Esc</Kbd> para cerrar
          </span>
        </header>

        <section className={styles.seccion}>
          <div className={styles.encabezado}>
            <h3>Carpetas de samples</h3>
            <Boton
              onClick={() => void useLibraryStore.getState().anadirCarpeta()}
              deshabilitado={escaneando !== null}
            >
              {escaneando === -1 ? "Escaneando…" : "Añadir carpeta"}
            </Boton>
          </div>

          {fuentes.length === 0 ? (
            <p className={styles.ayuda}>Todavía no has añadido ninguna.</p>
          ) : (
            <ul className={styles.carpetas}>
              {fuentes.map((f) => (
                <li key={f.id} className={styles.carpeta}>
                  <div className={styles.datos}>
                    <span className={styles.ruta} title={f.path}>
                      {f.path}
                    </span>
                    <span className={styles.cuenta}>
                      {cifra(f.total)} samples · {cifra(f.analyzed)} analizados
                    </span>
                  </div>
                  {confirmando === f.id ? (
                    <div className={styles.confirmar}>
                      <span>¿Quitar del índice?</span>
                      <button
                        type="button"
                        className={styles.si}
                        onClick={() => {
                          setConfirmando(null);
                          void useLibraryStore.getState().quitarFuenteDelIndice(f.id);
                        }}
                      >
                        Sí
                      </button>
                      <button
                        type="button"
                        className={styles.no}
                        onClick={() => setConfirmando(null)}
                      >
                        No
                      </button>
                    </div>
                  ) : (
                    <div className={styles.acciones}>
                      <Boton
                        onClick={() => void useLibraryStore.getState().reescanearFuente(f.id)}
                        deshabilitado={escaneando !== null}
                        titulo="Entra lo nuevo, se actualiza lo cambiado y se quita lo que ya no está en disco. Lo que hayas triado no se toca."
                      >
                        {escaneando === f.id ? "Reescaneando…" : "Reescanear"}
                      </Boton>
                      <Boton
                        variante="peligro"
                        onClick={() => setConfirmando(f.id)}
                        titulo="Solo la quita del índice: no se borra ningún archivo del disco"
                      >
                        Quitar
                      </Boton>
                    </div>
                  )}
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className={styles.seccion}>
          <h3>Apariencia</h3>
          <div className={styles.fila}>
            <span className={styles.etiqueta}>Tema</span>
            <div className={styles.opciones}>
              {(["dark", "light"] as const).map((t) => (
                <button
                  key={t}
                  type="button"
                  className={styles.pastilla}
                  data-activa={tema === t || undefined}
                  onClick={() => {
                    useUiStore.getState().aplicarTema(t);
                    void ipc.settingsSet("tema", t).catch(() => {});
                  }}
                >
                  {t === "dark" ? "Oscuro" : "Claro"}
                </button>
              ))}
              <Kbd>T</Kbd>
            </div>
          </div>
          <div className={styles.fila}>
            <span className={styles.etiqueta}>Densidad de la lista</span>
            <div className={styles.opciones}>
              {DENSIDADES.map(([valor, nombre]) => (
                <button
                  key={valor}
                  type="button"
                  className={styles.pastilla}
                  data-activa={densidad === valor || undefined}
                  onClick={() => {
                    useUiStore.getState().aplicarDensidad(valor);
                    void ipc.settingsSet("densidad", valor).catch(() => {});
                  }}
                >
                  {nombre}
                </button>
              ))}
            </div>
          </div>
        </section>

        <section className={styles.seccion}>
          <h3>Escucha</h3>
          <div className={styles.fila}>
            <span className={styles.etiqueta}>Sonar al enfocar una fila</span>
            <div className={styles.opciones}>
              <button
                type="button"
                className={styles.pastilla}
                data-activa={autoplay || undefined}
                onClick={() => usePlayerStore.getState().alternarAutoplay()}
              >
                {autoplay ? "Sí" : "No"}
              </button>
              <Kbd>⇧ A</Kbd>
            </div>
          </div>
          <div className={styles.fila}>
            <span className={styles.etiqueta}>Volumen</span>
            <div className={styles.opciones}>
              <span className={styles.valor}>{Math.round(volumen * 100)} %</span>
              <Kbd>+</Kbd>
              <Kbd>−</Kbd>
            </div>
          </div>
        </section>

        {papelera !== null && (
          <section className={styles.seccion}>
            <h3>Papelera</h3>
            <div className={styles.fila}>
              <span className={styles.etiqueta}>
                {papelera.files === 0
                  ? "Vacía"
                  : `${cifra(papelera.files)} archivos · ${tamano(papelera.bytes)}`}
              </span>
              {papelera.files > 0 && (
                <Boton
                  variante="peligro"
                  onClick={() => {
                    const detalle = `${cifra(papelera.files)} archivos (${tamano(papelera.bytes)})`;
                    const ok = window.confirm(
                      `Se van a borrar definitivamente ${detalle}.\n\nEs la única acción de la app que no se puede deshacer. ¿Seguro?`,
                    );
                    if (ok) void useTriageStore.getState().vaciarPapelera();
                  }}
                >
                  Vaciar
                </Boton>
              )}
            </div>
          </section>
        )}

        <section className={styles.seccion}>
          <h3>Información</h3>
          <dl className={styles.info}>
            <dt>Versión</dt>
            <dd>{info?.version ?? "—"}</dd>
            <dt>Índice</dt>
            <dd className={styles.ruta}>{info?.dbPath ?? "—"}</dd>
            <dt>Salida de audio</dt>
            <dd className={styles.filaAudio}>
              <span>
                {audio === null
                  ? (info?.audioError ?? "no disponible")
                  : `${audio.device} · ${audio.sampleRate} Hz · ${audio.channels} canales · buffer ${
                      audio.bufferFixed ? `${audio.bufferFrames} frames` : "del sistema"
                    }`}
              </span>
              <Boton
                variante="normal"
                onClick={() => void reconectarAudio()}
                deshabilitado={reconectando}
                titulo="Vuelve a abrir el dispositivo de salida. Úsalo si cambiaste de auriculares o altavoces y dejó de sonar."
              >
                {reconectando ? "Reconectando…" : "Reconectar"}
              </Boton>
            </dd>
            {audio !== null && audio.reconnections > 0 && (
              <>
                <dt>Reconexiones</dt>
                <dd>
                  {cifra(audio.reconnections)}
                  <span className={styles.sobre}> desde que abriste la app</span>
                </dd>
              </>
            )}
            {audio !== null && audio.shots > 0 && (
              <>
                <dt>Latencia medida</dt>
                <dd>
                  p50 {audio.latencyP50Ms.toFixed(2)} ms · p95 {audio.latencyP95Ms.toFixed(2)} ms
                  <span className={styles.sobre}> sobre {cifra(audio.shots)} disparos</span>
                </dd>
              </>
            )}
            {audio !== null && (
              <>
                <dt>Caché de audio</dt>
                <dd>
                  {tamano(audio.cacheBytes)} de {tamano(audio.cacheLimitBytes)} ·{" "}
                  {cifra(audio.cacheEntries)} samples en RAM
                </dd>
              </>
            )}
          </dl>
        </section>
      </div>
    </div>
  );
}
