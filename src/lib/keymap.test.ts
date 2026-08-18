import { describe, expect, it, vi } from "vitest";
import type { Atajo } from "./keymap";
import { algunaTecla, esDigito1a9, registrarKeymap, tecla } from "./keymap";

function evento(key: string, mods: Partial<KeyboardEvent> = {}): KeyboardEvent {
  return new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...mods });
}

describe("comparación de teclas", () => {
  it("exige los modificadores exactos", () => {
    const soloZ = tecla("z");
    expect(soloZ(evento("z"))).toBe(true);
    expect(soloZ(evento("z", { ctrlKey: true }))).toBe(false);
  });

  it("no confunde ctrl+z con ctrl+shift+z", () => {
    const deshacer = tecla("z", { ctrl: true });
    const rehacer = tecla("z", { ctrl: true, shift: true });
    const e = evento("z", { ctrlKey: true, shiftKey: true });
    expect(deshacer(e)).toBe(false);
    expect(rehacer(e)).toBe(true);
  });

  it("acepta varias teclas para la misma acción", () => {
    const bajar = algunaTecla(["arrowdown", "j"]);
    expect(bajar(evento("ArrowDown"))).toBe(true);
    expect(bajar(evento("j"))).toBe(true);
    expect(bajar(evento("k"))).toBe(false);
  });

  it("los dígitos de destino no se disparan con modificadores", () => {
    expect(esDigito1a9(evento("3"))).toBe(true);
    expect(esDigito1a9(evento("0"))).toBe(false);
    expect(esDigito1a9(evento("3", { ctrlKey: true }))).toBe(false);
    expect(esDigito1a9(evento("3", { altKey: true }))).toBe(false);
  });
});

describe("listener global", () => {
  function atajo(id: string, key: string, ejecutar: () => void, enTexto = false): Atajo {
    return {
      id,
      etiqueta: key,
      descripcion: id,
      grupo: "test",
      test: tecla(key),
      ejecutar,
      ...(enTexto ? { enTexto: true } : {}),
    };
  }

  it("ejecuta el primero que coincide y corta el evento", () => {
    const a = vi.fn();
    const b = vi.fn();
    const soltar = registrarKeymap(() => [atajo("a", "x", a), atajo("b", "x", b)]);
    const e = evento("x");
    window.dispatchEvent(e);
    expect(a).toHaveBeenCalledOnce();
    expect(b).not.toHaveBeenCalled();
    expect(e.defaultPrevented).toBe(true);
    soltar();
  });

  it("no dispara atajos normales mientras se escribe en un campo", () => {
    const normal = vi.fn();
    const enTexto = vi.fn();
    const soltar = registrarKeymap(() => [
      atajo("normal", "x", normal),
      atajo("escape", "escape", enTexto, true),
    ]);

    const campo = document.createElement("input");
    document.body.appendChild(campo);
    campo.focus();

    campo.dispatchEvent(evento("x"));
    expect(normal).not.toHaveBeenCalled();

    campo.dispatchEvent(evento("Escape"));
    expect(enTexto).toHaveBeenCalledOnce();

    campo.remove();
    soltar();
  });

  it("deja de escuchar al soltarlo", () => {
    const f = vi.fn();
    const soltar = registrarKeymap(() => [atajo("a", "x", f)]);
    soltar();
    window.dispatchEvent(evento("x"));
    expect(f).not.toHaveBeenCalled();
  });
});
