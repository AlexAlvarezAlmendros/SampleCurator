#!/usr/bin/env bash
# Captura la interfaz construida a un tamaño concreto, sin abrir la app.
#
# Sirve `dist/` en un puerto suelto y la fotografía con Chrome en modo headless. El WebView
# no tiene puente IPC, así que la lista sale vacía y aparece un aviso de error: da igual, esto
# es para mirar el LAYOUT, que es donde se cuelan los recortes.
#
#   ./scripts/captura.sh                    → 1362x861 (la ventana por defecto)
#   ./scripts/captura.sh 940 560 chico.png  → tamaño mínimo soportado
#
# El motivo de que exista: un alto fijo en el transporte estuvo recortando la barra inferior
# durante dos versiones, y solo se vio cuando alguien mandó una captura. Ahora se comprueba.
set -euo pipefail

ANCHO="${1:-1362}"
ALTO="${2:-861}"
SALIDA="${3:-/tmp/samplecurator-$ANCHO x$ALTO.png}"
SALIDA="${SALIDA// /}"
PUERTO="${PORT:-8${RANDOM:0:3}}"

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[ -f "$raiz/dist/index.html" ] || { echo "Falta dist/: ejecuta antes 'pnpm build'"; exit 1; }

python3 -m http.server "$PUERTO" --directory "$raiz/dist" >/dev/null 2>&1 &
servidor=$!
trap 'kill "$servidor" 2>/dev/null || true' EXIT
sleep 1

google-chrome --headless=new --disable-gpu --no-sandbox --hide-scrollbars --incognito \
  --virtual-time-budget=5000 --window-size="$ANCHO,$ALTO" \
  --screenshot="$SALIDA" "http://localhost:$PUERTO/?v=$(date +%s)" >/dev/null 2>&1

echo "$SALIDA"
