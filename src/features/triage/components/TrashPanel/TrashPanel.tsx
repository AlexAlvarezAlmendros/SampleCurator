import { Boton } from "../../../../components/Boton";
import { Kbd } from "../../../../components/Kbd";
import { cifra, duracion, tamano } from "../../../../lib/format";
import { usePlayerStore } from "../../../player/store";
import { useTriageStore } from "../../store";
import { useTrashStore } from "../../store.papelera";
import styles from "./TrashPanel.module.css";

function cuando(ms: number): string {
  if (ms === 0) return "sin fecha";
  const minutos = Math.round((Date.now() - ms) / 60000);
  if (minutos < 1) return "ahora mismo";
  if (minutos < 60) return `hace ${minutos} min`;
  const horas = Math.round(minutos / 60);
  if (horas < 24) return `hace ${horas} h`;
  return `hace ${Math.round(horas / 24)} d`;
}

export function TrashPanel() {
  const entradas = useTrashStore((s) => s.entradas);
  const cargando = useTrashStore((s) => s.cargando);
  const sonando = usePlayerStore((s) => s.sonando);
  const papelera = useTriageStore((s) => s.papelera);

  return (
    <div
      className={styles.fondo}
      onClick={() => useTrashStore.getState().cerrar()}
      role="presentation"
    >
      <div
        className={styles.hoja}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Papelera"
      >
        <header className={styles.cabecera}>
          <h2 className={styles.titulo}>Papelera</h2>
          <span className={styles.resumen}>
            {papelera !== null && papelera.files > 0
              ? `${cifra(papelera.files)} archivos · ${tamano(papelera.bytes)}`
              : "vacía"}
          </span>
          <span className={styles.pista}>
            <Kbd>Esc</Kbd> para cerrar
          </span>
        </header>

        <div className={styles.cuerpo}>
          {cargando && <p className={styles.aviso}>Leyendo…</p>}

          {!cargando && entradas.length === 0 && (
            <p className={styles.aviso}>
              No hay nada rechazado. Lo que descartes con <Kbd>X</Kbd> aparecerá aquí y podrás
              devolverlo a su sitio.
            </p>
          )}

          {entradas.map((e) => (
            <div
              key={e.trashPath}
              className={styles.entrada}
              data-sonando={sonando === e.sampleId || undefined}
            >
              <button
                type="button"
                className={styles.escuchar}
                onClick={() => useTrashStore.getState().escuchar(e)}
                disabled={e.sampleId === null}
                title={e.sampleId === null ? "Ya no está en el índice" : "Escuchar"}
                aria-label={`Escuchar ${e.filename}`}
              >
                ▸
              </button>

              <div className={styles.datos}>
                <span className={styles.nombre}>{e.filename}</span>
                <span className={styles.origen} title={e.originalPath}>
                  {e.originalPath === "" ? "sin origen anotado" : e.originalPath}
                </span>
              </div>

              <span className={styles.meta}>{duracion(e.durationMs)}</span>
              <span className={styles.meta}>{tamano(e.size)}</span>
              <span className={styles.meta}>{cuando(e.at)}</span>

              <Boton
                onClick={() => void useTrashStore.getState().restaurar(e.trashPath)}
                deshabilitado={e.originalPath === ""}
                titulo={
                  e.originalPath === ""
                    ? "No se sabe de dónde venía, así que no hay sitio al que devolverlo"
                    : "Devolverlo a su carpeta original"
                }
              >
                Restaurar
              </Boton>
            </div>
          ))}
        </div>

        {entradas.length > 0 && (
          <footer className={styles.pie}>
            <p className={styles.nota}>
              Restaurar devuelve el archivo a su carpeta y lo pone otra vez en la cola. Vaciar es la
              única acción de la app que no se puede deshacer.
            </p>
            <Boton
              variante="peligro"
              onClick={() => {
                const p = useTriageStore.getState().papelera;
                if (!p) return;
                const detalle = `${cifra(p.files)} archivos (${tamano(p.bytes)})`;
                const ok = window.confirm(
                  `Se van a borrar definitivamente ${detalle}.\n\nEs la única acción de la app que no se puede deshacer. ¿Seguro?`,
                );
                if (!ok) return;
                void useTriageStore
                  .getState()
                  .vaciarPapelera()
                  .then(() => useTrashStore.getState().refrescar());
              }}
            >
              Vaciar papelera
            </Boton>
          </footer>
        )}
      </div>
    </div>
  );
}
