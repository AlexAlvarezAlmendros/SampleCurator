#!/usr/bin/env python3
"""PostToolUse (Write|Edit) — vigila las convenciones no negociables de SampleCurator.

Devuelve el hallazgo a Claude (exit 2) para que lo corrija en el acto:

  CSS   · ningún color literal fuera de src/styles/tokens.css
  TS    · invoke() solo dentro de src/lib/ipc.ts   · sin console.log   · sin any
  Rust  · sin unwrap()/expect() fuera de tests, main.rs y lib.rs
  Audio · src-tauri/src/audio/graph.rs es código de tiempo real:
          sin locks, sin reservas de memoria, sin I/O, sin panics
"""
import json
import os
import re
import sys

HEX = re.compile(r"#[0-9a-fA-F]{3,8}\b|(?<![-\w])(rgb|rgba|hsl|hsla|oklch)\s*\(")
INVOKE = re.compile(r"\binvoke\s*[(<]")
CONSOLE = re.compile(r"\bconsole\.(log|debug)\s*\(")
ANY = re.compile(r":\s*any\b|\bas\s+any\b")
UNWRAP = re.compile(r"\.unwrap\(\)|\.expect\(")
RT_PROHIBIDO = re.compile(
    r"\.lock\(|RwLock|Mutex|println!|eprintln!|dbg!|log::|Vec::new|vec!\[|String::|format!|Box::new|\.to_vec\(\)|\.collect\(\)"
)


def leer(path):
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            return f.read()
    except Exception:
        return ""


def sin_tests(src):
    """Recorta el módulo de tests para no penalizar unwrap() en tests."""
    i = src.find("#[cfg(test)]")
    return src[:i] if i != -1 else src


def main():
    try:
        data = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    fp = ((data.get("tool_input") or {}).get("file_path") or "").replace("\\", "/")
    if "SampleCurator" not in fp:
        sys.exit(0)
    if any(x in fp for x in ("/node_modules/", "/target/", "/dist/", "/.git/", "/docs/", "/.claude/")):
        sys.exit(0)
    if not os.path.isfile(fp):
        sys.exit(0)

    src = leer(fp)
    if not src:
        sys.exit(0)

    fallos = []
    avisos = []

    # --- CSS: tokens ---
    if fp.endswith(".css") and not fp.endswith("styles/tokens.css"):
        m = HEX.search(src)
        if m:
            fallos.append(
                f"color literal `{m.group(0)}` en un CSS que no es tokens.css. "
                "Usa un token semántico (var(--color-…)). Los primitivos y los literales "
                "viven solo en src/styles/tokens.css (docs/DESIGN_SYSTEM.md §2)."
            )

    # --- TypeScript ---
    if fp.endswith((".ts", ".tsx")) and "/src/" in fp and not fp.endswith(".test.ts"):
        if INVOKE.search(src) and not fp.endswith(("lib/ipc.ts", "bindings.ts")):
            fallos.append(
                "llamada a invoke() fuera de src/lib/ipc.ts. Todo el IPC pasa por ese módulo "
                "(CLAUDE.md § Convenciones TypeScript)."
            )
        if CONSOLE.search(src):
            fallos.append("console.log en src/. Usa el logger de lib/ o quítalo.")
        if ANY.search(src):
            fallos.append("`any` en TypeScript. Los tipos del backend se generan en src/bindings.ts.")

    # --- Rust ---
    if fp.endswith(".rs") and "/src-tauri/" in fp:
        cuerpo = sin_tests(src)
        es_arranque = fp.endswith(("main.rs", "lib.rs")) or "/tests/" in fp or "/benches/" in fp
        if not es_arranque and UNWRAP.search(cuerpo):
            fallos.append(
                "unwrap()/expect() fuera de tests y del arranque. Propaga con AppError "
                "(CLAUDE.md § Convenciones Rust)."
            )
        if "/src/audio/" in fp:
            m = RT_PROHIBIDO.search(cuerpo)
            if m and fp.endswith("graph.rs"):
                # graph.rs ES el callback: aquí sí se corta.
                fallos.append(
                    f"`{m.group(0)}` en graph.rs — código de tiempo real. Dentro del callback de "
                    "audio no se reserva memoria, no se bloquea, no se hace I/O y no se panica. "
                    "Comunica por ring buffer SPSC y prepara los buffers en el hilo de control "
                    "(CLAUDE.md § Reglas de tiempo real)."
                )
            elif m:
                # El resto de audio/ es hilo de control, donde Mutex y alloc son legítimos:
                # se avisa sin bloquear, porque bloquear aquí sería llorar al lobo.
                avisos.append(
                    f"`{m.group(0)}` en {os.path.basename(fp)}: legítimo en el hilo de control, "
                    "pero comprueba que ese código no acabe llamándose desde el callback de cpal."
                )

    if avisos and not fallos:
        print(json.dumps({
            "hookSpecificOutput": {
                "hookEventName": "PostToolUse",
                "additionalContext": "SampleCurator — " + " · ".join(avisos),
            }
        }))
        sys.exit(0)

    if fallos:
        sys.stderr.write(
            "⚠️ Convenciones de SampleCurator en " + os.path.basename(fp) + ":\n  - "
            + "\n  - ".join(fallos)
            + "\nCorrígelo antes de seguir.\n"
        )
        sys.exit(2)

    sys.exit(0)


main()
