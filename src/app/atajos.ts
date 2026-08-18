import { TECLAS_TIPO, useLabelsStore } from "../features/labels/store";
/**
 * La tabla de atajos de SampleCurator. Es también la fuente de la pantalla de ayuda (`?`),
 * así que cada acción que exista aquí se ve documentada en la interfaz automáticamente.
 *
 * El bucle entero es: navegar con ↓/↑ (suena solo), decidir con 1…9 o X, deshacer con Ctrl+Z.
 */
import { filaEn, useLibraryStore } from "../features/library/store";
import { useMetaStore } from "../features/meta/store";
import { usePlayerStore } from "../features/player/store";
import { useTriageStore } from "../features/triage/store";
import { useTrashStore } from "../features/triage/store.papelera";
import * as ipc from "../lib/ipc";
import type { Atajo } from "../lib/keymap";
import { algunaTecla, digitoConAlt, esDigito1a9, tecla, teclaIgnorandoShift } from "../lib/keymap";
import { useUiStore } from "./uiStore";

function idEnFoco(): number | null {
  const lib = useLibraryStore.getState();
  return filaEn(lib, lib.foco)?.id ?? null;
}

/**
 * Atajos que siguen valiendo en modo etiquetado: moverse y escuchar. Etiquetar sin oír el
 * sample no tendría ningún sentido.
 */
const COMPARTIDOS = [
  "abajo",
  "arriba",
  "extender-abajo",
  "extender-arriba",
  "salto-abajo",
  "salto-arriba",
  "inicio",
  "fin",
  "repetir",
  "bucle",
  "atras",
  "adelante",
  "autoplay",
  "ayuda",
];

/**
 * La tabla depende del modo. Como `App` la pide en cada pulsación, cambiar de modo cambia el
 * teclado entero sin tocar el listener — y la pantalla de ayuda, que se genera de aquí,
 * muestra sola las teclas que valen ahora.
 */
export function construirAtajos(): Atajo[] {
  const base = atajosTriaje();
  if (!useLabelsStore.getState().modo) return base;
  return [...base.filter((a) => COMPARTIDOS.includes(a.id)), ...atajosEtiquetado()];
}

function atajosEtiquetado(): Atajo[] {
  const labels = () => useLabelsStore.getState();

  const clases: Atajo[] = TECLAS_TIPO.map(([letra, valor, nombre]) => ({
    id: `etiqueta-${valor}`,
    etiqueta: letra.toUpperCase(),
    descripcion: `Etiquetar como «${nombre}» y avanzar`,
    grupo: "Etiquetado",
    test: tecla(letra),
    ejecutar: () => labels().ponerTipo(valor),
  }));

  return [
    ...clases,
    {
      id: "salir-etiquetado",
      etiqueta: "Esc",
      descripcion: "Salir del modo etiquetado",
      grupo: "Etiquetado",
      enTexto: true,
      test: tecla("escape"),
      ejecutar: () => labels().alternarModo(),
    },
    {
      id: "modo-etiquetado-off",
      etiqueta: "⇧ L",
      descripcion: "Salir del modo etiquetado",
      grupo: "Etiquetado",
      test: tecla("l", { shift: true }),
      ejecutar: () => labels().alternarModo(),
    },
  ];
}

function atajosTriaje(): Atajo[] {
  const lib = () => useLibraryStore.getState();
  const player = () => usePlayerStore.getState();
  const triage = () => useTriageStore.getState();
  const ui = () => useUiStore.getState();

  return [
    // ── navegación ─────────────────────────────────────────────
    {
      id: "abajo",
      etiqueta: "↓",
      descripcion: "Siguiente sample (y suena)",
      grupo: "Navegación",
      test: tecla("arrowdown"),
      ejecutar: () => lib().mover(1),
    },
    {
      id: "arriba",
      etiqueta: "↑",
      descripcion: "Sample anterior (y suena)",
      grupo: "Navegación",
      test: tecla("arrowup"),
      ejecutar: () => lib().mover(-1),
    },
    // Alias estilo vim. Van aparte porque en modo etiquetado la J y la K son otra cosa.
    {
      id: "abajo-vim",
      etiqueta: "J",
      descripcion: "Siguiente sample (alias)",
      grupo: "Navegación",
      test: tecla("j"),
      ejecutar: () => lib().mover(1),
    },
    {
      id: "arriba-vim",
      etiqueta: "K",
      descripcion: "Sample anterior (alias)",
      grupo: "Navegación",
      test: tecla("k"),
      ejecutar: () => lib().mover(-1),
    },
    {
      id: "extender-abajo",
      etiqueta: "⇧ ↓",
      descripcion: "Extender la selección hacia abajo",
      grupo: "Navegación",
      test: tecla("arrowdown", { shift: true }),
      ejecutar: () => lib().mover(1, true),
    },
    {
      id: "extender-arriba",
      etiqueta: "⇧ ↑",
      descripcion: "Extender la selección hacia arriba",
      grupo: "Navegación",
      test: tecla("arrowup", { shift: true }),
      ejecutar: () => lib().mover(-1, true),
    },
    {
      id: "salto-abajo",
      etiqueta: "Av Pág",
      descripcion: "Bajar 10",
      grupo: "Navegación",
      test: tecla("pagedown"),
      ejecutar: () => lib().mover(10),
    },
    {
      id: "salto-arriba",
      etiqueta: "Re Pág",
      descripcion: "Subir 10",
      grupo: "Navegación",
      test: tecla("pageup"),
      ejecutar: () => lib().mover(-10),
    },
    {
      id: "inicio",
      etiqueta: "Inicio",
      descripcion: "Ir al primero",
      grupo: "Navegación",
      test: tecla("home"),
      ejecutar: () => lib().irA(0),
    },
    {
      id: "fin",
      etiqueta: "Fin",
      descripcion: "Ir al último",
      grupo: "Navegación",
      test: tecla("end"),
      ejecutar: () => lib().irA(lib().total - 1),
    },
    {
      id: "seleccionar-todo",
      etiqueta: "Ctrl+A",
      descripcion: "Seleccionar todo lo filtrado",
      grupo: "Navegación",
      test: tecla("a", { ctrl: true }),
      ejecutar: () => lib().seleccionarTodo(),
    },

    // ── escucha ────────────────────────────────────────────────
    {
      id: "repetir",
      etiqueta: "Espacio",
      descripcion: "Repetir desde el principio",
      grupo: "Escucha",
      test: tecla(" "),
      ejecutar: () => {
        const id = idEnFoco();
        if (id !== null) void player().reproducir(id);
      },
    },
    {
      id: "bucle",
      etiqueta: "⇧ Espacio",
      descripcion: "Bucle sí/no",
      grupo: "Escucha",
      test: tecla(" ", { shift: true }),
      ejecutar: () => player().alternarBucle(),
    },
    {
      id: "atras",
      etiqueta: "←",
      descripcion: "Retroceder 0,5 s",
      grupo: "Escucha",
      test: tecla("arrowleft"),
      ejecutar: () => player().saltarRelativo(-0.5),
    },
    {
      id: "adelante",
      etiqueta: "→",
      descripcion: "Avanzar 0,5 s",
      grupo: "Escucha",
      test: tecla("arrowright"),
      ejecutar: () => player().saltarRelativo(0.5),
    },
    {
      id: "silencio",
      etiqueta: "S",
      descripcion: "Silenciar / reanudar",
      grupo: "Escucha",
      test: tecla("s"),
      ejecutar: () => player().alternarSilencio(),
    },
    {
      id: "subir-volumen",
      etiqueta: "+",
      descripcion: "Subir el volumen",
      grupo: "Escucha",
      test: teclaIgnorandoShift(["+", "=", "add"]),
      ejecutar: () => player().ajustarVolumen(0.1),
    },
    {
      id: "bajar-volumen",
      etiqueta: "−",
      descripcion: "Bajar el volumen",
      grupo: "Escucha",
      test: teclaIgnorandoShift(["-", "subtract"]),
      ejecutar: () => player().ajustarVolumen(-0.1),
    },
    {
      id: "autoplay",
      etiqueta: "⇧ A",
      descripcion: "Autoplay al enfocar sí/no",
      grupo: "Escucha",
      test: tecla("a", { shift: true }),
      ejecutar: () => {
        player().alternarAutoplay();
        ui().avisar("info", player().autoplay ? "Autoplay activado" : "Autoplay desactivado");
      },
    },

    // ── decisión ───────────────────────────────────────────────
    {
      id: "destino",
      etiqueta: "1 … 9",
      descripcion: "Enviar al destino N y avanzar",
      grupo: "Decisión",
      test: esDigito1a9,
      ejecutar: (e) => triage().enviarATecla(e.key),
    },
    {
      id: "rechazar",
      etiqueta: "X / Supr",
      descripcion: "Rechazar (a la papelera) y avanzar",
      grupo: "Decisión",
      test: algunaTecla(["x", "delete"]),
      ejecutar: () => triage().rechazar(),
    },
    {
      id: "conservar",
      etiqueta: "Intro",
      descripcion: "Conservar en su sitio y avanzar",
      grupo: "Decisión",
      test: tecla("enter"),
      ejecutar: () => triage().conservar(),
    },
    {
      id: "deshacer",
      etiqueta: "Ctrl+Z",
      descripcion: "Deshacer (devuelve archivo, estado y foco)",
      grupo: "Decisión",
      test: tecla("z", { ctrl: true }),
      ejecutar: () => triage().deshacer(),
    },
    {
      id: "rehacer",
      etiqueta: "Ctrl+⇧+Z",
      descripcion: "Rehacer",
      grupo: "Decisión",
      test: tecla("z", { ctrl: true, shift: true }),
      ejecutar: () => triage().rehacer(),
    },
    {
      id: "valorar",
      etiqueta: "Alt+1…5",
      descripcion: "Poner de una a cinco estrellas (Alt+0 las quita)",
      grupo: "Decisión",
      test: digitoConAlt,
      ejecutar: async (e) => {
        const id = idEnFoco();
        if (id === null) return;
        const estrellas = Number.parseInt(e.key, 10);
        await ipc.valorar(id, estrellas);
        lib().parchear(id, { rating: estrellas });
      },
    },
    {
      id: "favorito",
      etiqueta: "F",
      descripcion: "Marcar como favorito (5 estrellas)",
      grupo: "Decisión",
      test: tecla("f"),
      ejecutar: async () => {
        const id = idEnFoco();
        if (id === null) return;
        const fila = filaEn(lib(), lib().foco);
        const nuevo = fila && fila.rating >= 5 ? 0 : 5;
        await ipc.valorar(id, nuevo);
        lib().parchear(id, { rating: nuevo });
      },
    },

    // ── biblioteca ─────────────────────────────────────────────
    {
      id: "renombrar",
      etiqueta: "F2",
      descripcion: "Renombrar el archivo",
      grupo: "Decisión",
      test: tecla("f2"),
      ejecutar: () => {
        if (idEnFoco() !== null) ui().setRenombrando(true);
      },
    },
    {
      id: "exportar",
      etiqueta: "Ctrl+E",
      descripcion: "Guardar las decisiones en library.json",
      grupo: "Biblioteca",
      test: tecla("e", { ctrl: true }),
      ejecutar: async () => {
        const proyecto = triage().proyecto;
        if (!proyecto) {
          ui().avisar("info", "No hay sesión que exportar");
          return;
        }
        try {
          const ruta = await ipc.exportarDecisiones(proyecto.id);
          ui().avisar("exito", `Decisiones guardadas en ${ruta}`);
        } catch {
          ui().avisar("error", "No se pudieron guardar las decisiones");
        }
      },
    },
    {
      id: "ajustes",
      etiqueta: "Ctrl+,",
      descripcion: "Abrir los ajustes (carpetas, apariencia, escucha e información)",
      grupo: "Biblioteca",
      test: tecla(",", { ctrl: true }),
      ejecutar: () => ui().setAjustes(true),
    },
    {
      id: "tema",
      etiqueta: "T",
      descripcion: "Cambiar entre tema oscuro y claro",
      grupo: "Biblioteca",
      test: tecla("t"),
      ejecutar: () => {
        ui().alternarTema();
        void ipc.settingsSet("tema", useUiStore.getState().tema).catch(() => {});
      },
    },
    {
      id: "buscar",
      etiqueta: "/",
      descripcion: "Buscar",
      grupo: "Biblioteca",
      test: tecla("/"),
      ejecutar: () => ui().setBuscando(true),
    },
    {
      id: "escape",
      etiqueta: "Esc",
      descripcion: "Cerrar búsqueda, ayuda o selección",
      grupo: "Biblioteca",
      enTexto: true,
      test: tecla("escape"),
      ejecutar: () => {
        const u = ui();
        if (useTrashStore.getState().abierta) {
          useTrashStore.getState().cerrar();
          return;
        }
        if (u.ajustesAbiertos) {
          u.setAjustes(false);
          return;
        }
        if (u.ayudaAbierta) {
          u.alternarAyuda();
          return;
        }
        if (u.asistenteAbierto) {
          u.setAsistente(false);
          return;
        }
        if (u.buscando) {
          u.setBuscando(false);
          void lib().setBusqueda("");
          return;
        }
        lib().limpiarSeleccion();
      },
    },
    {
      id: "abrir-origen",
      etiqueta: "O",
      descripcion: "Añadir una carpeta de samples",
      grupo: "Biblioteca",
      test: tecla("o"),
      ejecutar: () => ui().setAsistente(true),
    },
    {
      id: "elegir-destino",
      etiqueta: "D",
      descripcion: "Elegir la carpeta de destino",
      grupo: "Biblioteca",
      test: tecla("d"),
      ejecutar: () => ui().setAsistente(true),
    },
    {
      id: "revelar",
      etiqueta: "Ctrl+R",
      descripcion: "Abrir la carpeta del sample en el explorador",
      grupo: "Biblioteca",
      test: tecla("r", { ctrl: true }),
      ejecutar: async () => {
        const id = idEnFoco();
        if (id === null) return;
        const d = await ipc.detalle(id);
        await ipc.revelarEnElExplorador(d.absPath);
      },
    },
    {
      id: "solo-pendientes",
      etiqueta: "⇧ P",
      descripcion: "Filtrar solo los pendientes",
      grupo: "Biblioteca",
      test: tecla("p", { shift: true }),
      ejecutar: () => lib().setEstado(lib().estado === "pending" ? "all" : "pending"),
    },
    {
      id: "solo-duplicados",
      etiqueta: "⇧ D",
      descripcion: "Filtrar solo los duplicados",
      grupo: "Biblioteca",
      test: tecla("d", { shift: true }),
      ejecutar: () => lib().setEstado(lib().estado === "duplicates" ? "all" : "duplicates"),
    },
    {
      id: "papelera",
      etiqueta: "⇧ X",
      descripcion: "Abrir la papelera: escuchar y restaurar lo rechazado",
      grupo: "Decisión",
      test: tecla("x", { shift: true }),
      ejecutar: () => useTrashStore.getState().abrir(),
    },
    {
      id: "inspector",
      etiqueta: "I",
      descripcion: "Inspector: etiquetas, notas y valoración del sample enfocado",
      grupo: "Biblioteca",
      test: tecla("i"),
      ejecutar: () => useMetaStore.getState().alternarModo(),
    },
    {
      id: "modo-etiquetado",
      etiqueta: "⇧ L",
      descripcion: "Entrar en modo etiquetado (construye el conjunto de evaluación)",
      grupo: "Biblioteca",
      test: tecla("l", { shift: true }),
      ejecutar: () => useLabelsStore.getState().alternarModo(),
    },
    {
      id: "ayuda",
      etiqueta: "?",
      descripcion: "Mostrar u ocultar esta ayuda",
      grupo: "Biblioteca",
      test: tecla("?"),
      ejecutar: () => ui().alternarAyuda(),
    },
    {
      id: "ayuda-h",
      etiqueta: "H",
      descripcion: "Mostrar la ayuda (alias)",
      grupo: "Biblioteca",
      test: tecla("h"),
      ejecutar: () => ui().alternarAyuda(),
    },
  ];
}
