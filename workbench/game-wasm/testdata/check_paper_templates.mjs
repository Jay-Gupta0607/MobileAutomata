// Checks that the template of each algorithm of the papers in the page (site/index.html: agen6, atc3, ac4, aus5) answers every
// row up to degree 4 as its fixture NAME_rows_deg4.txt says; the Rust test `paper_algorithms_match_the_page_templates` checks the
// engine against the same fixtures.  The templates are the page's own transcription of each paper's rules, written apart from
// the engine's, so an agreement of the two is a check of both.
//   node testdata/check_paper_templates.mjs          verify
//   node testdata/check_paper_templates.mjs --write  rewrite the fixtures from the templates
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const html = readFileSync(join(here, '..', '..', 'site', 'index.html'), 'utf8');
const maxDeg = 4;
let bad = 0;
for (const [name, k] of [['agen6', 6], ['atc3', 3], ['ac4', 4], ['aus5', 5]]) {
  const open = `  ${name}: \``;
  const start = html.indexOf(open);
  if (start < 0) throw new Error(`${name} template not found in site/index.html`);
  const body = html.slice(start + open.length, html.indexOf('`,', start));
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
  const fixture = join(here, `${name}_rows_deg4.txt`);
  if (process.argv.includes('--write')) { writeFileSync(fixture, text); console.log(`${name}: wrote ${lines.length} rows`); }
  else if (readFileSync(fixture, 'utf8').replace(/\r\n/g, '\n') === text) console.log(`${name}: the page template matches the fixture (${lines.length} rows)`);
  else { bad++; console.error(`${name}: MISMATCH between the page template and the fixture`); }
}
process.exitCode = bad === 0 ? 0 : 1; // not process.exit(): it can cut off piped output (and trips a libuv assertion on Windows)
