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
const { paperStatus, colourLabel, colourHint, COLOUR_NEED, PAPER_ALGOS, ruleTag, ruleText, algoOfCode, article, stepsDue, RANDOM_FAMILIES, randomGraph, springLayout, layoutArea } = new Function(html.slice(a, b) + '\nreturn { paperStatus, colourLabel, colourHint, COLOUR_NEED, PAPER_ALGOS, ruleTag, ruleText, algoOfCode, article, stepsDue, RANDOM_FAMILIES, randomGraph, springLayout, layoutArea };')();

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

// 2. the colour names, the rule numbers and the rule descriptions of every algorithm of the papers
for (const key of ['agen6', 'atc3', 'ac4', 'aus5']) {
  const open = `  ${key}: \``, start = html.indexOf(open), tpl = html.slice(start, html.indexOf('`,', start));
  // the template's constants (INIT = 0, PATH = 1, ...) are what the engine agreement check pins to the engine's numbering
  const consts = tpl.match(/\nconst ([A-Z0-9]+ = \d(?:, [A-Z0-9]+ = \d)*);/);
  check(consts !== null, `${key}: the template declares its colour constants on one line`);
  if (consts) {
    const byNumber = [];
    for (const m of consts[1].matchAll(/([A-Z0-9]+) = (\d)/g)) byNumber[+m[2]] = m[1].toLowerCase();
    same(PAPER_ALGOS[key].colours, byNumber, `${key}: the page names colour n as the template does`);
  }
  // the rule numbers the template gives are exactly 1 to N, and each has a description
  const rules = PAPER_ALGOS[key].rules, N = rules.length - 1;
  same([...new Set([...tpl.matchAll(/rule: (\d+)/g)].map((m) => +m[1]))].sort((x, y) => x - y), Array.from({ length: N }, (_, i) => i + 1), `${key}: the template numbers its rules 1 to ${N}`);
  check(rules[0] === null && rules.slice(1).every((t) => typeof t === 'string' && t.length > 10), `${key}: every rule has a description, and index 0 is unused`);
  check(algoOfCode(tpl.slice(open.length)) === key, `${key}: the template is recognised by its first comment line`);
}
same(Object.fromEntries(Object.entries(PAPER_ALGOS).map(([key, a]) => [key, a.rules.length - 1])), { agen6: 11, atc3: 11, ac4: 14, aus5: 19 }, "the number of rules of each algorithm is the paper's (D1-D11, C1-C14, U1-U19)");
same(Object.fromEntries(Object.entries(PAPER_ALGOS).map(([key, a]) => [key, a.colours.length])), { agen6: 6, atc3: 3, ac4: 4, aus5: 5 }, 'and so is the number of colours');
same(Object.keys(PAPER_ALGOS).every((key) => COLOUR_NEED[key].exact && COLOUR_NEED[key].n === PAPER_ALGOS[key].colours.length), true, "COLOUR_NEED states each algorithm's colour count");
same([colourLabel(0, 'agen6'), colourLabel(3, 'agen6'), colourLabel(5, 'agen6')], ['init', 'head1', 'neigh'], 'named colours');
same([colourLabel(1, 'atc3'), colourLabel(2, 'ac4'), colourLabel(4, 'aus5')], ['l0', 'front', 'head'], 'named colours of the other algorithms');
same([colourLabel(0, null), colourLabel(4, null), colourLabel(0, 'sigma'), colourLabel(2, undefined)], ['white', 'colour 4', 'white', 'colour 2'], 'numbered colours');
same([colourLabel(3, 'atc3'), colourLabel(5, 'ac4')], ['unused', 'unused'], "a colour beyond the algorithm's own is unused");
same(colourLabel(1, 'constructor'), 'colour 1', 'a name that is not an algorithm is not looked up as one');
same([ruleTag('agen6', 5), ruleTag('atc3', 5), ruleTag('ac4', 9), ruleTag('aus5', 2), ruleTag(null, 7), ruleTag('sigma', 7)], ['r5', 'D5', 'C9', 'U2', 'r7', 'r7'], "rule tags: the paper's letter, r for A_Gen6 and for numbers given by code");
same([ruleText('atc3', 9).length > 10, ruleText('atc3', 12), ruleText('aus5', 19) !== null, ruleText(null, 1), ruleText('agen6', 0)], [true, null, true, null, null], 'rule texts, and none for a number the algorithm does not have');
same([algoOfCode('// A_Gen6 (x)\nreturn null;'), algoOfCode('  // A_TC3 (y)'), algoOfCode('// A_C4 (z)'), algoOfCode('// A_US5 (w)'), algoOfCode('// my own rule'), algoOfCode(''), algoOfCode('// A_Gen7'), algoOfCode('return null; // A_TC3')], ['agen6', 'atc3', 'ac4', 'aus5', null, null, null, null], 'code is an algorithm only by its first comment line');

same(['init', 'unused', 'Init', 'white', 'l0', 'head1', 'neigh', 'fin', 'front', 'path', 'colour 3'].map(article), ['an', 'an', 'an', 'a', 'a', 'a', 'a', 'a', 'a', 'a', 'a'], 'a or an before a colour name');

// 2a. the steps a play tick owes (a late tick catches up; never fewer than one, never more than the cap)
same([stepsDue(0, 130, 2000), stepsDue(50, 130, 2000), stepsDue(130, 130, 2000), stepsDue(260, 130, 2000), stepsDue(1000, 130, 2000), stepsDue(60000, 130, 2000), stepsDue(1e9, 130, 2000)], [1, 1, 1, 2, 7, 461, 2000], 'steps due by the clock');
same([stepsDue(-5, 130, 2000), stepsDue(NaN, 130, 2000), stepsDue(Infinity, 130, 2000), stepsDue(500, 0, 2000), stepsDue(500, NaN, 2000)], [1, 1, 1, 1, 1], 'a clock that makes no sense still gives one step');

// 2b. the hint next to the colours picker
same(colourHint('agen6', 6), { text: 'A_Gen6 needs 6 colours', warn: false }, 'A_Gen6 with six colours');
same(colourHint('atc3', 3), { text: 'A_TC3 needs 3 colours', warn: false }, 'A_TC3 with three colours');
same(colourHint('atc3', 5), { text: 'A_TC3 needs 3 colours (the other 2 unused)', warn: false }, 'more colours than the algorithm needs are unused, not a warning');
same(colourHint('aus5', 4), { text: 'A_US5 needs 5 colours (now 4)', warn: true }, 'A_US5 with too few colours');
same(colourHint('ac4', 4), { text: 'A_C4 needs 4 colours', warn: false }, 'A_C4 with four colours');
same(colourHint('agen6', 3), { text: 'A_Gen6 needs 6 colours (now 3)', warn: true }, 'A_Gen6 with too few colours');
same(colourHint('flipsweep4', 4), { text: 'flipsweep4 needs at least 4 colours', warn: false }, 'a minimum that is met');
same(colourHint('flipsweep5', 4), { text: 'flipsweep5 needs at least 5 colours (now 4)', warn: true }, 'a minimum that is not met');
same([colourHint('flipsweep5', 6).warn, colourHint('chase3', 2).warn, colourHint('eat3', 3).warn], [false, true, false], 'the boundary is k = n');
same([colourHint('sigma', 2), colourHint('none', 5), colourHint('sweep', 3), colourHint(null, 6), colourHint('chasewhite', 2)], [null, null, null, null, null], 'rules that work with any number have no hint');
same(Object.keys(COLOUR_NEED).sort(), ['ac4', 'agen6', 'atc3', 'aus5', 'chase3', 'eat3', 'flipsweep4', 'flipsweep4b', 'flipsweep5', 'flipsweep5d'], 'the rules with a stated need');

// 2c. the random graph option: every size gives a connected graph the engine accepts, and a layout that can be read
const seeded = (seed) => () => { seed = (seed + 0x6D2B79F5) | 0; let t = Math.imul(seed ^ (seed >>> 15), 1 | seed); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; // mulberry32
const degrees = (n, edges) => { const d = new Array(n).fill(0); for (const [u, v] of edges) { d[u]++; d[v]++; } return d; };
const reaches = (n, edges) => { const seen = new Set([0]), todo = [0]; while (todo.length) { const x = todo.pop(); for (const [u, v] of edges) { const y = u === x ? v : v === x ? u : -1; if (y >= 0 && !seen.has(y)) { seen.add(y); todo.push(y); } } } return seen.size === n; };
let trees = 0, withCycles = 0, texts = new Set();
for (let n = 2; n <= 32; n++) for (let seed = 1; seed <= 25; seed++) {
  const g = randomGraph(n, seeded(n * 1000 + seed)), what = `n=${n} seed=${seed}`;
  const [tn, ts, list] = g.text.split(' ');
  const parsed = list.split(',').map((e) => e.split('-').map(Number));
  check(+tn === n && ts === '0', `${what}: the text starts with the vertex count and start 0`);
  same(parsed, g.edges, `${what}: the text is the edge list`);
  check(parsed.every(([u, v]) => u < v && v < n && u >= 0), `${what}: edges join two different vertices of the graph, smaller first`);
  check(new Set(parsed.map((e) => e.join('-'))).size === parsed.length, `${what}: no edge twice`);
  check(reaches(n, parsed), `${what}: connected (the engine refuses a graph that is not)`);
  check(Math.max(...degrees(n, parsed)) <= 15, `${what}: no vertex has more than 15 neighbours (the engine refuses it)`);
  if (parsed.length === n - 1) trees++; else withCycles++;
  texts.add(g.text);
  if (seed <= 3) {
    const pos = springLayout(n, g.edges, 900, 560, seeded(seed));
    check(pos.length === n && pos.every(([x, y]) => Number.isFinite(x) && Number.isFinite(y) && x >= 0 && x <= 900 && y >= 0 && y <= 560), `${what}: every vertex is placed inside the box`);
    let closest = Infinity; for (let i = 0; i < n; i++) for (let j = i + 1; j < n; j++) closest = Math.min(closest, Math.hypot(pos[i][0] - pos[j][0], pos[i][1] - pos[j][1]));
    check(closest >= 40, `${what}: no two vertices drawn on top of each other (closest ${closest.toFixed(1)}, a vertex is 36 wide)`);
    same(springLayout(n, g.edges, 900, 560, seeded(seed)), pos, `${what}: the same randomness gives the same layout`);
  }
}
check(trees > 100 && withCycles > 100, `both trees (${trees}) and graphs with cycles (${withCycles}) turn up`);
check(texts.size > 700, `the graphs differ from one another (${texts.size} different of 775)`);
same(randomGraph(2, seeded(1)).text, '2 0 0-1', 'two vertices: the one edge');
// the extremes of the randomness: always the first choice, always the last, and a neighbour limit that has to be respected
for (const [name, rnd] of [['always 0', () => 0], ['always the largest', () => 1 - 2 ** -53]]) {
  const g = randomGraph(32, rnd);
  check(reaches(32, g.edges) && Math.max(...degrees(32, g.edges)) <= 15, `${name}: still connected, still at most 15 neighbours`);
}
same(Math.max(...degrees(32, randomGraph(32, () => 0).edges)), 15, 'always the first choice would make a star: it stops at 15 neighbours');
for (const bad of [0, 1, 33, -3, 2.5, NaN, Infinity, '5', null, undefined]) {
  let message = null; try { randomGraph(bad, Math.random); } catch (e) { message = e.message; }
  check(message === 'A random graph has 2 to 32 vertices.', `${String(bad)} vertices is refused with a message (got ${message})`);
}

// the layout also has to work in the smaller areas that layoutArea can give (down to 250 units high)
for (const h of [250, 435]) for (const n of [2, 9, 16, 24, 32]) for (let seed = 1; seed <= 10; seed++) {
  const g = randomGraph(n, seeded(n * 77 + seed)), pos = springLayout(n, g.edges, 900, h, seeded(seed)), what = `area 900x${h}, n=${n}, seed=${seed}`;
  check(pos.every(([x, y]) => x >= 0 && x <= 900 && y >= 0 && y <= h), `${what}: inside the area`);
  let closest = Infinity; for (let i = 0; i < n; i++) for (let j = i + 1; j < n; j++) closest = Math.min(closest, Math.hypot(pos[i][0] - pos[j][0], pos[i][1] - pos[j][1]));
  check(closest >= 40, `${what}: no two vertices on top of each other (closest ${closest.toFixed(1)})`);
}

// 2c-ii. the families of random graphs: every graph has exactly n vertices, is connected, repeats no edge, has at most 15 neighbours at
// a vertex and is in its family, as a block decomposition written here, apart from the generator, says; the families nest
const blocksOf = (n, edges) => { // Tarjan: the blocks of a connected graph, as lists of edges
  const adj = Array.from({ length: n }, () => []), disc = new Array(n).fill(0), low = new Array(n).fill(0), stack = [], out = [];
  for (const [u, v] of edges) { adj[u].push(v); adj[v].push(u); }
  let time = 0;
  const dfs = (u, parent) => {
    disc[u] = low[u] = ++time;
    for (const v of adj[u]) {
      if (disc[v] === 0) {
        stack.push([u, v]); dfs(v, u); low[u] = Math.min(low[u], low[v]);
        if (low[v] >= disc[u]) { const block = []; for (;;) { const e = stack.pop(); block.push(e); if (e[0] === u && e[1] === v) break; } out.push(block); }
      } else if (v !== parent && disc[v] < disc[u]) { stack.push([u, v]); low[u] = Math.min(low[u], disc[v]); }
    }
  };
  dfs(0, -1);
  return out;
};
const vertsOf = (block) => [...new Set(block.flat())];
const hasEdgeIn = (block, a, b) => block.some(([x, y]) => (x === a && y === b) || (x === b && y === a));
const isBridgeBlock = (block) => block.length === 1;
const isCliqueBlock = (block) => { const v = vertsOf(block).length; return block.length === v * (v - 1) / 2; };
const isTriangleFreeBlock = (block) => { const v = vertsOf(block); for (const a of v) for (const b of v) for (const c of v) if (a < b && b < c && hasEdgeIn(block, a, b) && hasEdgeIn(block, b, c) && hasEdgeIn(block, a, c)) return false; return true; };
const isCycleBlock = (block) => { const v = vertsOf(block); return v.length >= 3 && block.length === v.length && v.every((x) => block.filter(([a, b]) => a === x || b === x).length === 2); };
const isCompleteBipartiteBlock = (block) => { // two-colour the block, then every pair across the sides must be an edge
  const v = vertsOf(block), side = new Map([[v[0], 0]]), todo = [v[0]];
  while (todo.length) {
    const x = todo.pop();
    for (const [a, b] of block) {
      const y = a === x ? b : b === x ? a : -1;
      if (y < 0) continue;
      if (!side.has(y)) { side.set(y, 1 - side.get(x)); todo.push(y); } else if (side.get(y) === side.get(x)) return false;
    }
  }
  const p = [...side.values()].filter((s) => s === 0).length;
  return block.length === p * (v.length - p);
};
const inFamily = {
  any: () => true,
  tree: (n, e) => e.length === n - 1,
  cycle: (n, e) => e.length === n && n >= 3 && degrees(n, e).every((d) => d === 2),
  cactus: (n, e) => blocksOf(n, e).every((b) => isBridgeBlock(b) || isCycleBlock(b)),
  cb: (n, e) => blocksOf(n, e).every((b) => isBridgeBlock(b) || isCycleBlock(b) || isCompleteBipartiteBlock(b)),
  ktf: (n, e) => blocksOf(n, e).every((b) => isCliqueBlock(b) || isTriangleFreeBlock(b)),
};
const wider = { tree: ['cactus', 'cb', 'ktf', 'any'], cycle: ['cactus', 'cb', 'ktf', 'any'], cactus: ['cb', 'ktf', 'any'], cb: ['ktf', 'any'], ktf: ['any'], any: [] };
same(Object.keys(RANDOM_FAMILIES).sort(), Object.keys(inFamily).sort(), 'the families the generator offers are the ones tested here');
let familyGraphs = 0;
for (const family of Object.keys(RANDOM_FAMILIES)) {
  for (let n = RANDOM_FAMILIES[family].min; n <= 32; n++) for (let seed = 1; seed <= 12; seed++) {
    const g = randomGraph(n, seeded(n * 131 + seed * 7 + family.length * 7919), family), what = `${family} n=${n} seed=${seed}`;
    const [tn, ts, list] = g.text.split(' '), parsed = list ? list.split(',').map((e) => e.split('-').map(Number)) : [];
    check(+tn === n && ts === '0', `${what}: the text has n vertices and start 0`);
    check(parsed.length === g.edges.length && parsed.every(([u, v], i) => u === g.edges[i][0] && v === g.edges[i][1]), `${what}: the text is the edge list`);
    check(new Set(parsed.map((e) => e.join('-'))).size === parsed.length && parsed.every(([u, v]) => u >= 0 && u < v && v < n), `${what}: edges distinct, smaller end first, inside the graph`);
    check(reaches(n, parsed), `${what}: connected (the engine refuses a graph that is not)`);
    check(Math.max(...degrees(n, parsed)) <= 15, `${what}: at most 15 neighbours at a vertex`);
    check(inFamily[family](n, parsed), `${what}: in its family`);
    for (const w of wider[family]) check(inFamily[w](n, parsed), `${what}: and so in the wider family ${w}`);
    familyGraphs++;
  }
}
check(familyGraphs > 1000, `${familyGraphs} family graphs checked`);
// what turns up across many big graphs: each family uses its kinds of block, and the families really differ
const variety = (family, pred, runs = 60) => { let hits = 0; for (let seed = 1; seed <= runs; seed++) { const g = randomGraph(32, seeded(seed * 17 + 3), family); if (pred(blocksOf(32, g.edges), g.edges)) hits++; } return hits; };
check(variety('ktf', (bs) => bs.some((b) => isCliqueBlock(b) && vertsOf(b).length >= 4)) > 0, 'clique / triangle-free blocks: some graph has a clique of four or more vertices');
check(variety('ktf', (bs) => bs.some((b) => isCliqueBlock(b) && vertsOf(b).length === 3)) > 0, 'clique / triangle-free blocks: some graph has a triangle');
check(variety('ktf', (bs) => bs.some((b) => vertsOf(b).length >= 4 && isTriangleFreeBlock(b) && !isCycleBlock(b) && !isCompleteBipartiteBlock(b))) > 0, 'clique / triangle-free blocks: some triangle-free block is neither a cycle nor complete bipartite');
check(variety('cb', (bs) => bs.some((b) => isCompleteBipartiteBlock(b) && !isCycleBlock(b) && !isBridgeBlock(b))) > 0, 'cycle / K(p,q) blocks: some K(p,q) with a side of three or more');
check(variety('cb', (bs) => bs.some(isCycleBlock) && bs.some(isBridgeBlock)) > 0, 'cycle / K(p,q) blocks: cycles and bridges turn up');
check(variety('cactus', (bs) => bs.some(isCycleBlock) && bs.some(isBridgeBlock)) > 0, 'cactus: cycles and bridges turn up');
check(variety('tree', (bs) => bs.every(isBridgeBlock)) === 60, 'tree: every block is a bridge');
check(variety('ktf', (bs, e) => !inFamily.cb(32, e)) > 0, 'the clique / triangle-free family is bigger than the cycle / K(p,q) family');
check(variety('cb', (bs, e) => !inFamily.cactus(32, e)) > 0, 'the cycle / K(p,q) family is bigger than the cacti');
check(variety('cactus', (bs, e) => !inFamily.tree(32, e)) > 0, 'the cacti are more than trees');
check(variety('any', (bs, e) => !inFamily.ktf(32, e)) >= 15, 'a good share (a quarter or more) of the big random graphs of no family are outside clique / triangle-free blocks, which is why that choice exists');
// refusals, and the menu of the page
const refusal = (fn) => { try { fn(); return null; } catch (e) { return e.message; } };
same(refusal(() => randomGraph(2, Math.random, 'cycle')), 'A random simple cycle has 3 to 32 vertices.', 'a cycle needs three vertices');
same(refusal(() => randomGraph(33, Math.random, 'tree')), 'A random tree has 2 to 32 vertices.', 'a size above 32 is refused for a family');
same(refusal(() => randomGraph(33, Math.random)), 'A random graph has 2 to 32 vertices.', 'the default family keeps its message');
same([refusal(() => randomGraph(5, Math.random, 'nope')), refusal(() => randomGraph(5, Math.random, 'constructor'))], ['Unknown family of graphs: nope.', 'Unknown family of graphs: constructor.'], 'a name that is not a family is refused, not looked up as one');
same(refusal(() => randomGraph(2, seeded(1), 'tree')), null, 'two vertices make a tree');
const menu = html.match(/<select id="rfam"[^>]*>([\s\S]*?)<\/select>/)[1];
same([...menu.matchAll(/<option value="(\w+)"/g)].map((m) => m[1]).sort(), Object.keys(RANDOM_FAMILIES).sort(), 'the menu offers exactly the families the generator has');
for (const [key, f] of Object.entries(RANDOM_FAMILIES)) check(new RegExp(`<option value="${key}"[^>]*>[^<]*\\(${f.from}\\)</option>`).test(menu), `${key}: the menu names the first algorithm guaranteed, ${f.from}`);

// 2d. where a random graph may go: clear of the legend (top right) and the hint (bottom left), which are drawn over the canvas
const legend = { left: 700, right: 990, top: 10, bottom: 61 }, hint = { left: 12, right: 370, top: 580, bottom: 630 };
same(layoutArea([]), { x: 50, y: 40, w: 900, h: 560 }, 'nothing covered: the usual margins');
same(layoutArea([legend]), { x: 50, y: 103, w: 900, h: 497 }, 'the legend pushes the top edge down, a halo and a little more below it');
same(layoutArea([hint]), { x: 50, y: 40, w: 900, h: 498 }, 'the hint pushes the bottom edge up');
same(layoutArea([legend, hint]), { x: 50, y: 103, w: 900, h: 435 }, 'both');
same(layoutArea([hint, legend]), layoutArea([legend, hint]), 'in either order');
same(layoutArea([{ ...legend, left: 1010, right: 1300 }, { ...hint, left: -300, right: -10 }]), layoutArea([]), 'boxes beside the canvas (it is letterboxed in a wide window) cost nothing');
same(layoutArea([{ left: 700, right: 990, top: -90, bottom: -40 }]), layoutArea([]), 'a box above the canvas costs nothing');
same(layoutArea([{ left: 0, right: 1000, top: 0, bottom: 300 }, { left: 0, right: 1000, top: 340, bottom: 640 }]), layoutArea([]), 'if that would leave too little room, the margins alone');
for (const [what, cover] of [['legend', [legend]], ['hint', [hint]], ['both', [legend, hint]], ['tall legend and hint', [{ ...legend, bottom: 150 }, { ...hint, top: 520 }]]]) {
  const area = layoutArea(cover);
  check(area.h >= 250 && area.y >= 40 && area.y + area.h <= 600 && area.x === 50 && area.x + area.w === 950, `${what}: the area is inside the margins and tall enough for 32 vertices`);
  for (const r of cover) check(!(r.bottom + 34 > area.y && r.top < area.y) && !(r.top - 34 < area.y + area.h && r.bottom > area.y + area.h), `${what}: a vertex on the area's edge, with its halo, clears the box`);
}

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
