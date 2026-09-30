// Checks that the A_Gen6 template of the page (site/index.html) answers every row up to
// degree 4 as the fixture agen6_rows_deg4.txt says; the Rust test `agen6_rows_match_the_page_template`
// checks the engine against the same fixture.
//   node testdata/check_agen6_template.mjs          verify
//   node testdata/check_agen6_template.mjs --write  rewrite the fixture from the template
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const html = readFileSync(join(here, '..', '..', 'site', 'index.html'), 'utf8');
const start = html.indexOf('  agen6: `');
if (start < 0) throw new Error('agen6 template not found in site/index.html');
const body = html.slice(start + '  agen6: `'.length, html.indexOf('`,', start));
const k = 6, maxDeg = 4;
const f = new Function('own', 'bag', 'k', 'deg', 'has', 'nxt', 'prv', 'W', body);
const nxt = (c) => (c === 0 ? 1 : (c % (k - 1)) + 1), prv = (c) => (c <= 1 ? k - 1 : c - 1);
const lines = [];
const cnt = new Array(k).fill(0);
const rec = (c, left, own) => {
  if (c === k) {
    if (cnt.reduce((a, b) => a + b, 0) >= 1) {
      const bag = cnt.slice(), has = (x) => Number.isInteger(x) && x >= 0 && x < k && bag[x] > 0;
      const out = f(own, bag, k, bag.reduce((a, b) => a + b, 0), has, nxt, prv, 0);
      const row = own + '.' + cnt.map((x) => x.toString(16)).join('');
      lines.push(out === null ? `${row} undefined` : `${row} ${out.paint}>${out.target} r${out.rule}`);
    }
    return;
  }
  for (let x = 0; x <= left; x++) { cnt[c] = x; rec(c + 1, left - x, own); }
  cnt[c] = 0;
};
for (let own = 0; own < k; own++) rec(0, maxDeg, own);
lines.sort();
const text = lines.join('\n') + '\n';
const fixture = join(here, 'agen6_rows_deg4.txt');
if (process.argv.includes('--write')) { writeFileSync(fixture, text); console.log(`wrote ${lines.length} rows`); }
else if (readFileSync(fixture, 'utf8').replace(/\r\n/g, '\n') === text) console.log(`page template matches the fixture (${lines.length} rows)`);
else { console.error('MISMATCH between the page template and the fixture'); process.exit(1); }
