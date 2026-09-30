// A headless check of the page's Play tab: the real script of site/index.html runs in Node against the real
// WebAssembly engine (site/game.wasm), with a stand-in for the browser's DOM, and its state machine is driven the way
// the buttons drive it (stepOnce, undo, loadReplay).  What is checked is what only manual browser runs covered before:
// when a play ends (classic and paper model), the banners and indicators it renders, rule numbers, colour names,
// the "does a rule stop" cache, undo after a stop, and the replay of an execution the engine found.
//   node testdata/check_page_play.mjs        (after `sh workbench/build.sh`)
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import vm from 'node:vm';

const here = dirname(fileURLToPath(import.meta.url));
const site = join(here, '..', '..', 'site');
const html = readFileSync(join(site, 'index.html'), 'utf8').replace(/\r\n/g, '\n');
let src = html.match(/<script>([\s\S]*?)<\/script>/)[1];

// test-only hooks, added to a copy of the source: a promise for when the page has started, and its internals
const head = '(async function () {', tail = /resetPlay\(\); render\(\);\n\}\)\(\);\s*$/;
if (!src.includes(head) || !tail.test(src)) throw new Error('the page script no longer has the shape this test expects');
src = src.replace(head, 'globalThis.__ready = ' + head).replace(tail, `resetPlay(); render();
globalThis.__page = { state, loadGraphText, resetPlay, stepOnce, undo, loadReplay, render, save, paperMode, namedColours, cname, usesStop, engine, requestText, compileAlgo, ALGO_TEMPLATES, playPaperStatus };
})();`);

// a stand-in for an element: remembers what is set on it, answers every method with itself
const elements = new Map();
function element(sel) {
  if (elements.has(sel)) return elements.get(sel);
  const props = Object.create(null);
  const el = new Proxy(function () {}, {
    get(_, k) {
      if (typeof k === 'symbol') return undefined;
      if (k in props) return props[k];
      if (['value', 'innerHTML', 'textContent', 'innerText'].includes(k)) return '';
      if (['disabled', 'checked', 'open', 'hidden'].includes(k)) return false;
      if (k === 'length') return 0;
      if (k === 'options' || k === 'selectedOptions') return [];
      if (k === 'querySelectorAll') return () => [];
      if (k === 'querySelector') return (s) => element(sel + ' ' + s);
      return props[k] = (...a) => el;
    },
    set(_, k, v) { props[k] = v; return true; },
    apply() { return el; },
  });
  elements.set(sel, el);
  return el;
}
const document = { querySelector: (s) => element(s), querySelectorAll: () => [], activeElement: null, addEventListener() {} };
class Worker { constructor() {} postMessage() {} terminate() {} }
const sandbox = {
  document, Worker, console, setInterval, clearInterval, setTimeout, clearTimeout, Blob, URL, TextEncoder, TextDecoder,
  localStorage: { getItem: () => null, setItem() {} },
  fetch: async () => { const b = readFileSync(join(site, 'game.wasm')); return { ok: true, arrayBuffer: async () => b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) }; },
  alert() {}, confirm: () => true,
};
sandbox.globalThis = sandbox;
vm.createContext(sandbox);
vm.runInContext(src, sandbox, { filename: 'site/index.html' });
await sandbox.__ready;
const P = sandbox.__page;

let bad = 0;
const check = (ok, what) => { if (!ok) { bad++; console.log('FAIL  ' + what); } };
const same = (got, want, what) => check(JSON.stringify(got) === JSON.stringify(want), `${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
const panel = () => P.state.play ? (P.render(), String(element('#playstatus').innerHTML)) : '';
const setup = (o) => { // graph, colours, rule
  Object.assign(P.state, { k: o.k, def: o.def || 'none', table: new Map(o.table || []), algoSrc: o.algoSrc || '', algo: null, algoError: null });
  P.loadGraphText(o.graph); P.save();
};
const run = (max = 200) => { // Auto step until the play is over; remember when every vertex was first visited
  const p = P.state.play; let n = 0, allAt = null;
  while (!p.done && p.pending && n < max) { P.stepOnce(true); n++; if (allAt === null && p.vis.size === P.state.nodes.length) allAt = { step: n, over: !!p.done }; }
  return { steps: n, allAt };
};
const wrong = [['0.10', { paint: 1, target: 0 }], ['0.01', { paint: 0, target: -2 }]];

// 1. the classic game ends as soon as every vertex is visited, with no indicators
setup({ k: 5, def: 'sigma', graph: '4 0 0-1,0-2,1-3' });
same(P.paperMode(), false, 'sigma* is not the paper model');
let r = run(); const classic = panel();
same([P.state.play.done.kind, r.steps], ['explored', 4], 'classic sigma* on the tree ends when everything is visited');
check(classic.includes('Explored: all 4 vertices visited') && !classic.includes('class="pills"'), 'classic banner, no indicators');

// 2. A_Gen6 (paper model): the play does not end when the last vertex is reached; it ends at the stop, after 24 steps
setup({ k: 6, def: 'agen6', graph: '4 0 0-1,0-2,1-3' });
same(P.paperMode(), true, 'A_Gen6 is the paper model');
r = run(); const explored = panel(), p = P.state.play;
same([p.done.kind, r.steps], ['stopped', 24], 'A_Gen6 on the tree stops after 24 steps');
same(r.allAt, { step: 13, over: false }, 'every vertex is first visited at step 13 and the play goes on');
same(P.playPaperStatus(p).outcome, 'explores', 'and it explores in this play');
check(p.pending === null, 'nothing is left pending after the stop (the panel used to keep the stop action)');
check(explored.includes('Explored in this play (paper model)') && !explored.includes('paints'), 'the banner replaces the pending action');
const text = explored.replace(/<[^>]+>/g, ''); // what a reader, or a screen reader, gets
check(['coverage: 4 of 4 visited', 'return: stopped on the start vertex', 'termination: stopped'].every((t) => text.includes(t)), `three indicators, readable as text: ${text.slice(0, 160)}`);
same(p.steps.slice(0, 5).map((s) => s.rule), [3, 4, 2, 5, 7], 'the rule numbers of the paper, as in the hand trace');
same(p.steps.at(-1).rule, 9, 'the last step is rule 9');
same(p.steps.at(-1).target, -2, 'and it is a stop');

// 3. undo after a stop brings the play back, and stepping again ends it the same way
P.undo();
check(p.done === null && p.pending !== null && p.steps.length === 23, 'undo after the stop reopens the play');
P.stepOnce(true);
same([p.done && p.done.kind, p.steps.length], ['stopped', 24], 'stepping again stops again');

// 4. the three ways to fail, and a rule that never stops
const cases = [
  ['wrong stop', { k: 2, table: wrong, graph: '2 0 0-1' }, 'stopped', 'stopped_off_start', 'Stopped away from the start in this play'],
  ['early stop', { k: 2, table: [['0.10', { paint: 1, target: -2 }]], graph: '2 0 0-1' }, 'stopped', 'stopped_early', 'Stopped early in this play'],
  ['return and stop', { k: 2, table: [['0.10', { paint: 1, target: 0 }], ['0.01', { paint: 0, target: 1 }], ['1.10', { paint: 1, target: -2 }]], graph: '2 0 0-1' }, 'stopped', 'explores', 'Explored in this play'],
  ['never stops', { k: 3, table: [['0.100', { paint: 1, target: 0 }], ['0.010', { paint: 0, target: 1 }], ['1.100', { paint: 1, target: 0 }], ['2.100', { paint: 2, target: -2 }]], graph: '2 0 0-1' }, 'confined', 'never_stops', 'Never stops, under these choices'],
];
for (const [name, o, kind, outcome, text] of cases) {
  setup(o); P.render(); run();
  const pl = P.state.play, shown = panel();
  same([pl.done && pl.done.kind, P.playPaperStatus(pl).outcome], [kind, outcome], `${name}: how the play ends`);
  check(shown.includes(text), `${name}: the banner says "${text}"`);
  check(shown.replace(/<[^>]+>/g, '').includes('termination: '), `${name}: the indicators are shown`);
}
setup(cases[1][1]); run(); same(P.state.play.done.unvisited, [1], 'an early stop names the unvisited vertex');

// 5. "does a rule stop" is cached, and save() (called after every table edit in the page) clears it
setup({ k: 2, graph: '2 0 0-1', table: [['0.10', { paint: 1, target: 0 }]] });
same(P.usesStop(), false, 'no stop row: the classic game');
P.state.table.set('0.01', { paint: 0, target: -2 });
same(P.usesStop(), false, 'the answer is cached until save()');
P.save();
same(P.usesStop(), true, 'after save() the stop row is seen');
P.state.table.delete('0.01'); P.save();
same(P.usesStop(), false, 'and its removal too');

// 6. a play that the engine's own analysis found can be replayed step by step
setup({ k: 2, table: wrong, graph: '2 0 0-1' });
const found = P.engine(P.requestText('exact', { cap: 1000000 }));
same([found.status, found.reason], ['fails', 'stopped_off_start'], 'the engine finds the wrong stop');
P.loadReplay(found.trace); run();
same([P.state.play.done.kind, P.playPaperStatus(P.state.play).outcome], ['stopped', 'stopped_off_start'], 'the replay ends in the same place');

// 7. colour names: only when A_Gen6 is the rule that answers
setup({ k: 6, def: 'agen6', graph: '3 0 0-1,1-2' });
same([P.namedColours(), P.cname(3)], [true, 'head1'], 'A_Gen6 as the fallback names its colours');
setup({ k: 6, def: 'agen6', graph: '3 0 0-1,1-2', algoSrc: 'return { paint: 1, target: "stay" };' });
same([P.namedColours(), P.cname(3)], [false, 'colour 3'], 'another algorithm in use keeps numbers');
setup({ k: 6, def: 'none', graph: '3 0 0-1,1-2', algoSrc: P.ALGO_TEMPLATES.agen6 });
same(P.namedColours(), true, 'the A_Gen6 template names its colours');
setup({ k: 5, def: 'sigma', graph: '3 0 0-1,1-2' });
same([P.namedColours(), P.cname(0), P.cname(2)], [false, 'white', 'colour 2'], 'classic rules keep numbers');

// 7b. the hint next to the colours picker, as the page draws it
const hint = () => (P.render(), { text: String(element('#khint').textContent), warn: String(element('#khint').className).includes('warn') });
setup({ k: 6, def: 'agen6', graph: '3 0 0-1,1-2' });
same(hint(), { text: 'A_Gen6 needs 6 colours', warn: false }, 'the hint for A_Gen6 with six colours');
setup({ k: 3, def: 'agen6', graph: '3 0 0-1,1-2' });
same(hint(), { text: 'A_Gen6 needs 6 colours (now 3)', warn: true }, 'the hint warns when there are too few colours');
setup({ k: 6, def: 'none', graph: '3 0 0-1,1-2', algoSrc: P.ALGO_TEMPLATES.agen6 });
same(hint().text, 'A_Gen6 needs 6 colours', 'the A_Gen6 template has the same hint');
setup({ k: 6, def: 'agen6', graph: '3 0 0-1,1-2', algoSrc: 'return { paint: 1, target: "stay" };' });
same(hint(), { text: '', warn: false }, 'other code in use has no stated need, whatever the fallback says');
setup({ k: 4, def: 'flipsweep5', graph: '3 0 0-1,1-2' });
same(hint(), { text: 'flipsweep5 needs at least 5 colours (now 4)', warn: true }, 'a built-in rule with a minimum');
setup({ k: 2, def: 'sigma', graph: '3 0 0-1,1-2' });
same(hint(), { text: '', warn: false }, 'sigma* works with any number: no hint');

// 8. compiled code keeps the paper's rule number and knows that it stops
const compiled = P.compileAlgo(P.ALGO_TEMPLATES.agen6, 6, 3);
same([compiled.table.get('4.000010').rule, compiled.table.get('4.000010').target, compiled.stops], [9, -2, true], 'the template compiles with rule numbers and a stop');
same(compiled.errors, [], 'and without rejected rows');
same(compiled.table.has('2.000001'), false, 'fin rows stay undefined');

console.log(bad === 0 ? 'page play ok (the real page script, the real engine)' : `${bad} problem(s)`);
process.exitCode = bad === 0 ? 0 : 1; // not process.exit(): it can cut off piped output (and trips a libuv assertion on Windows)
