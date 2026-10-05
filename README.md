# MobileAutomata

Simulators for mobile automata exploring graphs.

* **Colour Exploration Workbench** — served at <https://drdebmath.github.io/MobileAutomata/>
  from [`docs/`](docs/): one deterministic agent with `k` colours, rows `own.bag`,
  actions `paint>target`, an adversary choosing among same-coloured neighbours.
  Draw a graph by clicking, write a rule table or grow it row by row while playing,
  play the adversary yourself, and search **every** adversarial execution with the
  exact game.  Rust compiled to WebAssembly, no dependencies.  Source and build
  script in [`workbench/`](workbench/) (`sh workbench/build.sh` rewrites
  `docs/index.html`).
  A second mode, **two agents** (the *agents* box), runs the built-in five-colour rule A_2x5: two agents that see each other,
  with the five colours that one agent manages only on restricted classes.  It has been checked on every connected graph of up
  to eight vertices; it is not proved for all graphs (see [`workbench/README.md`](workbench/README.md), "Two agents", for the
  model, its assumptions and what is checked).
* **DMA Graph Exploration Simulator** — the original cytoscape-based simulator,
  now in [`dma-simulator/`](dma-simulator/) (open its `index.html`).

## Quick start

**Just use it (nothing to install).**  Open [`docs/index.html`](docs/index.html) in a browser: double-click it.  It is
one self-contained file with the engine built in, and it is the page GitHub Pages serves.

    git clone https://github.com/drdebmath/MobileAutomata.git
    cd MobileAutomata
    # then open docs/index.html

(While a change is still a branch or a pull request, `git checkout` that branch first.)

**Serve the source page** (Python 3):

    python -m http.server 8000 --directory workbench/site

and open <http://localhost:8000>.  The source page `workbench/site/index.html` cannot be opened by double-clicking it:
a browser will not let a page loaded from disk fetch `game.wasm` ("Failed to fetch").

**Rebuild, test, use the command line** (see [`workbench/README.md`](workbench/README.md) for all of it):

| To | Needs | Run |
|---|---|---|
| rebuild `docs/index.html` and `workbench/site/game.wasm` | Rust 1.78 or newer and `rustup target add wasm32-unknown-unknown`; `sh` and `python3` (Git Bash on Windows) | `sh workbench/build.sh` |
| run the engine, tool and fixture tests | Rust (on Windows also the Visual Studio C++ Build Tools) | `cd workbench/game-wasm` then `cargo test` |
| run the page and WebAssembly checks | Node 18 or newer | `node testdata/check_page_play.mjs`, `check_page_logic.mjs`, `check_wasm_parity.mjs`, `check_paper_templates.mjs` (from `workbench/game-wasm`) |
| explore a graph from the command line | Rust | `cargo run --bin explore -- --help` (from `workbench/game-wasm`) |

## Hosting

GitHub Pages serves the `docs/` folder of `main` (Settings → Pages → Deploy from a
branch → `main` / `docs`).  The served site is a single file, `docs/index.html`,
with the WebAssembly engine inlined, so nothing else is needed.
