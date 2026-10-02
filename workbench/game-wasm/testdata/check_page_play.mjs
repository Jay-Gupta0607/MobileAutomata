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
globalThis.__page = { state, loadGraphText, resetPlay, stepOnce, undo, loadReplay, render, save, paperMode, namedColours, cname, usesStop, engine, requestText, compileAlgo, ALGO_TEMPLATES, playPaperStatus, makeRandomGraph, analyse, togglePlay };
})();`);

// a stand-in for an element: remembers what is set on it, answers every method with itself (and notes which were called)
const elements = new Map();
const calls = [], alerts = [];
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
      return props[k] = (...a) => { calls.push(k); return el; };
    },
    set(_, k, v) { props[k] = v; return true; },
    apply() { return el; },
  });
  elements.set(sel, el);
  return el;
}
const docListeners = {}; // what the page registers on the document, by event
const document = { hidden: false, querySelector: (s) => element(s), querySelectorAll: () => [], activeElement: null, addEventListener(type, fn) { (docListeners[type] = docListeners[type] || []).push(fn); } };
let fakeNow = 0; // performance.now() for the page, moved by the test
// a worker that starts (answers 'init') but never finishes a search, so a search can be "running" while the test acts
let workersStarted = 0;
const workers = []; // every worker the page starts; one that is told a number is the play clock, and remembers its interval
class Worker { constructor() { workersStarted++; workers.push(this); } postMessage(m) { if (m && m.type === 'init') queueMicrotask(() => this.onmessage({ data: { type: 'ready' } })); else if (typeof m === 'number') this.tickMs = m; } terminate() {} }
const sandbox = {
  document, Worker, performance: { now: () => fakeNow }, console, setInterval, clearInterval, setTimeout, clearTimeout, Blob, URL, TextEncoder, TextDecoder,
  localStorage: { getItem: () => null, setItem() {} },
  fetch: async () => { const b = readFileSync(join(site, 'game.wasm')); return { ok: true, arrayBuffer: async () => b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength) }; },
  alert: (m) => alerts.push(String(m)), confirm: () => true,
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

// 7c. a step must not scroll the panel or the page (scrollIntoView did, at every step); the ledger's own box follows
//     the newest step only when it was already at the bottom
setup({ k: 6, def: 'agen6', graph: '4 0 0-1,0-2,1-3' });
const box = element('#ledgerwrap');
Object.assign(box, { scrollHeight: 500, clientHeight: 260, scrollTop: 240 }); // scrolled to the bottom
calls.length = 0; P.stepOnce(true); P.stepOnce(true); P.render();
check(!calls.includes('scrollIntoView'), 'stepping does not call scrollIntoView (it scrolled the whole panel)');
same(box.scrollTop, 500, 'the ledger box follows the newest step when it was at the bottom');
box.scrollTop = 0; // the reader scrolled up to look at older steps
P.stepOnce(true); P.render();
same(box.scrollTop, 0, 'and stays where the reader put it');
check(!calls.includes('scrollIntoView'), 'still no scrollIntoView');

// 8. compiled code keeps the paper's rule number and knows that it stops
const compiled = P.compileAlgo(P.ALGO_TEMPLATES.agen6, 6, 3);
same([compiled.table.get('4.000010').rule, compiled.table.get('4.000010').target, compiled.stops], [9, -2, true], 'the template compiles with rule numbers and a stop');
same(compiled.errors, [], 'and without rejected rows');
same(compiled.table.has('2.000001'), false, 'fin rows stay undefined');

// 9. the random graph option, as the button drives it: every size from 2 to 32 gives a graph on the canvas that the
//    real engine accepts, with a fresh play at the start vertex; A_Gen6 explores each of them (Definition 1)
Object.assign(P.state, { k: 6, def: 'agen6', table: new Map(), algoSrc: '', algo: null, algoError: null });
const seenGraphs = new Set();
for (let n = 2; n <= 32; n++) {
  element('#rn').value = String(n); alerts.length = 0;
  P.makeRandomGraph();
  const { nodes, edges } = P.state, text = `${nodes.length} ${P.state.start} ${edges.map(([u, v]) => u + '-' + v).join(',')}`;
  same([nodes.length, P.state.start, alerts.length], [n, 0, 0], `n=${n}: the canvas has the chosen number of vertices, start 0, no complaint`);
  check(nodes.every((nd) => nd.x >= 50 && nd.x <= 950 && nd.y >= 40 && nd.y <= 600), `n=${n}: every vertex is on the canvas (the svg is 1000 by 640)`);
  check(P.state.play && P.state.play.cur === 0 && P.state.play.steps.length === 0 && P.state.play.vis.size === 1, `n=${n}: a fresh play at the start vertex`);
  const ans = P.engine(P.requestText('walks', { walks: 8 }));
  same([ans.status, ans.model], ['unrefuted', 'paper'], `n=${n}: the engine accepts the graph (${text}) and A_Gen6 survives the walks`);
  seenGraphs.add(text);
}
check(seenGraphs.size >= 20, `the button gives different graphs each time (${seenGraphs.size} different of 31)`);
// the rule, the colours and the rows in use are left alone
Object.assign(P.state, { k: 3, def: 'sigma', table: new Map([['0.10', { paint: 1, target: 0 }]]) });
element('#rn').value = '6'; P.makeRandomGraph();
same([P.state.k, P.state.def, P.state.table.size, P.state.nodes.length], [3, 'sigma', 1, 6], 'a new graph does not change the colours, the rule or the rows');
// a size that is not 2 to 32 is refused with a message, and the graph on the canvas stays
const graphOf = () => `${P.state.nodes.length} ${P.state.start} ${P.state.edges.map(([u, v]) => u + '-' + v).join(',')}`;
setup({ k: 6, def: 'agen6', graph: '4 0 0-1,0-2,1-3' });
for (const bad of ['', '0', '1', '33', '-4', '2.5', 'abc', '1e3']) {
  element('#rn').value = bad; alerts.length = 0;
  P.makeRandomGraph();
  same([alerts, graphOf()], [['A random graph has 2 to 32 vertices.'], '4 0 0-1,0-2,1-3'], `"${bad}" vertices: refused, canvas unchanged`);
}

// 9b. the "start" label above the start vertex is clear of the agent's ring (radius 27, stroke 5: 29.5 out) and of the
//     option halo (34.2 out), and is not pushed off the top of the canvas
setup({ k: 6, def: 'agen6', graph: '4 0 0-1,0-2,1-3' }); // the agent starts on the start vertex, so its ring is drawn there
P.render();
const startLabel = () => { const m = String(element('#svg').innerHTML).match(/<text x="([\d.]+)" y="(-?[\d.]+)"[^>]*>start<\/text>/); return m ? { x: +m[1], y: +m[2] } : null; };
const startNode = P.state.nodes[P.state.start], startY = startNode.y, label = startLabel();
check(label !== null, 'the start vertex has a label');
check(label && startY - label.y >= 34.2 + 2, `the label's baseline is above the halo, so above the ring (${label && (startY - label.y).toFixed(1)} units over the centre)`);
check(label && label.x === startNode.x, 'and it is centred on the vertex');
startNode.y = 20; P.render(); // a vertex dragged to the top edge
check(startLabel().y >= 8, `near the top of the canvas the label stays on it (baseline ${startLabel().y})`);
startNode.y = startY; P.render();

// 10. what the Analysis tab shows belongs to the graph it was computed for: replacing the whole graph (the random graph
//     button, a preset, Import) clears it, and cancels a search that is still running for the old graph.  Before this,
//     "Load this execution" stayed on the page and replayed the old graph's trace on the new graph ("Trapped").
const analysis = () => String(element('#analysis').innerHTML), buttons = () => [element('#b-exact').disabled, element('#b-cancel').disabled];
setup({ k: 6, def: 'agen6', graph: '4 0 0-1,0-2,1-3' });
element('#analysis').innerHTML = '<div class="banner bad"><b>Fails.</b></div><button id="b-load-trace">Load this execution into the player</button>';
const started = workersStarted;
element('#rn').value = '9'; P.makeRandomGraph();
same(analysis(), '', 'a finished result is gone once a random graph replaces the canvas');
same(workersStarted, started, 'and no search was running, so the worker is left alone');
element('#analysis').innerHTML = '<div class="banner good">Explores</div>';
P.loadGraphText('3 0 0-1,1-2');
same(analysis(), '', 'loading a graph from text (Import, presets) clears it too');
// a search that is still running
P.analyse('exact');
await new Promise((res) => setTimeout(res, 0));
same([buttons(), analysis().includes('Running')], [[true, false], true], 'a search is running: search disabled, cancel enabled');
P.makeRandomGraph();
same([buttons(), analysis(), workersStarted > started], [[false, true], '', true], 'a new graph cancels it: buttons back, panel empty, a fresh worker');
// 11. the algorithms of Hiraoka et al. in the page, chosen as the rule and as the code of the template: the same play, the paper's
//     letter on the rule numbers of the ledger, the algorithm's own colour names.  The engine's formula and the page's template
//     are two transcriptions of the paper's rules; every step of the two plays must agree.
const hiraoka = [
  ['atc3', 3, '5 0 0-1,1-2,2-3,3-4,0-4', 'D', 'l0'], ['atc3', 3, '7 0 0-1,0-2,1-3,1-4,2-5,2-6', 'D', 'l1'],
  ['ac4', 4, '5 0 0-1,1-2,2-3,3-4,1-4', 'C', 'front'], ['ac4', 4, '5 0 0-3,0-4,1-3,1-4,2-3,2-4', 'C', 'path'],
  ['aus5', 5, '4 0 0-1,0-2,0-3,1-2,1-3,2-3', 'U', 'neigh'], ['aus5', 5, '6 0 0-1,0-2,0-3,1-2,1-3,2-3,2-4,4-5', 'U', 'head'],
];
for (const [key, k, graph, letter, colourName] of hiraoka) {
  const plays = {};
  for (const via of ['rule', 'template']) {
    const what = `${key} on ${graph} via ${via}`;
    setup(via === 'rule' ? { k, def: key, graph } : { k, def: 'none', graph, algoSrc: P.ALGO_TEMPLATES[key] });
    same(P.paperMode(), true, `${what}: the paper model`);
    P.render(); run(400);
    const p = P.state.play;
    same([p.done && p.done.kind, P.playPaperStatus(p).outcome], ['stopped', 'explores'], `${what}: stops on the start with every vertex visited`);
    check(p.steps.length > 0 && p.steps.every((s) => Number.isInteger(s.rule)), `${what}: every step carries the paper's rule number`);
    P.render();
    const ledger = String(element('#ledger tbody').innerHTML), legend = String(element('#legend').innerHTML);
    check(ledger.includes(`· ${letter}`) && !ledger.includes('· r'), `${what}: the ledger tags the rules ${letter}n`);
    check(legend.includes(`</span>${colourName}</span>`), `${what}: the legend names the colours (${colourName})`);
    plays[via] = p.steps.map((s) => [s.cur, s.row, s.paint, s.target, s.next, s.rule]);
  }
  same(plays.template, plays.rule, `${key} on ${graph}: the template and the formula play identically`);
}
// the panel's sentence uses the right article: A_TC3 starts by painting l0 and moving to an init neighbour
setup({ k: 3, def: 'atc3', graph: '3 0 0-1,1-2' });
check(panel().includes('moves to an <b>init</b> neighbour'), 'the panel says "an init neighbour", not "a init neighbour"');
// more colours than the algorithm needs are unused
setup({ k: 5, def: 'atc3', graph: '3 0 0-1,1-2' });
same([P.cname(1), P.cname(2), P.cname(3), P.cname(4)], ['l0', 'l1', 'unused', 'unused'], 'A_TC3 with five colours leaves two unused');
same(hint(), { text: 'A_TC3 needs 3 colours (the other 2 unused)', warn: false }, 'and the hint says so without a warning');
setup({ k: 2, def: 'aus5', graph: '3 0 0-1,1-2' });
same(hint(), { text: 'A_US5 needs 5 colours (now 2)', warn: true }, 'too few colours is a warning');
// the engine refuses too few colours, and the page shows its message instead of a play
check(String(P.engine(P.requestText('step', { colours: '0 0 0', cur: 0, vis: '0' })).message).includes('aus5 needs 5 colours'), 'the engine says aus5 needs five colours');
// a table row still wins over the algorithm, and then carries no rule number
setup({ k: 3, def: 'atc3', graph: '3 0 0-1,1-2', table: [['0.100', { paint: 1, target: 0 }]] });
check(P.state.play.pending.source === 'table' && !P.state.play.pending.rule, 'a row of the table wins over A_TC3 and has no rule number');

// 12. the play loop runs on the clock of a worker and does the steps that are due, so it keeps its speed in a tab nobody is
//     looking at: a late tick (a browser may slow a hidden tab) catches up, and a hidden tab draws nothing until it is shown
setup({ k: 6, def: 'agen6', graph: '4 0 0-1,0-2,1-3' });
element('#speed').value = '5'; // 1300 / 5 = 260 ms a step
fakeNow = 1000; const workersBefore = workers.length;
P.togglePlay();
const clock = workers[workers.length - 1];
same([P.state.playing, workers.length - workersBefore, clock.tickMs, String(element('#b-play').textContent)], [true, 1, 260, '❚❚ Pause'], 'Play starts the clock of a worker at the speed of the slider');
fakeNow += 260; clock.onmessage({ data: 0 });
same(P.state.play.steps.length, 1, 'a tick on time does one step');
fakeNow += 260 * 10.5; clock.onmessage({ data: 0 });
same(P.state.play.steps.length, 11, 'a tick that comes 10.5 steps late does the 10 steps that are due');
fakeNow += 260 * 0.5; clock.onmessage({ data: 0 });
same(P.state.play.steps.length, 12, 'and the half step that was left over counts towards the next');
// hidden: the steps are done, nothing is drawn
element('#playstatus').innerHTML = ''; document.hidden = true;
fakeNow += 260 * 3; clock.onmessage({ data: 0 });
same([P.state.play.steps.length, String(element('#playstatus').innerHTML), P.state.stale], [15, '', true], 'in a hidden tab the steps are done but nothing is drawn');
document.hidden = false; docListeners.visibilitychange.forEach((fn) => fn());
check(String(element('#playstatus').innerHTML).includes('Step 15') && P.state.stale === false, 'shown again, the tab draws where the play has got to');
// the play ends by itself at the stop, whatever the lateness, and the clock is stopped
fakeNow += 260 * 100000; clock.onmessage({ data: 0 });
same([P.state.playing, P.state.play.done && P.state.play.done.kind, P.state.play.steps.length, clock.tickMs, String(element('#b-play').textContent)], [false, 'stopped', 24, 0, '▶ Play'], 'the play ends at the stop, A_Gen6 having stopped on the start after 24 steps, and the clock stops');
// a tick that arrives after the play was stopped does nothing
const n24 = P.state.play.steps.length; fakeNow += 5000; clock.onmessage({ data: 0 });
same(P.state.play.steps.length, n24, 'a stray tick after the end does nothing');
// pausing stops the clock, and Play again carries on from where it was
setup({ k: 6, def: 'agen6', graph: '4 0 0-1,0-2,1-3' });
fakeNow = 0; P.togglePlay(); fakeNow += 260 * 4; clock.onmessage({ data: 0 });
P.togglePlay();
same([P.state.playing, clock.tickMs, P.state.play.steps.length], [false, 0, 4], 'Pause stops the clock');
fakeNow += 1e6; clock.onmessage({ data: 0 });
same(P.state.play.steps.length, 4, 'a paused play does not move');
P.togglePlay(); fakeNow += 260 * 2; clock.onmessage({ data: 0 });
same([P.state.playing, P.state.play.steps.length, workers.filter((w) => w === clock).length], [true, 6, 1], 'Play again carries on, with the same worker');
P.togglePlay();

// 13. the family choice, as the button drives it, against the real engine: every algorithm that is guaranteed for a family explores the
//     graphs of that family, whatever their size; and the algorithms with fewer colours do fail outside their families, so the choice
//     matters.  Math.random of the page is seeded here, so the graphs (and any failure) can be reproduced.
vm.runInContext('(() => { let s = 20261002; Math.random = () => { s = (s + 0x6D2B79F5) | 0; let t = Math.imul(s ^ (s >>> 15), 1 | s); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; })()', sandbox);
const colourCount = { atc3: 3, ac4: 4, aus5: 5, agen6: 6 };
const useRule = (key) => Object.assign(P.state, { k: colourCount[key], def: key, table: new Map(), algoSrc: '', algo: null, algoError: null });
const generate = (family, n) => { element('#rfam').value = family; element('#rn').value = String(n); alerts.length = 0; P.makeRandomGraph(); return alerts.length === 0; };
const guaranteed = { any: ['agen6'], ktf: ['aus5', 'agen6'], cb: ['ac4', 'aus5', 'agen6'], cactus: ['ac4', 'aus5', 'agen6'], tree: ['atc3', 'ac4', 'aus5', 'agen6'], cycle: ['atc3', 'ac4', 'aus5', 'agen6'] };
let familyRuns = 0;
for (const [family, algos] of Object.entries(guaranteed)) {
  for (const n of [3, 4, 5, 6, 12, 20, 32]) for (let rep = 0; rep < 2; rep++) {
    check(generate(family, n), `${family} n=${n}: generated without a complaint`);
    same(P.state.nodes.length, n, `${family} n=${n}: the canvas has ${n} vertices`);
    for (const key of algos) {
      useRule(key);
      const small = n <= 6, ans = P.engine(P.requestText(small ? 'exact' : 'walks', small ? { cap: 5000000 } : { walks: 24 }));
      same(ans.status, small ? 'explores' : 'unrefuted', `${family} n=${n} with ${key}: ${P.state.edges.map(([u, v]) => u + '-' + v).join(',')}`);
      familyRuns++;
    }
  }
}
check(familyRuns >= 180, `${familyRuns} runs of an algorithm on a graph of a family it is guaranteed for`);
// outside its family an algorithm may fail (the walks only ever find real failures)
const failures = (family, key, n, runs) => {
  let found = 0;
  for (let i = 0; i < runs; i++) { generate(family, n); useRule(key); if (P.engine(P.requestText('walks', { walks: 24 })).status === 'fails') found++; }
  return found;
};
const outside = [['any', 'aus5', 'A_US5 on graphs of no family (its guarantee is for clique / triangle-free blocks)'], ['ktf', 'ac4', 'A_C4 on graphs with clique blocks (its guarantee is for cycle / K(p,q) blocks)'], ['cb', 'atc3', 'A_TC3 on graphs with K(p,q) blocks (its guarantee is for trees and cycles)'], ['cactus', 'atc3', 'A_TC3 on cacti (its guarantee is for trees and cycles)']];
for (const [family, key, what] of outside) check(failures(family, key, 20, 30) > 0, `${what} fails on some graph`);

console.log(bad === 0 ? 'page play ok (the real page script, the real engine)' : `${bad} problem(s)`);
process.exitCode = bad === 0 ? 0 : 1; // not process.exit(): it can cut off piped output (and trips a libuv assertion on Windows)
