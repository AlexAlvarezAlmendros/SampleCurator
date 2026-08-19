#!/usr/bin/env bash
# Publica una versión: sube el número en los tres sitios, comitea, etiqueta y empuja.
# A partir de ahí manda `.github/workflows/release.yml`, que compila, firma y publica.
#
#   scripts/publicar.sh 0.3.0
set -euo pipefail

version="${1:-}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "uso: scripts/publicar.sh X.Y.Z" >&2
  exit 1
fi

raiz="$(cd "$(dirname "$0")/.." && pwd)"
cd "$raiz"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "hay cambios sin comitear: publica desde un árbol limpio" >&2
  exit 1
fi
if [[ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]]; then
  echo "publica desde main" >&2
  exit 1
fi
if git rev-parse "v$version" >/dev/null 2>&1; then
  echo "la etiqueta v$version ya existe" >&2
  exit 1
fi

# Los tres sitios donde vive el número. Si se descuadran, el actualizador compara mal.
python3 - "$version" <<'PY'
import json, pathlib, re, sys
v = sys.argv[1]
for ruta in ("package.json", "src-tauri/tauri.conf.json"):
    p = pathlib.Path(ruta)
    d = json.loads(p.read_text())
    d["version"] = v
    p.write_text(json.dumps(d, indent=2, ensure_ascii=False) + "\n")
p = pathlib.Path("src-tauri/Cargo.toml")
p.write_text(re.sub(r'^version = "[^"]+"', f'version = "{v}"', p.read_text(), count=1, flags=re.M))
PY

cargo update --manifest-path src-tauri/Cargo.toml -p samplecurator --quiet 2>/dev/null || true

git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "publicar la versión $version"
git tag -a "v$version" -m "SampleCurator $version"
git push origin main "v$version"

echo
echo "Etiqueta v$version empujada. El workflow compila, firma y publica:"
echo "  https://github.com/AlexAlvarezAlmendros/SampleCurator/actions/workflows/release.yml"
