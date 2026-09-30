// Checks the page (site/index.html): that its script is valid JavaScript at all, and the pure logic in it
// (the block between `<page-logic>` and `</page-logic>`): the paper-model status, the colour names and the
// rule descriptions.
// It is also held to the engine: for every paper-model fixture that has a trace, the page's verdict
// must be the engine's (status, reason and the three indicators).
//   node testdata/check_page_logic.mjs
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import vm from 'node:vm';

const here = dirname(fileURLToPath(import.meta.url));
const html = readFileSync(join(here, '..', '..', 'site', 'index.html'), 'utf8').replace(/\r\n/g, '\n');
// 0. the whole script must compile: a stray backtick inside one of the code templates (they are template
// literals) ends the string early and the page then does not load at all, which no test of the pieces notices
// (docs/index.html is what GitHub Pages serves: the same page with the engine inlined, built from site/index.html)
for (const [name, text] of [['site/index.html', html], ['docs/index.html', readFileSync(join(here, '..', '..', '..', 'docs', 'index.html'), 'utf8').replace(/\r\n/g, '\n')]]) {
  const scripts = [...text.matchAll(/<script>([\s\S]*?)<\/script>/g)].map((m) => m[1]);
  if (scripts.length === 0) throw new Error(`no <script> in ${name}`);
  for (const [i, src] of scripts.entries()) {
    try { new vm.Script(src, { filename: `${name} script ${i + 1}` }); }
    catch (e) { console.log(`FAIL  the script of ${name} does not compile: ${e.message}`); process.exit(1); }
  }
}
const a = html.indexOf('// <page-logic>'), b = html.indexOf('// </page-logic>');
if (a < 0 || b < a) throw new Error('the page-logic block was not found in site/index.html');
const { paperStatus, colourLabel, colourHint, COLOUR_NEED, AGEN6_COLOURS, AGEN6_RULES } = new Function(html.slice(a, b) + '\nreturn { paperStatus, colourLabel, colourHint, COLOUR_NEED, AGEN6_COLOURS, AGEN6_RULES };')();

let bad = 0;
const check = (ok, what) => { if (!ok) { bad++; console.log('FAIL  ' + what); } };
const same = (got, want, what) => check(JSON.stringify(got) === JSON.stringify(want), `${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);

// 1. the three parts and the outcome, case by case
const v = (o) => ({ visited: 5, n: 5, cur: 0, start: 0, stopped: false, cycle: false, ...o });
same(paperStatus(v({ visited: 2 })).outcome, 'running', 'still exploring');
same(paperStatus(v({ stopped: true })).outcome, 'explores', 'stopped on the start with everything visited');
same(paperStatus(v({ stopped: true, cur: 3 })).outcome, 'stopped_off_start', 'everything visited, stopped elsewhere');
same(paperStatus(v({ stopped: true, visited: 3 })).outcome, 'stopped_early', 'stopped on the start with vertices unvisited');
same(paperStatus(v({ stopped: true, visited: 3, cur: 2 })).outcome, 'stopped_early', 'unvisited vertices win over the wrong place, as in the engine');
same(paperStatus(v({ cycle: true })).outcome, 'never_stops', 'the position repeats although everything was visited');
same(paperStatus(v({ cycle: true, visited: 2 })).outcome, 'never_stops', 'the position repeats with vertices unvisited');
same(paperStatus(v({ stopped: true, cycle: true })).outcome, 'explores', 'a stop is the end, whatever the history');
const s = paperStatus(v({ visited: 3, cur: 2 }));
same([s.covered, s.onStart, s.termination], [false, false, 'running'], 'the three parts while running');
same(paperStatus(v({ stopped: true })).termination, 'stopped', 'termination: stopped');
same(paperStatus(v({ cycle: true })).termination, 'never', 'termination: never');

// 2. the colour names and the rule descriptions
// the template's constants (INIT = 0, PATH = 1, ...) are what the engine agreement check pins to the engine's numbering
const tpl = html.slice(html.indexOf('  agen6: `'));
const consts = tpl.match(/const INIT = (\d), PATH = (\d), FIN = (\d), HEAD1 = (\d), HEAD2 = (\d), NEIGH = (\d);/);
check(consts !== null, 'the template declares its colour constants on one line');
if (consts) {
  const names = ['init', 'path', 'fin', 'head1', 'head2', 'neigh'], byNumber = [];
  names.forEach((nm, i) => { byNumber[+consts[i + 1]] = nm; });
  same(AGEN6_COLOURS, byNumber, 'the page names colour n as the template does');
}
same([colourLabel(0, true), colourLabel(3, true), colourLabel(5, true)], ['init', 'head1', 'neigh'], 'named colours');
same([colourLabel(0, false), colourLabel(4, false)], ['white', 'colour 4'], 'numbered colours');
same(AGEN6_RULES.length, 12, 'rule descriptions for 1 to 11 (index 0 unused)');
check(AGEN6_RULES.slice(1).every((t) => typeof t === 'string' && t.length > 10), 'every rule has a description');
check(AGEN6_RULES[0] === null, 'index 0 is unused');

// 2b. the hint next to the colours picker
same(colourHint('agen6', 6), { text: 'A_Gen6 needs 6 colours', warn: false }, 'A_Gen6 with six colours');
same(colourHint('agen6', 3), { text: 'A_Gen6 needs 6 colours (now 3)', warn: true }, 'A_Gen6 with too few colours');
same(colourHint('flipsweep4', 4), { text: 'flipsweep4 needs at least 4 colours', warn: false }, 'a minimum that is met');
same(colourHint('flipsweep5', 4), { text: 'flipsweep5 needs at least 5 colours (now 4)', warn: true }, 'a minimum that is not met');
same([colourHint('flipsweep5', 6).warn, colourHint('chase3', 2).warn, colourHint('eat3', 3).warn], [false, true, false], 'the boundary is k = n');
same([colourHint('sigma', 2), colourHint('none', 5), colourHint('sweep', 3), colourHint(null, 6), colourHint('chasewhite', 2)], [null, null, null, null, null], 'rules that work with any number have no hint');
same(Object.keys(COLOUR_NEED).sort(), ['agen6', 'chase3', 'eat3', 'flipsweep4', 'flipsweep4b', 'flipsweep5', 'flipsweep5d'], 'the rules with a stated need');

// 3. the page's verdict against the engine's, on every paper-model fixture with a trace
const dir = join(here, 'fixtures');
let compared = 0;
for (const f of readdirSync(dir).filter((x) => x.endsWith('.json'))) {
  const ans = JSON.parse(readFileSync(join(dir, f), 'utf8'));
  const trace = ans.trace;
  if (ans.model !== 'paper' || !trace || !['explores', 'fails'].includes(ans.status)) continue;
  const req = readFileSync(join(dir, f.replace(/\.json$/, '.req')), 'utf8');
  const g = req.split('\n').find((l) => l.startsWith('graph ')).split(/\s+/);
  const n = +g[1], start = +g[2];
  const stopped = trace.stopped_at !== null;
  const st = paperStatus({ visited: n - trace.unvisited.length, n, cur: stopped ? trace.stopped_at : -1, start, stopped, cycle: trace.repeat_at !== null });
  const want = ans.status === 'explores' ? 'explores' : ans.reason;
  same(st.outcome, want, `${f}: outcome`);
  if (ans.indicators) {
    same([st.covered, st.termination === 'stopped', stopped && st.onStart], [ans.indicators.visited_all, ans.indicators.stopped, ans.indicators.stopped_at_start], `${f}: the three indicators`);
  }
  compared++;
}
check(compared >= 8, `only ${compared} engine verdicts were compared`);

console.log(bad === 0 ? `page logic ok (${compared} engine verdicts compared)` : `${bad} problem(s)`);
process.exitCode = bad === 0 ? 0 : 1; // not process.exit(): it can cut off piped output (and trips a libuv assertion on Windows)
