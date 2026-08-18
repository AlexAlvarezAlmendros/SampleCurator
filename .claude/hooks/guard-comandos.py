#!/usr/bin/env python3
"""PreToolUse (Bash) — guardarraíles de comandos en SampleCurator.

1. Bloquea borrados recursivos peligrosos (rm -rf con rutas amplias o fuera del repo).
2. Bloquea borrar/mover a mano archivos de audio: eso lo hace fileops con journal y papelera.
3. Bloquea npm/yarn: el gestor del proyecto es pnpm.
4. Avisa si se compila Rust sin --manifest-path desde la raíz.
"""
import json
import os
import re
import sys

PROJECT = "SampleCurator"
AUDIO_EXT = (".wav", ".aiff", ".aif", ".flac", ".mp3", ".ogg", ".m4a")

# rm -rf sobre algo que no es un subdirectorio claramente temporal del repo
RM_PELIGROSO = re.compile(r"\brm\s+(-[a-zA-Z]*[rf][a-zA-Z]*\s+)+(/|~|\$HOME|\*|\.\s*$|\.\./)")
RM_CUALQUIERA = re.compile(r"\brm\s+")
NPM = re.compile(r"^\s*(npm|yarn)\s+(i|install|add|run|remove)\b", re.M)


def deny(msg):
    sys.stderr.write(msg + "\n")
    sys.exit(2)


def main():
    try:
        data = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    if PROJECT not in (data.get("cwd") or ""):
        sys.exit(0)

    cmd = ((data.get("tool_input") or {}).get("command") or "")
    low = cmd.lower()

    if RM_PELIGROSO.search(cmd):
        deny(
            "⛔ Bloqueado: borrado recursivo sobre una ruta amplia o fuera del repo. "
            "Si de verdad hace falta, pídeselo al usuario y que lo ejecute él."
        )

    if RM_CUALQUIERA.search(cmd) and any(e in low for e in AUDIO_EXT):
        deny(
            "⛔ Bloqueado: no se borran archivos de audio con `rm`. En SampleCurator los "
            "descartes van a la papelera gestionada mediante fileops, con journal y undo "
            "(ver CLAUDE.md § Seguridad de los datos del usuario)."
        )

    if NPM.search(cmd):
        deny("⛔ Bloqueado: el gestor de paquetes del proyecto es pnpm. Usa `pnpm …`.")

    if re.search(r"^\s*cargo\s+(build|test|clippy|fmt|bench)\b", cmd, re.M) \
       and "--manifest-path" not in cmd and "src-tauri" not in cmd:
        print(json.dumps({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "additionalContext": (
                    "Recuerda: el crate de Rust vive en src-tauri/. Ejecuta cargo con "
                    "`--manifest-path src-tauri/Cargo.toml` desde la raíz del repo."
                ),
            }
        }))

    sys.exit(0)


main()
