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
* **Random graph.** The *random graph* box in the header takes a number of vertices (2 to 32) and a *family*, and *Generate*
  (or Enter) replaces the canvas with a new random connected graph of that size and family.  The families are the
  classes the algorithms of the papers are built for, each inside the next, so each algorithm is guaranteed on its
  family and on every family above it (the menu names the first one):

  | family | blocks of the graph | guaranteed for |
  |---|---|---|
  | simple cycle | one cycle (3 to 32 vertices) | A_TC3, A_C4, A_US5, A_Gen6 |
  | tree | bridges only | A_TC3, A_C4, A_US5, A_Gen6 |
  | cactus | bridges and cycles | A_C4, A_US5, A_Gen6 |
  | cycle / K(p,q) blocks | bridges, cycles and complete bipartite K(p,q), p and q from 2 to 4 | A_C4, A_US5, A_Gen6 |
  | clique / triangle-free blocks | bridges, cliques K3 to K5, cycles and triangle-free blocks with chords | A_US5, A_Gen6 |
  | any connected graph | a random spanning tree plus up to n/2 extra edges | A_Gen6 |

  The first five are built block by block: vertex 0, then blocks attached at random vertices until exactly n vertices are
  placed.  No vertex gets more than 15 neighbours (the engine's limit); the start vertex is 0; the graph is laid out by a
  spring layout so it can be read, inside the part of the canvas that the legend (top right) and the hint bar (bottom
  left) do not cover (they are measured after the new graph is drawn, since that can change the size of the canvas box).
  "Any connected graph" is what the box gave before the families existed; it falls outside the A_US5 family often (about
  one graph in five at 10 vertices, about half at 32), which is when A_US5 can fail on it.  The rule, the colours and the
  rows in use are left alone.  A size outside the family's range (a cycle needs 3) is refused with a message and the
  canvas stays as it was.  Replacing the whole graph (this option, a preset, Import) also clears the Analysis tab and
  cancels a search still running, since a result belongs to the graph it was computed for.
* **Play.** *Play* steps at the speed of the slider.  Its clock is a timer inside a Web Worker rather than a timer of the
  page, and each tick does the steps that are due by the clock, so a play keeps its speed while you are in another tab
  (a browser slows the timers of a page nobody is looking at; a tick that comes late catches up instead).  A hidden
  tab does the steps but draws nothing until it is shown again.
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

### The page in the paper model (A_Gen6, A_TC3, A_C4, A_US5)

Choose one of the four algorithms of the papers under "answered by" (`A_Gen6 (paper model, k=6)`, `A_TC3 ... k=3`,
`A_C4 ... k=4`, `A_US5 ... k=5`; choosing one raises a palette that is too small), load the template of the same name in
the algorithm editor, or load one of the presets: the four `A_Gen6 ·` ones (the path, the triangle and the tree of the
hand traces in `notes/agen6-hand-trace.xlsx`, and Sketch I), `A_TC3 ·` on a 5-cycle and a binary tree, `A_C4 ·` on the
two graphs of Figure 2 of its paper (a bridge with a 4-cycle, and K3,2), and `A_US5 ·` on K4 and on K4 with a tail (a preset
asks before it replaces rows or an algorithm in use).  Any rule that can `stop` (a table row whose target is `stop`) is
played the same way.  Then:

* the play does not end when the last vertex is reached: it ends when the agent stops (or a position repeats), and
  the panel says what happened in the paper's terms: *Explores* (every vertex visited, stopped on the start vertex),
  *Stopped early* (the unvisited vertices are named and ringed), *Stopped away from the start*, or *Never stops*;
* three separate indicators show **coverage** (vertices visited), **return** (on, or stopped on, the start vertex)
  and **termination** (running, stopped, never stops), live during the play;
* the legend, the colour pickers and the ledger use the algorithm's own colour names (A_Gen6: init, path, fin, head1,
  head2, neigh; A_TC3: init, l0, l1; A_C4: init, fin, front, path; A_US5: init, path, neigh, fin, head; a colour the
  algorithm does not use is called unused), and the Play tab shows which rule answered the row just read, with a
  one-line description; the ledger keeps the rule of each step, tagged as in the paper (r5 for A_Gen6, D5, C9, U2 for
  the others);
* the Analysis tab says what failed, from the engine's own reason (never stops, stopped early, stopped away from the
  start) with the same three indicators, and a found execution can be replayed step by step;
* the rules table and the define-row prompt offer `stop (paper model)` next to `stay`.
* a hint next to the colours picker says how many colours the rule in use needs: the four algorithms of the papers
  exactly six, three, four and five (shown in a warning colour, with the current number, when fewer are chosen, and
  noting the unused ones when more), flipsweep4 and 4b, flipsweep5 and 5d, eat3 and chase3 at least 4, 5 and 3; rules
  that work with any number have no hint.  The numbers are checked against the engine.

The classic game keeps its own behaviour and wording.  `game-wasm/testdata/check_page_logic.mjs` compiles the script of
both pages and holds the paper-model logic to the engine (for every paper-model fixture the page's verdict must equal
the engine's reason and indicators); `check_page_play.mjs` runs the page's real script headlessly against the real
engine and drives the Play tab (when a play ends, the banners and indicators, rule numbers, colour names, undo after a
stop, replay) and the random graph button (every size from 2 to 32 gives a graph the engine accepts, on which A_Gen6
survives the walks), checks the play clock (a late tick catches up, a hidden tab draws nothing, pause stops the clock),
generates graphs of every family at several sizes and runs every algorithm guaranteed for the family on them (they must
explore, and the algorithms with fewer colours must fail somewhere outside their family), and plays each of the three
algorithms of the second paper twice, once chosen as the rule and once as
the code of its template, which must give the same play step for step; `check_page_logic.mjs` also checks the generator
(connected, no repeated edge, at most 15 neighbours, different graphs each time, the extremes of the randomness), that
every graph of each family is in its family (by a separate block decomposition) and the families nest, and that the layout puts no two vertices on top of each other, and that each template's colour constants and rule numbers
agree with the page's colour names and rule descriptions; `check_wasm_parity.mjs` also requires every preset that
brings one of the algorithms of the papers to explore.

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
`agen6`, `atc3`, `ac4`, `aus5`; each of the last four sets its own `--k`), `--table FILE` (rows `own.bag paint>target`, `#` comments), or both (table rows win).  Also `--k`
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
`docs/index.html` embeds that build.  `node game-wasm/testdata/check_paper_templates.mjs` checks the page's template of each of the four
algorithms against the engine's rule (action and rule number, for every row up to degree 4: 1,254, 102, 276 and 625 rows),
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
`model` key the paper model is used whenever the rule can stop: `default`
one of the algorithms of the papers (next section) or a table row answering
`stop`.  A colour target that no neighbour has still leaves the agent in place
in both models.

Every `exact`/`walks` answer carries `"model"`, and each trace carries
`stopped_at` (the vertex the agent stopped on, or null; a `stop` action is
reported as target `-2`, `stay` as `-1`).  In the paper model an
`explores`/`fails` answer also carries `indicators` `{visited_all, stopped,
stopped_at_start}` and a failure a `reason`: `never_stops`, `stopped_early`
or `stopped_off_start`.

### The algorithms of the papers (`default NAME`)

Four rules are built in as named formulas.  Each is one rule function of the model of the papers, with the paper's
own rule numbers (an answer for a row carries `"rule": N`, and the page and the ledger show it).

| name | from | colours (numbers) | explores | moves |
|---|---|---|---|---|
| `agen6` | Takahashi et al., arXiv 2505.02789, Algorithm 2 | 6: 0 init, 1 path, 2 fin, 3 head1, 4 head2, 5 neigh | every graph | |
| `atc3` | Hiraoka, Imori, Takahashi, Sudo, arXiv 2609.14356, Algorithm 1 (rules D1-D11) | 3: 0 init, 1 l0, 2 l1 | every tree and every simple cycle | O(n) |
| `ac4` | same paper, Algorithm 2 (rules C1-C14) | 4: 0 init, 1 fin, 2 front, 3 path | graphs whose blocks are cycles or complete bipartite graphs (all cacti) | O(n) |
| `aus5` | same paper, Algorithm 3 (rules U1-U19) | 5: 0 init, 1 path, 2 neigh, 3 fin, 4 head | graphs whose blocks are cliques or triangle-free (all unichord-free graphs) | O(nΔ) |

The paper model is chosen for them without being asked.  Fewer colours than an algorithm needs is an error
(`atc3 needs 3 colours (k 3)`); more are left unused.  Where the paper gives no output the row is undefined: a `fin`
vertex (the agent is never on one: no rule requests `fin`), and the observations the papers prove unreachable, such as
an `l0` vertex seeing only `init` neighbours in A_TC3.  `stay` and `stop` are the engine's `stay` and `stop` targets.

How the transcription of the second paper is checked (`src/paper_tests.rs`, run by `cargo test`): every rule on a row
derived by hand; the two traces of Figure 2 of the paper for A_C4, whose rule sequences the figure prints; every
connected graph of up to seven vertices, up to isomorphism, from every start vertex and against every adversary
(167, 776 and 1,117 graph-and-start pairs in the three classes, all explored); a scan of every reachable position for
an undefined row or a request for a colour no neighbour has, which the paper counts as a failure but the engine would
turn into a stay (none occurs); failures outside the classes (A_TC3 fails on a triangle with a pendant vertex, as
Theorem 13 of the paper requires of any three-colour rule; A_US5 on the diamond, A_C4 on K4).
