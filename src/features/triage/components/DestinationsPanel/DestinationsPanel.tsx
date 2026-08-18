import { useState } from "react";
import { Boton } from "../../../../components/Boton";
import { Kbd } from "../../../../components/Kbd";
import { cifra, tamano } from "../../../../lib/format";
import { useTriageStore } from "../../store";
import styles from "./DestinationsPanel.module.css";

/**
 * Los cubos de destino. Es el único feedback de que la tecla ha entrado —el contador sube y
 * el cubo parpadea— así que siempre está visible y siempre responde en el acto.
 */
export function DestinationsPanel() {
  const proyecto = useTriageStore((s) => s.proyecto);
  const destinos = useTriageStore((s) => s.destinos);
  const papelera = useTriageStore((s) => s.papelera);
  const [nuevo, setNuevo] = useState("");

  const crear = () => {
    const nombre = nuevo.trim();
    if (nombre === "") return;
    void useTriageStore.getState().crearDestino(nombre);
    setNuevo("");
  };

  if (!proyecto) {
    return (
      <aside className={styles.panel}>
        <div className={styles.titulo}>Destinos</div>
        <div className={styles.vacio}>
          <p>Aún no hay sesión de triaje.</p>
          <p>
            Pulsa <Kbd>D</Kbd> para elegir la carpeta donde quieres construir tu librería.
          </p>
        </div>
      </aside>
    );
  }

  return (
    <aside className={styles.panel}>
      <div className={styles.titulo}>Destinos</div>

      <div className={styles.cubos}>
        {destinos.map((d) => (
          <button
            key={d.id}
            type="button"
            className={styles.cubo}
            style={{ ["--color-cubo" as string]: `var(--${d.color})` }}
            onClick={() => void useTriageStore.getState().enviarA(d.id)}
            title={`Enviar a ${d.name}`}
          >
            <span className={styles.tecla}>{d.hotkey ?? "·"}</span>
            <span className={styles.nombre}>{d.name}</span>
            <span className={styles.contador} key={`${d.id}-${d.count}`}>
              {cifra(d.count)}
            </span>
          </button>
        ))}
        {destinos.length === 0 && (
          <div className={styles.vacio}>
            <p>Sin destinos todavía.</p>
            <p>Escribe un nombre abajo y se le asignará la siguiente tecla libre.</p>
          </div>
        )}
      </div>

      <div className={styles.nuevo}>
        <input
          className={styles.campo}
          value={nuevo}
          placeholder="Nuevo destino…"
          onChange={(e) => setNuevo(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") crear();
          }}
          aria-label="Nombre del nuevo destino"
        />
        <Boton onClick={crear} deshabilitado={nuevo.trim() === ""}>
          Añadir
        </Boton>
      </div>

      <div className={styles.pie}>
        <div className={styles.modo}>
          <span>Modo</span>
          <button
            type="button"
            className={styles.pastilla}
            data-activa={proyecto.mode === "move" || undefined}
            onClick={() => void useTriageStore.getState().cambiarModo("move")}
          >
            mover
          </button>
          <button
            type="button"
            className={styles.pastilla}
            data-activa={proyecto.mode === "copy" || undefined}
            onClick={() => void useTriageStore.getState().cambiarModo("copy")}
          >
            copiar
          </button>
        </div>

        {papelera !== null && papelera.files > 0 && (
          <div className={styles.papelera}>
            <span>
              Papelera: {cifra(papelera.files)} · {tamano(papelera.bytes)}
            </span>
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
          </div>
        )}
      </div>
    </aside>
  );
}
