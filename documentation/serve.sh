#!/usr/bin/env bash
# Serve the GET documentation site locally.
#
#   ./serve.sh          -> http://localhost:8000
#   ./serve.sh 9000     -> http://localhost:9000
#
# The site is plain static HTML with no build step and no external assets, so
# opening index.html straight from the filesystem also works. A server is only
# nicer because relative links and the browser's history behave normally.

set -euo pipefail

port="${1:-8000}"
cd "$(dirname "$0")"

echo "GET documentation → http://localhost:${port}/"
echo "Ctrl-C to stop."
exec python3 -m http.server "$port"
