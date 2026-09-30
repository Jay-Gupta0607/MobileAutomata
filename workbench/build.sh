#!/bin/sh
# Build the WebAssembly engine and the two pages:
#   site/index.html + site/game.wasm   (serve the directory over HTTP)
#   site/standalone.html               (the engine inlined; opens from a file:// URL)
#   ../docs/index.html                 (the same file: what GitHub Pages serves)
set -e
cd "$(dirname "$0")"
( cd game-wasm && cargo build --release --lib --target wasm32-unknown-unknown )  # --lib: the command-line tool in src/bin is native only
cp game-wasm/target/wasm32-unknown-unknown/release/game_wasm.wasm site/game.wasm
python3 - <<'PY'
import base64
html = open('site/index.html').read()
b64 = base64.b64encode(open('site/game.wasm', 'rb').read()).decode()
marker = 'const WASM_B64 = null;'
assert html.count(marker) == 1
open('site/standalone.html', 'w').write(html.replace(marker, 'const WASM_B64 = "%s";' % b64))
open('../docs/index.html', 'w').write(open('site/standalone.html').read())
print('site/standalone.html and ../docs/index.html written, %d KB' % (len(html) // 1024 + len(b64) // 1024))
PY
