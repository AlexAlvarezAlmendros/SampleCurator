import { Kbd } from "../../../../components/Kbd";
import { useUpdaterStore } from "../../store";
import styles from "./AvisoActualizacion.module.css";

/**
 * Aviso de versión nueva: vive en la barra lateral, no interrumpe nada y se puede descartar.
 * Actualizar es una decisión del usuario, no del programa — aquí no se instala nada solo.
 */
export function AvisoActualizacion() {
  const info = useUpdaterStore((s) => s.info);
  const estado = useUpdaterStore((s) => s.estado);
  const descartado = useUpdaterStore((s) => s.descartado);
  const descargado = useUpdaterStore((s) => s.descargado);
  const total = useUpdaterStore((s) => s.total);

  if (!info || descartado) return null;

  const porcentaje = total > 0 ? Math.round((descargado / total) * 100) : null;
  const ocupado = estado === "descargando" || estado === "instalando";

  const texto =
    estado === "instalando"
      ? "Instalando y reiniciando…"
      : estado === "descargando"
        ? porcentaje === null
          ? "Descargando…"
          : `Descargando ${porcentaje} %`
        : info.canInstall
          ? `Actualizar a ${info.version}`
          : `Descargar ${info.version}`;

  return (
    <div className={styles.aviso}>
      <div className={styles.titulo}>
        <span>{`Versión ${info.version} disponible`}</span>
        {!ocupado && (
          <button
            type="button"
            className={styles.cerrar}
            onClick={() => useUpdaterStore.getState().descartar()}
            title="Ocultar hasta el próximo arranque"
            aria-label="Ocultar el aviso de actualización"
          >
            ×
          </button>
        )}
      </div>

      <button
        type="button"
        className={styles.accion}
        onClick={() => void useUpdaterStore.getState().instalar()}
        disabled={ocupado}
      >
        <span>{texto}</span>
        {!ocupado && <Kbd>U</Kbd>}
      </button>

      {estado === "descargando" && (
        <div className={styles.barra}>
          <div
            className={styles.relleno}
            style={porcentaje === null ? undefined : { width: `${porcentaje}%` }}
            data-indeterminada={porcentaje === null}
          />
        </div>
      )}

      {!info.canInstall && estado === "disponible" && (
        <p className={styles.nota}>
          Instalaste por paquete del sistema: se abre la página de descargas.
        </p>
      )}
    </div>
  );
}
