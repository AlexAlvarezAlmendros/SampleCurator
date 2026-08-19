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
#
# Se toca SOLO la línea de la versión, con una sustitución quirúrgica. Releer y volcar el JSON
# entero reformatea el resto del fichero y deja el CI en rojo por formato: pasó una vez.
python3 - "$version" <<'FIN'
import pathlib, re, sys

v = sys.argv[1]
for ruta in ("package.json", "src-tauri/tauri.conf.json"):
    p = pathlib.Path(ruta)
    texto, n = re.subn(r'("version":\s*)"[^"]+"', rf'\g<1>"{v}"', p.read_text(), count=1)
    assert n == 1, f"no encontre la version en {ruta}"
    p.write_text(texto)

p = pathlib.Path("src-tauri/Cargo.toml")
texto, n = re.subn(r'^version = "[^"]+"', f'version = "{v}"', p.read_text(), count=1, flags=re.M)
assert n == 1, "no encontre la version en Cargo.toml"
p.write_text(texto)
FIN

# Y se comprueba, en vez de confiar: el CI rechaza cualquier desajuste de formato.
pnpm biome check package.json src-tauri/tauri.conf.json

cargo update --manifest-path src-tauri/Cargo.toml -p samplecurator --quiet 2>/dev/null || true

git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "publicar la versión $version"
git tag -a "v$version" -m "SampleCurator $version"
git push origin main "v$version"

echo
echo "Etiqueta v$version empujada. El workflow compila, firma y publica:"
echo "  https://github.com/AlexAlvarezAlmendros/SampleCurator/actions/workflows/release.yml"
