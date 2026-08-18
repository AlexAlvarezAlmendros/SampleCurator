---
name: samplecurator-componente
description: "Use this skill when creating a NEW React component in the SampleCurator frontend (/home/poio/Documentos/GIT/SampleCurator/src). Triggers on 'nuevo componente', 'crea un componente', 'add a component', 'nueva fila/panel/chip/vista'. Scaffolds the folder-per-component convention (Component.tsx + Component.module.css + index.ts) wired to the design tokens, with the keyboard-first, zero-hex, virtualization-safe rules that keep the sample list at 60 fps."
metadata:
  version: 1.0.0
---

# Componente React en SampleCurator

## Estructura

```
src/components/Kbd/              ← primitiva reutilizable
  Kbd.tsx
  Kbd.module.css
  index.ts                       export { Kbd } from "./Kbd";

src/features/library/components/Row/   ← componente de una feature
  Row.tsx
  Row.module.css
  index.ts
```

Primitiva (`src/components/`) si la usan dos features o más; si no, vive dentro de su feature.
`components/` **nunca** importa de `features/`.

## Plantilla

```tsx
// src/features/library/components/Row/Row.tsx
import { memo } from "react";
import styles from "./Row.module.css";

interface RowProps {
  id: number;
  filename: string;
  durationMs: number | null;
  isSelected: boolean;
  isPlaying: boolean;
}

function RowImpl({ id, filename, durationMs, isSelected, isPlaying }: RowProps) {
  return (
    <div
      className={styles.row}
      data-selected={isSelected || undefined}
      data-playing={isPlaying || undefined}
      role="option"
      aria-selected={isSelected}
      data-id={id}
    >
      <span className={styles.name}>{filename}</span>
      <span className={styles.duration}>{formatDuration(durationMs)}</span>
    </div>
  );
}

export const Row = memo(RowImpl);
```

```css
/* Row.module.css */
.row {
  display: grid;
  grid-template-columns: 4px 1fr 56px 64px auto;
  align-items: center;
  gap: 12px;
  height: var(--row-height);
  padding-inline: 8px;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--color-text);
}

.row:hover            { background: var(--color-bg-hover); }
.row[data-selected]   { background: var(--color-bg-selected); }
.row[data-playing]    { box-shadow: inset 2px 0 0 var(--color-text-accent); }
```

## Reglas no negociables

**Color y forma**
- Cero `#hex`, `rgb()`, `hsl()`: solo `var(--color-…)` semánticos o tokens de componente.
  Un hook lo comprueba en cada edición.
- Radios solo `4px`, `6px`, `10px`. Sin sombras salvo en overlays.
- Espaciado múltiplo de 4.

**Rendimiento (la lista es virtualizada: cada fila cuenta)**
- Todo componente que aparezca dentro de la lista va envuelto en `memo` y recibe **solo props
  primitivas**. Nada de objetos ni de callbacks recreados por render: rompen la memoización.
- Los eventos de fila se resuelven por delegación en el contenedor (`data-id` + un `onClick`
  arriba), no con un handler por fila.
- Estado por `data-*` + CSS, no por clases concatenadas en JS.
- Nada de `useEffect` para pintar: si hay que dibujar, es canvas + `requestAnimationFrame`
  fuera de React.
- Selectores de Zustand atómicos: `useStore(s => s.selectedId)`. Nunca devuelvas un objeto nuevo.

**Teclado y accesibilidad**
- Si el componente tiene una acción, esa acción tiene atajo, y el atajo se muestra con `<Kbd>`.
- Los atajos se registran en `src/lib/keymap.ts`, **nunca** con `addEventListener` propio.
- El foco siempre visible (`outline: 2px solid var(--color-focus-ring); outline-offset: -2px`).
- Roles ARIA correctos: la lista es `role="listbox"`, las filas `role="option"` con `aria-selected`.
- Objetivo de ratón ≥ 24 px de alto aunque la fila mida 28.

**Movimiento**
- La selección **no se anima**. Micro-interacciones 80 ms, paneles 140 ms, solo `opacity`
  y `transform`. Respeta `prefers-reduced-motion`.

**Datos**
- El componente no llama a `ipc.ts` directamente: recibe los datos por props o de un hook de la
  feature (`useLibraryPage`, `useTransport`). La lógica vive en el hook, no en el JSX.

## Tests

Se testean los **hooks** y la lógica de selección/keymap con `renderHook`, mockeando
`src/lib/ipc.ts`. No se testean las primitivas visuales.

## Checklist

- [ ] Carpeta con `Componente.tsx` + `.module.css` + `index.ts`
- [ ] Sin colores literales; solo tokens semánticos
- [ ] `memo` + props primitivas si vive dentro de la lista
- [ ] Atajo registrado en `keymap.ts` y visible con `<Kbd>`
- [ ] Foco visible, roles ARIA correctos
- [ ] Sin `invoke()`, sin `console.log`, sin `any`
- [ ] `pnpm typecheck` y `pnpm biome check .` limpios
