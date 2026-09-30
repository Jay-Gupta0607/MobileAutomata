// Sends every fixture of testdata/fixtures to the WebAssembly build (site/game.wasm) and compares the
// answer with the expected NAME.json, the file the native engine is held to by `cargo test --test fixtures`.
// It also checks that docs/index.html (what GitHub Pages serves) embeds this very build.
//   node testdata/check_wasm_parity.mjs          (after `sh workbench/build.sh`)
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const workbench = join(here, '..', '..');
const wasmBytes = readFileSync(join(workbench, 'site', 'game.wasm'));
const { instance } = await WebAssembly.instantiate(wasmBytes, {});
const x = instance.exports;
const enc = new TextEncoder(), dec = new TextDecoder();

// the same calling convention as the page (site/index.html, makeCaller), but the raw JSON text is kept
function run(text) {
  const bytes = enc.encode(text);
  const ptr = x.alloc(bytes.length);
  new Uint8Array(x.memory.buffer, ptr, bytes.length).set(bytes);
  const out = x.run(ptr, bytes.length);
  const n = new DataView(x.memory.buffer).getUint32(out, true);
  const s = dec.decode(new Uint8Array(x.memory.buffer, out + 4, n));
  x.free_result(out);
  x.dealloc(ptr, bytes.length);
  return s;
}

const dir = join(here, 'fixtures');
const names = readdirSync(dir).filter((f) => f.endsWith('.req')).map((f) => f.slice(0, -4)).sort();
let bad = 0;
for (const name of names) {
  const request = readFileSync(join(dir, name + '.req'), 'utf8');
  const expected = readFileSync(join(dir, name + '.json'), 'utf8').replace(/\r\n/g, '\n').trimEnd();
  const t = Date.now();
  const got = run(request);
  const ms = Date.now() - t;
  if (got === expected) console.log(`ok    ${name} (${ms} ms)`);
  else { bad++; console.log(`FAIL  ${name}\n  expected ${expected.slice(0, 200)}\n  got      ${got.slice(0, 200)}`); }
}
if (names.length < 20) { console.log(`only ${names.length} fixtures found`); bad++; }

// the Pages file must embed exactly this build
const b64 = wasmBytes.toString('base64');
const docs = readFileSync(join(workbench, '..', 'docs', 'index.html'), 'utf8');
if (docs.includes(`const WASM_B64 = "${b64}";`)) console.log('ok    docs/index.html embeds site/game.wasm');
else { bad++; console.log('FAIL  docs/index.html does not embed the current site/game.wasm (run sh workbench/build.sh)'); }

console.log(bad === 0 ? `all ${names.length} fixtures match in WebAssembly` : `${bad} problem(s)`);
process.exit(bad === 0 ? 0 : 1);
