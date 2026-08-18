#!/usr/bin/env python3
"""SessionStart — recuerda en qué fase está SampleCurator al abrir la sesión."""
import json
import os
import re
import sys


def main():
    try:
        data = json.load(sys.stdin)
    except Exception:
        data = {}

    cwd = data.get("cwd") or os.getcwd()
    if "SampleCurator" not in cwd:
        sys.exit(0)

    root = cwd.split("SampleCurator")[0] + "SampleCurator"
    roadmap = os.path.join(root, "docs", "planning", "ROADMAP.md")
    if not os.path.isfile(roadmap):
        sys.exit(0)

    with open(roadmap, "r", encoding="utf-8", errors="ignore") as f:
        texto = f.read()

    m = re.search(r"## Foco actual\s*\n+(.+?)\n##", texto, re.S)
    foco = m.group(1).strip() if m else "sin foco declarado"

    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": (
                "SampleCurator — estado del roadmap (docs/planning/ROADMAP.md):\n\n"
                + foco
                + "\n\nToda petición de desarrollo empieza aplicando la skill `samplecurator-plan`."
            ),
        }
    }))
    sys.exit(0)


main()
