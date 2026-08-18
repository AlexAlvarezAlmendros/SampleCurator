#!/usr/bin/env python3
"""PreToolUse (Bash) — bloquea commits y PRs con atribución a la IA en SampleCurator.

Regla global del usuario: nada de "Co-Authored-By: Claude", "Generated with Claude"
ni noreply@anthropic.com en mensajes de commit ni en cuerpos de PR.
"""
import json
import sys

BANNED = (
    "co-authored-by: claude",
    "generated with claude",
    "generated with [claude code]",
    "noreply@anthropic.com",
)


def main():
    try:
        data = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    if "SampleCurator" not in (data.get("cwd") or ""):
        sys.exit(0)

    command = ((data.get("tool_input") or {}).get("command") or "")
    is_publish = "git commit" in command or "gh pr create" in command or "gh pr edit" in command

    if is_publish and any(b in command.lower() for b in BANNED):
        sys.stderr.write(
            "⛔ Bloqueado: los commits y PRs de SampleCurator no llevan atribución a la IA. "
            "Reescribe el mensaje sin 'Co-Authored-By: Claude' ni 'Generated with Claude'.\n"
        )
        sys.exit(2)

    sys.exit(0)


main()
