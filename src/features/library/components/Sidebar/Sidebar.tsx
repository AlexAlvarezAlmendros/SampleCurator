import { useEffect, useRef, useState } from "react";
import { useUiStore } from "../../../../app/uiStore";
import type { SortBy, StatusFilter } from "../../../../bindings";
import { Kbd } from "../../../../components/Kbd";
import { cifra } from "../../../../lib/format";
import { useLibraryStore } from "../../store";
import styles from "./Sidebar.module.css";

const FILTROS: Array<{ valor: StatusFilter; etiqueta: string; atajo?: string }> = [
  { valor: "all", etiqueta: "Todos" },
  { valor: "pending", etiqueta: "Pendientes", atajo: "⇧P" },
  { valor: "decided", etiqueta: "Decididos" },
  { valor: "kept", etiqueta: "Conservados" },
  { valor: "rejected", etiqueta: "Rechazados" },
  { valor: "duplicates", etiqueta: "Duplicados", atajo: "⇧D" },
];

const ORDENES: Array<{ valor: SortBy; etiqueta: string }> = [
  { valor: "path", etiqueta: "Ruta" },
  { valor: "filename", etiqueta: "Nombre" },
  { valor: "duration", etiqueta: "Duración" },
  { valor: "size", etiqueta: "Tamaño" },
  { valor: "loudness", etiqueta: "Volumen" },
  { valor: "recent", etiqueta: "Recientes" },
];

export function Sidebar() {
  const fuentes = useLibraryStore((s) => s.fuentes);
  const fuenteActiva = useLibraryStore((s) => s.fuenteActiva);
  const estado = useLibraryStore((s) => s.estado);
  const orden = useLibraryStore((s) => s.orden);
  const duracion = useLibraryStore((s) => s.duracion);
  const minValoracion = useLibraryStore((s) => s.minValoracion);
  const stats = useLibraryStore((s) => s.stats);
  const progreso = useLibraryStore((s) => s.progreso);
  const buscando = useUiStore((s) => s.buscando);

  const escaneando = useLibraryStore((s) => s.escaneando);
  const [texto, setTexto] = useState("");
  const [confirmando, setConfirmando] = useState<number | null>(null);
  const campo = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (buscando) campo.current?.focus();
  }, [buscando]);

  // Búsqueda incremental con freno: 80 ms es suficiente para no consultar en cada tecla y
  // poco para que se sienta inmediata.
  useEffect(() => {
    const t = setTimeout(() => {
      void useLibraryStore.getState().setBusqueda(texto);
    }, 80);
    return () => clearTimeout(t);
  }, [texto]);

  const analizando = progreso !== null && progreso.pendingAnalysis > 0;

  return (
    <aside className={styles.barra}>
      <div className={styles.busqueda}>
        <input
          ref={campo}
          className={styles.campo}
          type="search"
          value={texto}
          placeholder="Buscar…"
          onChange={(e) => setTexto(e.target.value)}
          onFocus={() => useUiStore.getState().setBuscando(true)}
          aria-label="Buscar samples"
        />
        <Kbd>/</Kbd>
      </div>

      <div className={styles.seccion}>
        <div className={styles.cabeceraSeccion}>
          <span className={styles.titulo}>Carpetas</span>
          <button
            type="button"
            className={styles.accionTitulo}
            onClick={() => void useLibraryStore.getState().anadirCarpeta()}
            disabled={escaneando !== null}
            title="Añadir otra carpeta de samples"
            aria-label="Añadir carpeta"
          >
            +
          </button>
        </div>

        {fuentes.length === 0 && (
          <div className={styles.vacio}>
            Ninguna. Pulsa <Kbd>O</Kbd>
          </div>
        )}

        {fuentes.map((f) => (
          <div
            key={f.id}
            className={styles.fuente}
            data-activa={f.id === fuenteActiva || undefined}
            data-ocupada={escaneando === f.id || undefined}
          >
            {confirmando === f.id ? (
              // Confirmación en la propia fila: el flujo de esta app no abre modales.
              <div className={styles.confirmar}>
                <span className={styles.pregunta}>¿Quitar del índice?</span>
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
                <button type="button" className={styles.no} onClick={() => setConfirmando(null)}>
                  No
                </button>
              </div>
            ) : (
              <>
                <button
                  type="button"
                  className={styles.nombreBoton}
                  onClick={() => void useLibraryStore.getState().setFuente(f.id)}
                  title={f.path}
                >
                  <span className={styles.nombreFuente}>{f.path.split("/").pop() ?? f.path}</span>
                  <span className={styles.cuenta}>
                    {escaneando === f.id ? "…" : cifra(f.total)}
                  </span>
                </button>
                <span className={styles.acciones}>
                  <button
                    type="button"
                    className={styles.iconito}
                    disabled={escaneando !== null}
                    onClick={() => void useLibraryStore.getState().reescanearFuente(f.id)}
                    title="Volver a recorrerla: entra lo nuevo y se quita lo que ya no está"
                    aria-label={`Reescanear ${f.path}`}
                  >
                    ↻
                  </button>
                  <button
                    type="button"
                    className={styles.iconito}
                    onClick={() => setConfirmando(f.id)}
                    title="Quitarla del índice (no se borra ningún archivo del disco)"
                    aria-label={`Quitar ${f.path}`}
                  >
                    ×
                  </button>
                </span>
              </>
            )}
          </div>
        ))}
      </div>

      <div className={styles.seccion}>
        <div className={styles.titulo}>Filtro</div>
        {FILTROS.map((f) => (
          <button
            key={f.valor}
            type="button"
            className={styles.opcion}
            data-activa={estado === f.valor || undefined}
            onClick={() => void useLibraryStore.getState().setEstado(f.valor)}
          >
            <span>{f.etiqueta}</span>
            {f.atajo !== undefined && <Kbd>{f.atajo}</Kbd>}
          </button>
        ))}
      </div>

      <div className={styles.seccion}>
        <div className={styles.titulo}>Duración</div>
        <div className={styles.ordenes}>
          {(
            [
              ["todo", "Todo"],
              ["oneshots", "One-shots"],
              ["loops", "Loops"],
            ] as const
          ).map(([valor, etiqueta]) => (
            <button
              key={valor}
              type="button"
              className={styles.pastilla}
              data-activa={duracion === valor || undefined}
              onClick={() => void useLibraryStore.getState().setDuracion(valor)}
            >
              {etiqueta}
            </button>
          ))}
        </div>
        <div className={styles.ordenes}>
          {[0, 3, 5].map((v) => (
            <button
              key={v}
              type="button"
              className={styles.pastilla}
              data-activa={minValoracion === v || undefined}
              onClick={() => void useLibraryStore.getState().setMinValoracion(v)}
            >
              {v === 0 ? "Sin filtro" : `★ ${v}+`}
            </button>
          ))}
        </div>
      </div>

      <div className={styles.seccion}>
        <div className={styles.titulo}>Orden</div>
        <div className={styles.ordenes}>
          {ORDENES.map((o) => (
            <button
              key={o.valor}
              type="button"
              className={styles.pastilla}
              data-activa={orden === o.valor || undefined}
              onClick={() => void useLibraryStore.getState().setOrden(o.valor)}
            >
              {o.etiqueta}
            </button>
          ))}
        </div>
      </div>

      {stats !== null && (
        <div className={styles.resumen}>
          <div>
            <span className={styles.numero}>{cifra(stats.pending)}</span> pendientes
          </div>
          <div>
            <span className={styles.numero}>{cifra(stats.kept + stats.moved)}</span> conservados
          </div>
          <div>
            <span className={styles.numero}>{cifra(stats.rejected)}</span> rechazados
          </div>
          {stats.duplicates > 0 && (
            <div>
              <span className={styles.numero}>{cifra(stats.duplicates)}</span> duplicados
            </div>
          )}
        </div>
      )}

      {analizando && progreso !== null && (
        <div className={styles.analizando}>
          Analizando… quedan {cifra(progreso.pendingAnalysis)}
        </div>
      )}

      <button
        type="button"
        className={styles.ajustes}
        onClick={() => useUiStore.getState().setAjustes(true)}
      >
        <span>Ajustes</span>
        <Kbd>Ctrl+,</Kbd>
      </button>
    </aside>
  );
}
