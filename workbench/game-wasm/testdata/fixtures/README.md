# Shared fixtures

Each `NAME.req` is a request of the engine's line protocol (see `workbench/README.md`) and `NAME.json` is the
answer it must give. They are checked from two sides:

- natively, by `cargo test --test fixtures` (runs the engine on every request and compares byte for byte);
- in WebAssembly, by `node testdata/check_wasm_parity.mjs` (sends the same requests to `site/game.wasm`).

The answers were produced by the native tool and reviewed by hand. The two large ones reproduce the numbers of
the engine's own tests (sigma* explores Sketch I in 1,601,969 positions; flipsweep5d fails it in 1,291,525).

To regenerate after an intended change (review the diff before committing), from `game-wasm/`:

    for f in testdata/fixtures/*.req; do cargo run -q --release --bin explore -- --request-file "$f" > "${f%.req}.json"; done

The exit status of the tool is non-zero for `fails`, `undefined` and so on; that is expected here.
