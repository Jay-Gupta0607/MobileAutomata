# Colour Exploration Workbench (browser)

Draw a graph, write a rule, play the adversary, and let the exact game search
every adversarial execution — in the browser, with no dependencies.  All logic
is Rust compiled to `wasm32-unknown-unknown` (`game-wasm/`, no crates, plain
C-ABI exports: `alloc`, `dealloc`, `run`, `free_result`); the page
(`site/index.html`) is one HTML file with inline CSS and JavaScript and no
libraries or web fonts.

## Build

    sh workbench/build.sh          # needs: rustup target add wasm32-unknown-unknown

produces `site/game.wasm` (~90 KB) and `../docs/index.html`, the same page with the engine inlined (base64),
which opens from a `file://` URL and is what GitHub Pages serves (the build also writes `site/standalone.html`, an
identical copy that is git-ignored, so a fresh clone has only `docs/index.html`).  `site/index.html` +
`site/game.wasm` are for serving over HTTP (any static server, e.g. `python -m http.server 8000 --directory
workbench/site`); the source page cannot be opened from disk.  Needs Rust 1.78 or newer with the `wasm32` target
(Windows: also the Visual Studio C++ Build Tools), `sh` and `python3`; the `node testdata/check_*.mjs` scripts need Node 18
or newer.

## What the page does

* **Drawing.** Modes *Add vertex* (click empty space), *Add edge* (click two
  vertices), *Move* (drag), *Delete* (click a vertex or an edge), *Set start*
  (click; or double-click a vertex).  Keyboard `N E M D S`.  Up to 32
  vertices, degree at most 15.  Import/export as `n start u-v,u-v,...`, the
  `cegis` instance format; presets include the two hand-drawn sketches.
* **Rules.** A table of `own.bag -> paint>target` rows (the `rule_final.txt`
  format, importable), plus a default for rows the table does not list:
  nothing (the play stops at the first undefined row and asks for its action,
  so a rule can be written while playing), σ* or any `sweep(c,t,p)`, the
  flip-sweeps, `eat3`, `chase3`, or chase-white-else-stay.  Rows in the table
  always win over the default.
* **Algorithm editor.** A rule can be written as code: a JavaScript function
  body over the row (`own`, `bag`, `k`, `deg`, helpers `has`, `nxt`, `prv`, `W`)
  returning `{ paint, target }`.  *Compile and use* evaluates it on every row up
  to the graph's largest degree and hands the resulting table to the engine, so
  the exact game analyses exactly the code written.  Every built-in rule has a
  template (*Edit this rule as code*) and can also be written into the editable
  row table (*Edit this rule as table*, *To table*); the templates agree with
  the engine's formulas on every row up to degree 6.  Precedence: table rows,
  then the algorithm, then the fallback.
* **Play.** Step by step: the page shows the row read, the action, and the
  neighbours the adversary may serve; when there are several you click the one
  the agent reaches (or *Auto step*, which serves a visited vertex farthest
  from the unvisited region).  A repeated position ends the play as trapped,
  with the never-visited vertices marked.
* **Analysis.** *Search all executions* runs the exact game in a Web Worker:
  every position the adversary can reach is enumerated up to a cap; the
  verdict is *explores* (no adversarial execution leaves a vertex unvisited;
  the adversary's longest obstruction is offered for replay) or *fails* (one
  trapping execution is offered for replay).  An undefined row met during the
  search is reported with the path that reaches it.  *Run walks* tries the
  three deterministic and N random adversary walks first, a cheap way to find
  a failure, never a proof of success.

### The page in the paper model (A_Gen6)

Choose `A_Gen6 (paper model, k=6)` under "answered by", load the template of the same name in the algorithm editor,
or load one of the four `A_Gen6 ·` presets (the path, the triangle and the tree of the hand traces in
`notes/agen6-hand-trace.xlsx`, and Sketch I; a preset asks before it replaces rows or an algorithm in use).  Any
rule that can `stop` (a table row whose target is `stop`) is played the same way.  Then:

* the play does not end when the last vertex is reached: it ends when the agent stops (or a position repeats), and
  the panel says what happened in the paper's terms: *Explores* (every vertex visited, stopped on the start vertex),
  *Stopped early* (the unvisited vertices are named and ringed), *Stopped away from the start*, or *Never stops*;
* three separate indicators show **coverage** (vertices visited), **return** (on, or stopped on, the start vertex)
  and **termination** (running, stopped, never stops), live during the play;
* the legend, the colour pickers and the ledger use the paper's colour names (init, path, fin, head1, head2, neigh),
  and the Play tab shows which of the 11 rules answered the row just read, with a one-line description; the ledger
  keeps the rule number of each step;
* the Analysis tab says what failed, from the engine's own reason (never stops, stopped early, stopped away from the
  start) with the same three indicators, and a found execution can be replayed step by step;
* the rules table and the define-row prompt offer `stop (paper model)` next to `stay`.
* a hint next to the colours picker says how many colours the rule in use needs: A_Gen6 exactly six (shown in a
  warning colour, with the current number, when fewer are chosen), flipsweep4 and 4b, flipsweep5 and 5d, eat3 and chase3
  at least 4, 5 and 3; rules that work with any number have no hint.  The numbers are checked against the engine.

The classic game keeps its own behaviour and wording.  `game-wasm/testdata/check_page_logic.mjs` compiles the script of
both pages and holds the paper-model logic to the engine (for every paper-model fixture the page's verdict must equal
the engine's reason and indicators); `check_page_play.mjs` runs the page's real script headlessly against the real
engine and drives the Play tab (when a play ends, the banners and indicators, rule numbers, colour names, undo after a
stop, replay); `check_wasm_parity.mjs` also requires the A_Gen6 presets to explore.

## Command line (`explore`)

A native tool over the same engine, for scripts and for checking an algorithm without the page.  It builds
the request of the protocol below and calls the same `handle` function the WebAssembly build exports, so a
native run and a browser run answer identically.  From `game-wasm/`:

    cargo run --bin explore -- exact --graph "4 0 0-1,0-2,1-3" --rule agen6 --summary
    cargo run --bin explore -- play --graph "4 0 0-1,0-2,1-3" --rule agen6 --prefer "1 2 3 0"
    cargo run --bin explore -- exact --preset sketch1 --rule sigma --cap 3000000 --summary
    cargo run --bin explore -- walks --graph "3 0 0-1,1-2" --table rules.txt --k 3
    cargo run --bin explore -- answer --rule agen6 --row 4.001000
    cargo run --bin explore -- --help

Commands: `exact` (every adversary), `walks`, `play` (one adversary, named by `--prefer`), `answer` (the
action for one row).  The graph comes from `--graph "n s u-v,..."`, `--graph-file FILE` or `--preset sketch1`
(edge pieces may be separated by spaces or line breaks).  The rule is `--rule NAME` (a formula such as `sigma` or
`agen6`), `--table FILE` (rows `own.bag paint>target`, `#` comments), or both (table rows win).  Also `--k`
(2 to 6), `--model classic|paper`, `--cap` (exact only, 1000 to 12000000), `--walks` (walks only, 0 to
100000), `--budget` (walks and play, 10 to 5000000), `--prefer` (play only), `--row` (answer only): an option
that does not belong to the command, or a value outside its range, is an error rather than being adjusted
quietly.  Files may be UTF-8 with or without a byte order mark.  The JSON of the protocol is printed; `--summary`
prints one line, `--request` prints the request instead of running it.

Exit status: 0 explores, 1 fails, 2 undefined row, 3 overflow (raise `--cap`), 4 unrefuted (the walks found no
failure: not a proof), 5 unfinished (`play` ran out of budget), 64 usage error or an error from the engine.
`cargo test` in `game-wasm/` also runs the tool's tests (`tests/cli.rs`).  `build.sh` builds only the library
for WebAssembly.

Native and WebAssembly are held to the same answers by the fixtures in `game-wasm/testdata/fixtures` (protocol
requests with their expected JSON): `cargo test` runs them natively, and after `sh build.sh`,
`node game-wasm/testdata/check_wasm_parity.mjs` sends the same requests to `site/game.wasm` and also checks that
`docs/index.html` embeds that build.  `node game-wasm/testdata/check_agen6_template.mjs` checks the page's A_Gen6
template against the engine's rule (action and rule number, for every row up to degree 4), ,
`node game-wasm/testdata/check_page_logic.mjs` checks that the pages' scripts compile and their paper-model logic, and
`node game-wasm/testdata/check_page_play.mjs` drives the Play tab headlessly.

## Protocol (for other front ends)

`run` takes UTF-8 text lines `key value`: `cmd step|exact|walks|play|answer`,
`k`, `graph n s u-v,...`, `default NAME`, `cap`, `walks`, `budget`,
`colours c0 c1 ...`, `cur v`, `vis v1 v2 ...`, `row own.bag`, `prefer v1 v2 ...`, `model
classic|paper`, and a block `table` … `end` of `row paint>target` lines, where
`target` is a colour number, `stay` (also `s`, `-`) or `stop`; the numbers 254
and 255 are not colours and are rejected.  It returns a `u32` little-endian
length followed by JSON: for `step` the row, its source, action, options and
the automatic choice; for `exact`/`walks` `{status: explores|fails|undefined|
overflow|unrefuted, positions, method, trace:{repeat_at, unvisited, steps:[{cur,
row, paint, target, options, next}]}}`.  `cargo test` in `game-wasm/` checks
the engine against the native `cegis` numbers on Sketch I.

### One play under a named adversary (`cmd play`)

`cmd play` runs a single execution and answers with its full trace, so a hand
trace can be reproduced step by step.  The adversary chooses, whenever several
neighbours carry the target colour, the one listed first in `prefer v1 v2 ...`
(a vertex not listed comes last, the smaller number first; a vertex outside the
graph is an error).  `budget` limits the number of steps (at least 10).  The
answer is `explores` (the play won), `fails` (it stopped in the wrong place or
repeated a position), `undefined` (no action for a row) or `unfinished` (the
budget ran out before the play was decided).  It shows one adversary's play;
only `exact` covers every adversary.

### Paper model (`model paper`)

The default `model classic` is the game above: the agent wins as soon as every
vertex is visited, and `stop` would only mean "stay".  `model paper` is the
model of Takahashi et al. (arXiv 2505.02789, Section 2): `stop` ends the run
after painting the vertex, and the agent explores only if it stops **on the
start vertex with every vertex visited**; a play that never stops, stops
elsewhere, or stops before everything is visited is a failure.  With no
`model` key the paper model is used whenever the rule can stop: `default
agen6` (A_Gen6, Algorithm 2, colours 0 init, 1 path, 2 fin, 3 head1, 4 head2,
5 neigh; needs `k 6`) or a table row answering `stop`.  A colour target that
no neighbour has still leaves the agent in place in both models.

Every `exact`/`walks` answer carries `"model"`, and each trace carries
`stopped_at` (the vertex the agent stopped on, or null; a `stop` action is
reported as target `-2`, `stay` as `-1`).  In the paper model an
`explores`/`fails` answer also carries `indicators` `{visited_all, stopped,
stopped_at_start}` and a failure a `reason`: `never_stops`, `stopped_early`
or `stopped_off_start`.
