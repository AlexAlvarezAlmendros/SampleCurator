import { useEffect, useState } from "react";
import { Chip } from "../../../../components/Chip";
import { Estrellas } from "../../../../components/Estrellas";
import { Kbd } from "../../../../components/Kbd";
import { canales, duracion, hz, tamano } from "../../../../lib/format";
import { useMetaStore } from "../../store";
import styles from "./InspectorPanel.module.css";

/**
 * Todo lo que se puede decir de un sample, en el sitio del panel de destinos.
 *
 * Nada de esto toca el archivo: vive en el índice. Es la diferencia entre poder deshacerlo
 * todo y tener que fiarse.
 */
export function InspectorPanel() {
  const detalle = useMetaStore((s) => s.detalle);
  const catalogo = useMetaStore((s) => s.catalogo);
  const [nueva, setNueva] = useState("");
  const [notas, setNotas] = useState("");

  useEffect(() => {
    setNotas(detalle?.notes ?? "");
    setNueva("");
  }, [detalle]);

  if (!detalle) {
    return (
      <aside className={styles.panel}>
        <div className={styles.titulo}>
          Inspector <Kbd>I</Kbd>
        </div>
        <p className={styles.vacio}>Enfoca un sample para ver y editar sus datos.</p>
      </aside>
    );
  }

  const fila = detalle.row;
  const sugeridas = catalogo
    .filter(([n]) => !detalle.tags.includes(n) && n.startsWith(nueva.trim().toLowerCase()))
    .slice(0, 6);

  return (
    <aside className={styles.panel}>
      <div className={styles.titulo}>
        Inspector <Kbd>I</Kbd>
      </div>

      <div className={styles.nombre} title={fila.relPath}>
        {fila.filename}
      </div>

      <div className={styles.bloque}>
        <span className={styles.etiqueta}>Valoración</span>
        <div className={styles.valoracion}>
          <Estrellas
            valor={fila.rating}
            tamano="grande"
            onChange={(v) => void useMetaStore.getState().valorar(v)}
          />
          <Kbd>Alt+1…5</Kbd>
        </div>
      </div>

      <div className={styles.bloque}>
        <span className={styles.etiqueta}>Etiquetas</span>
        <div className={styles.tags}>
          {detalle.tags.map((t) => (
            <button
              key={t}
              type="button"
              className={styles.tag}
              onClick={() => void useMetaStore.getState().quitarEtiqueta(t)}
              title="Quitar esta etiqueta"
            >
              {t} <span className={styles.quitar}>×</span>
            </button>
          ))}
          {detalle.tags.length === 0 && <span className={styles.ninguna}>ninguna</span>}
        </div>
        <input
          className={styles.campo}
          value={nueva}
          placeholder="Añadir etiqueta…"
          onChange={(e) => setNueva(e.target.value)}
          onKeyDown={(e) => {
            if (e.key !== "Enter" || nueva.trim() === "") return;
            void useMetaStore.getState().anadirEtiqueta(nueva);
            setNueva("");
          }}
          aria-label="Añadir etiqueta"
        />
        {sugeridas.length > 0 && (
          <div className={styles.sugerencias}>
            {sugeridas.map(([n, cuantos]) => (
              <button
                key={n}
                type="button"
                className={styles.sugerencia}
                onClick={() => {
                  void useMetaStore.getState().anadirEtiqueta(n);
                  setNueva("");
                }}
              >
                {n} <span className={styles.cuantos}>{cuantos}</span>
              </button>
            ))}
          </div>
        )}
      </div>

      <div className={styles.bloque}>
        <span className={styles.etiqueta}>Notas</span>
        <textarea
          className={styles.notas}
          value={notas}
          placeholder="Para qué sirve, con qué pega…"
          onChange={(e) => setNotas(e.target.value)}
          onBlur={() => void useMetaStore.getState().guardarNotas(notas)}
          aria-label="Notas del sample"
        />
      </div>

      <dl className={styles.datos}>
        <dt>Duración</dt>
        <dd>{duracion(fila.durationMs)}</dd>
        <dt>Formato</dt>
        <dd>
          {hz(fila.sampleRate)} · {canales(fila.channels)}
          {detalle.bitDepth !== null && ` · ${detalle.bitDepth} bits`}
        </dd>
        <dt>Tamaño</dt>
        <dd>{tamano(fila.size)}</dd>
        {detalle.loudnessDb !== null && (
          <>
            <dt>Volumen</dt>
            <dd>{detalle.loudnessDb.toFixed(1)} dB</dd>
          </>
        )}
        <dt>Estado</dt>
        <dd>
          {fila.destination !== null ? <Chip tono="keep">{fila.destination}</Chip> : fila.status}
        </dd>
      </dl>

      <p className={styles.aviso}>Nada de esto modifica el archivo: vive en el índice.</p>
    </aside>
  );
}
