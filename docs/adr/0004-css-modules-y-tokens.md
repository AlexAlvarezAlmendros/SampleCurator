# ADR-0004 — CSS Modules con tokens, sin CSS-in-JS ni framework de utilidades

**Fecha:** 2026-08-18 · **Estado:** aceptada

## Contexto

La UI es muy específica (filas densas, waveforms, chips de destino, teclas visibles) y muy
sensible al rendimiento: la lista repinta con cada pulsación de flecha.

## Decisión

CSS Modules (`Componente.module.css`) junto a cada componente, apoyados en tres capas de tokens
CSS declaradas en `src/styles/tokens.css`.

## Por qué no las alternativas

- **CSS-in-JS en runtime (styled-components, emotion).** Serializa y inyecta estilos durante el
  render. En una lista virtualizada que remonta filas al hacer scroll, eso es trabajo por frame
  que no podemos permitirnos.
- **Tailwind.** Zero-runtime, así que el rendimiento no es el problema; el problema es que este
  proyecto tiene pocos componentes y muy específicos. `Row` tendría 25 clases de utilidad en el
  JSX y la waveform seguiría necesitando CSS propio. Los tokens semánticos como contrato explícito
  (`--color-waveform`) documentan mejor un sistema de diseño de nicho que una cadena de utilidades.
- **Una librería de componentes (shadcn, MUI, Radix Themes).** No hay ni un componente de este
  producto que se parezca a los de una librería genérica. Traería peso y opiniones a cambio de casi
  nada; solo se plantea `radix-ui` a nivel de primitivas sin estilo si aparece un menú o un diálogo
  con requisitos de accesibilidad complejos.

## Consecuencias

- La disciplina de tokens hay que sostenerla: **ningún color literal fuera de `tokens.css`**.
  Un hook de PostToolUse lo revisa en cada edición de CSS.
- El tema claro (Fase 6) se implementa redefiniendo únicamente la capa semántica bajo
  `[data-theme="light"]`, sin tocar un solo componente.
- Los nombres de clase se generan con hash en build: sin colisiones y sin convenciones BEM.
