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

// the page's own presets that bring one of the algorithms of the papers must explore in this engine, and have a position for every vertex
{
  const page = readFileSync(join(workbench, 'site', 'index.html'), 'utf8').replace(/\r\n/g, '\n');
  const a = page.indexOf('const PRESETS = {'), b = page.indexOf('\n', page.indexOf('PRESETS.agen6sketch1'));
  const PRESETS = new Function(page.slice(a, b) + '\nreturn PRESETS;')();
  let n = 0;
  for (const [name, pr] of Object.entries(PRESETS).filter(([, p]) => ['agen6', 'atc3', 'ac4', 'aus5'].includes(p.def))) {
    const nodes = +pr.text.split(/\s+/)[0];
    const got = JSON.parse(run(`cmd exact\nk ${pr.k}\ngraph ${pr.text}\ndefault ${pr.def}\ncap 2000000\n`));
    const ok = got.status === 'explores' && got.trace.stopped_at === +pr.text.split(/\s+/)[1] && (!pr.coords || pr.coords.length === nodes);
    if (ok) console.log(`ok    preset ${name} explores (${got.positions} positions)`);
    else { bad++; console.log(`FAIL  preset ${name}: ${got.status}, ${pr.coords ? pr.coords.length : 'no'} coordinates for ${nodes} vertices`); }
    n++;
  }
  if (n < 10) { bad++; console.log(`only ${n} presets of the papers' algorithms found`); }
}

// the colour counts the page's hint states must be the engine's: one colour fewer and the rule has no answer (A_Gen6 is an error)
{
  const page = readFileSync(join(workbench, 'site', 'index.html'), 'utf8').replace(/\r\n/g, '\n');
  const a = page.indexOf('const COLOUR_NEED = {'), b = page.indexOf('};', a) + 2;
  const COLOUR_NEED = new Function(page.slice(a, b) + '\nreturn COLOUR_NEED;')();
  for (const [rule, need] of Object.entries(COLOUR_NEED)) {
    const at = (k) => JSON.parse(run(`cmd answer\nk ${k}\nrow 0.1\ndefault ${rule}\n`));
    const fewer = at(need.n - 1), enough = at(need.n);
    const ok = (fewer.status === 'error' || fewer.defined === false) && enough.status === 'ok' && enough.defined === true;
    if (ok) console.log(`ok    ${rule} needs ${need.n} colours, as the page says`);
    else { bad++; console.log(`FAIL  ${rule}: the page says ${need.n} colours, the engine answers ${JSON.stringify(fewer)} with ${need.n - 1} and ${JSON.stringify(enough)} with ${need.n}`); }
  }
}

// the Pages file must embed exactly this build
const b64 = wasmBytes.toString('base64');
const docs = readFileSync(join(workbench, '..', 'docs', 'index.html'), 'utf8');
if (docs.includes(`const WASM_B64 = "${b64}";`)) console.log('ok    docs/index.html embeds site/game.wasm');
else { bad++; console.log('FAIL  docs/index.html does not embed the current site/game.wasm (run sh workbench/build.sh)'); }

console.log(bad === 0 ? `all ${names.length} fixtures match in WebAssembly` : `${bad} problem(s)`);
process.exitCode = bad === 0 ? 0 : 1; // not process.exit(): it can cut off piped output (and trips a libuv assertion on Windows)
