import { useEffect, useState } from "react";
import { Boton } from "../../../components/Boton";
import { Kbd } from "../../../components/Kbd";
import { useLibraryStore } from "../../../features/library/store";
import { useTriageStore } from "../../../features/triage/store";
import { cifra } from "../../../lib/format";
import * as ipc from "../../../lib/ipc";
import { log } from "../../../lib/log";
import { useUiStore } from "../../uiStore";
import styles from "./SetupWizard.module.css";

/**
 * Preparar la sesión: carpeta de origen, carpeta de destino y los cubos.
 *
 * Las tres cosas a la vez, no un asistente por pasos: se hace una vez y en treinta segundos,
 * y obligar a pasar tres pantallas para volver a tocar una es peor que verlas todas.
 */
export function SetupWizard() {
  const fuentes = useLibraryStore((s) => s.fuentes);
  const proyecto = useTriageStore((s) => s.proyecto);
  const destinos = useTriageStore((s) => s.destinos);
  const [sugeridos, setSugeridos] = useState<string[]>([]);
  const [nuevo, setNuevo] = useState("");
  const [ocupado, setOcupado] = useState(false);

  useEffect(() => {
    if (!proyecto) return;
    ipc
      .destinosSugeridos(proyecto.id)
      .then(setSugeridos)
      .catch((e) => log.warn("no se pudieron leer las subcarpetas del destino", e));
  }, [proyecto]);

  const anadirOrigen = async () => {
    const ruta = await ipc.elegirCarpeta("Elige la carpeta con tus samples");
    if (!ruta) return;
    setOcupado(true);
    try {
      await ipc.anadirFuente(ruta, (p) => useLibraryStore.getState().setProgreso(p));
      await useLibraryStore.getState().cargarFuentes();
      await useLibraryStore.getState().refrescar();
    } catch (e) {
      useUiStore.getState().avisar("error", "No se pudo añadir la carpeta");
      log.error("añadir fuente", e);
    } finally {
      setOcupado(false);
    }
  };

  const elegirDestino = async () => {
    const ruta = await ipc.elegirCarpeta("Elige dónde construir tu librería ordenada");
    if (!ruta) return;
    const nombre = ruta.split("/").pop() ?? "Sesión";
    await useTriageStore.getState().crearProyecto(nombre, ruta, "move");
  };

  const yaListo = fuentes.length > 0 && proyecto !== null && destinos.length > 0;

  return (
    <div
      className={styles.fondo}
      onClick={() => useUiStore.getState().setAsistente(false)}
      role="presentation"
    >
      <div
        className={styles.hoja}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Preparar la sesión"
      >
        <header className={styles.cabecera}>
          <h2 className={styles.titulo}>Preparar la sesión</h2>
          <span className={styles.pista}>
            <Kbd>Esc</Kbd> para cerrar
          </span>
        </header>

        <section className={styles.paso}>
          <div className={styles.numero}>1</div>
          <div className={styles.contenido}>
            <div className={styles.encabezado}>
              <h3>Carpeta de samples</h3>
              <Boton onClick={() => void anadirOrigen()} atajo="O" deshabilitado={ocupado}>
                {ocupado ? "Escaneando…" : "Elegir carpeta"}
              </Boton>
            </div>
            {fuentes.length === 0 ? (
              <p className={styles.ayuda}>La carpeta desordenada, la que quieres triar.</p>
            ) : (
              <ul className={styles.lista}>
                {fuentes.map((f) => (
                  <li key={f.id} className={styles.elemento}>
                    <span className={styles.ruta}>{f.path}</span>
                    <span className={styles.cuenta}>{cifra(f.total)} samples</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </section>

        <section className={styles.paso}>
          <div className={styles.numero}>2</div>
          <div className={styles.contenido}>
            <div className={styles.encabezado}>
              <h3>Carpeta de destino</h3>
              <Boton onClick={() => void elegirDestino()} atajo="D">
                {proyecto ? "Cambiar" : "Elegir carpeta"}
              </Boton>
            </div>
            {proyecto ? (
              <p className={styles.ruta}>{proyecto.destRoot}</p>
            ) : (
              <p className={styles.ayuda}>Donde se construirá tu librería ordenada.</p>
            )}
          </div>
        </section>

        <section className={styles.paso} data-apagado={proyecto === null || undefined}>
          <div className={styles.numero}>3</div>
          <div className={styles.contenido}>
            <div className={styles.encabezado}>
              <h3>Destinos</h3>
              <span className={styles.ayuda}>Se les asigna una tecla del 1 al 9</span>
            </div>

            {sugeridos.length > 0 && (
              <div className={styles.sugerencias}>
                {sugeridos
                  .filter((s) => !destinos.some((d) => d.name === s))
                  .map((s) => (
                    <button
                      key={s}
                      type="button"
                      className={styles.sugerencia}
                      onClick={() => void useTriageStore.getState().crearDestino(s)}
                    >
                      + {s}
                    </button>
                  ))}
              </div>
            )}

            <div className={styles.nuevo}>
              <input
                className={styles.campo}
                value={nuevo}
                placeholder="Kicks, Snares, FX…"
                disabled={proyecto === null}
                onChange={(e) => setNuevo(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && nuevo.trim() !== "") {
                    void useTriageStore.getState().crearDestino(nuevo.trim());
                    setNuevo("");
                  }
                }}
                aria-label="Nombre del destino"
              />
            </div>

            <div className={styles.cubos}>
              {destinos.map((d) => (
                <span key={d.id} className={styles.cubo}>
                  <Kbd>{d.hotkey ?? "·"}</Kbd> {d.name}
                  <button
                    type="button"
                    className={styles.quitar}
                    onClick={() => void useTriageStore.getState().borrarDestino(d.id)}
                    aria-label={`Quitar ${d.name}`}
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
          </div>
        </section>

        <footer className={styles.pie}>
          <Boton
            variante="principal"
            onClick={() => useUiStore.getState().setAsistente(false)}
            atajo="Intro"
            deshabilitado={!yaListo}
          >
            {yaListo ? "Empezar a triar" : "Faltan pasos"}
          </Boton>
        </footer>
      </div>
    </div>
  );
}
