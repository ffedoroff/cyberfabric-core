#!/usr/bin/env bash
set -euo pipefail

PORT="${1:-8899}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${ROOT_DIR:-${SCRIPT_DIR}}"

if [[ ! -f "${ROOT_DIR}/rg/openapi.yaml" ]]; then
  echo "Expected file not found: ${ROOT_DIR}/rg/openapi.yaml" >&2
  exit 1
fi

echo "Serving directory: ${ROOT_DIR}"
echo "Starting local server on http://127.0.0.1:${PORT}"
echo "OpenAPI UI:   http://127.0.0.1:${PORT}/rg/openapi.html"
echo "OpenAPI YAML: http://127.0.0.1:${PORT}/rg/openapi.yaml"

exec python3 -c "
import http.server, functools, sys

class NoCacheHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cache-Control', 'no-store, no-cache, must-revalidate, max-age=0')
        super().end_headers()

handler = functools.partial(NoCacheHandler, directory='${ROOT_DIR}')
httpd = http.server.HTTPServer(('127.0.0.1', ${PORT}), handler)
httpd.serve_forever()
"
