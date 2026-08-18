import { useEffect, useState } from "react";
import { Boton } from "../../../../components/Boton";
import { Kbd } from "../../../../components/Kbd";
import { cifra } from "../../../../lib/format";
import { TECLAS_TIPO, useLabelsStore } from "../../store";
import styles from "./LabelPanel.module.css";

/**
 * El panel del modo etiquetado. Ocupa el sitio de los destinos porque hace su mismo papel:
 * una tecla, una decisión, y a por el siguiente.
 *
 * Arriba enseña la medida que decide el gate de la Fase 8 — cuánto coincide lo que dicen los
 * nombres con lo que dices tú — para que se vea crecer mientras etiquetas.
 */
export function LabelPanel() {
  const etiquetas = useLabelsStore((s) => s.etiquetas);
  const stats = useLabelsStore((s) => s.stats);
  const extrayendo = useLabelsStore((s) => s.extrayendo);
  const [bpm, setBpm] = useState("");
  const [tono, setTono] = useState("");

  useEffect(() => {
    setBpm(etiquetas?.bpm !== null && etiquetas?.bpm !== undefined ? String(etiquetas.bpm) : "");
    setTono(etiquetas?.key ?? "");
  }, [etiquetas]);

  const tipo = etiquetas?.kind ?? null;
  const hechos = stats?.labeledSamples ?? 0;
  const objetivo = stats?.target ?? 200;
  const kind = stats?.fields.find((f) => f.field === "kind");

  return (
    <aside className={styles.panel}>
      <div className={styles.titulo}>
        Etiquetado <Kbd>⇧L</Kbd>
      </div>

      <div className={styles.progreso}>
        <div className={styles.cifra}>
          {cifra(hechos)} <span className={styles.de}>/ {cifra(objetivo)}</span>
        </div>
        <div className={styles.barra}>
          <div
            className={styles.relleno}
            style={{ width: `${Math.min(100, (hechos / Math.max(1, objetivo)) * 100)}%` }}
          />
        </div>
        <div className={styles.etiquetaProgreso}>samples con verdad tuya</div>
      </div>

      {kind !== undefined && kind.pairs > 0 && (
        <div className={styles.medida}>
          <div className={styles.medidaTitulo}>Los nombres aciertan</div>
          <div className={styles.medidaCifra}>{(kind.accuracy * 100).toFixed(0)}%</div>
          <div className={styles.medidaPie}>
            sobre {cifra(kind.pairs)} comparaciones · {cifra(kind.onlyUser)} sin pista en el nombre
          </div>
        </div>
      )}

      <div className={styles.clases}>
        {TECLAS_TIPO.map(([tecla, valor, nombre]) => (
          <button
            key={valor}
            type="button"
            className={styles.clase}
            data-activa={tipo === valor || undefined}
            onClick={() => void useLabelsStore.getState().ponerTipo(valor)}
          >
            <Kbd>{tecla.toUpperCase()}</Kbd>
            <span className={styles.nombreClase}>{nombre}</span>
          </button>
        ))}
      </div>

      <div className={styles.campos}>
        <label className={styles.campo}>
          <span className={styles.etiquetaCampo}>BPM</span>
          <input
            className={styles.entrada}
            value={bpm}
            placeholder="128"
            onChange={(e) => setBpm(e.target.value)}
            onBlur={() => void useLabelsStore.getState().poner("bpm", bpm)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void useLabelsStore.getState().poner("bpm", bpm);
            }}
          />
        </label>
        <label className={styles.campo}>
          <span className={styles.etiquetaCampo}>Tono</span>
          <input
            className={styles.entrada}
            value={tono}
            placeholder="A:min"
            onChange={(e) => setTono(e.target.value)}
            onBlur={() => void useLabelsStore.getState().poner("key", tono)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void useLabelsStore.getState().poner("key", tono);
            }}
          />
        </label>
      </div>

      <div className={styles.pie}>
        <Boton
          onClick={() => void useLabelsStore.getState().extraerDeNombres()}
          deshabilitado={extrayendo}
          titulo="Vuelve a leer todos los nombres de archivo de la biblioteca"
        >
          {extrayendo ? "Leyendo…" : "Releer nombres"}
        </Boton>
        <p className={styles.ayuda}>
          Escucha con <Kbd>↓</Kbd> <Kbd>↑</Kbd> y etiqueta con la letra. <Kbd>Esc</Kbd> para salir.
        </p>
      </div>
    </aside>
  );
}
