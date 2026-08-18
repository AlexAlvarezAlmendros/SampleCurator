#!/usr/bin/env python3
"""PostToolUse (Write|Edit) — formatea automáticamente lo que se acaba de escribir.

  · .ts/.tsx/.js/.json/.css  → biome check --write (si está instalado en el checkout)
  · .rs                      → cargo fmt sobre el crate de src-tauri

Nunca bloquea (siempre exit 0): formatear no debe cortar el flujo de trabajo.
"""
import json
import os
import subprocess
import sys

WEB_EXTS = (".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".json", ".css")


def repo_root(path):
    d = os.path.dirname(path)
    while d and d != "/":
        if os.path.basename(d) == "SampleCurator" or os.path.isdir(os.path.join(d, ".git")):
            return d
        d = os.path.dirname(d)
    return None


def main():
    try:
        data = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    fp = ((data.get("tool_input") or {}).get("file_path") or "").replace("\\", "/")
    if "SampleCurator" not in fp:
        sys.exit(0)
    if any(x in fp for x in ("/node_modules/", "/target/", "/dist/", "/.git/")):
        sys.exit(0)

    root = repo_root(fp)
    if not root:
        sys.exit(0)

    if fp.endswith(WEB_EXTS):
        biome = os.path.join(root, "node_modules", ".bin", "biome")
        if os.path.isfile(biome):
            try:
                subprocess.run(
                    [biome, "check", "--write", "--no-errors-on-unmatched", fp],
                    cwd=root, capture_output=True, text=True, timeout=60,
                )
            except Exception:
                pass

    elif fp.endswith(".rs"):
        manifest = os.path.join(root, "src-tauri", "Cargo.toml")
        if os.path.isfile(manifest):
            try:
                subprocess.run(
                    ["cargo", "fmt", "--manifest-path", manifest, "--", fp],
                    cwd=root, capture_output=True, text=True, timeout=60,
                )
            except Exception:
                pass

    sys.exit(0)


main()
