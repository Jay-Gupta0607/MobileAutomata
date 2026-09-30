# A_Gen6 rule-to-code checklist

Source: Takahashi, Kanaya, Hiraoka, Eguchi, Sudo, *Recolorable Graph Exploration by an Oblivious Agent with Fewer Colors*, arXiv 2505.02789v1. Model: Section 2. Algorithm: Algorithm 2 (A_Gen6), Section 3.2. Semi-DFS: Algorithm 1, Section 3.1.

Engine: `workbench/game-wasm/src/lib.rs` (Rust, compiled to Wasm) and `workbench/site/index.html` (page, algorithm editor).

Status: **E1 to E8 are implemented** on branch `feature/a-gen6` (one commit each; the engine tests and the page-template agreement check pass). Written as a plan before any engine change; D3 decided (keep absent target means stay; the `y ∉ M` termination clause is deferred). Follow-up fixes after review: a walk judges its last position even when the budget runs out; a rule that can `stop` is judged by the paper model when no `model` is given; the numbers 254 and 255 are no longer colour targets; `agen6` with fewer than six colours is an error. Still open: the page's Play tab and indicators, and rebuilding `site/game.wasm` / `docs/index.html` (Stage 3). Hand traces are in `agen6-hand-trace.xlsx` (path, triangle, tree).

---

## 1. Colours

The engine starts every vertex in colour 0 (`W`), which is the paper's `init`. `k = 6` equals `MAXK`, so all six colours fit.

| # | Paper colour | Role in Semi-DFS |
|---|---|---|
| 0 | `init` | not visited, not on P, not in F |
| 1 | `path` | on P, not the head |
| 2 | `fin` | in F (finished) |
| 3 | `head1` | the head u_k of P |
| 4 | `head2` | head, probing finished, deciding |
| 5 | `neigh` | probed neighbour rejected (not in U) |

Same numbering as the Legend sheet in the trace workbook.

## 2. Input and output, paper against engine

| Paper | Engine | Status |
|---|---|---|
| input `(c, M)`, M a multiset of neighbour colours | `Row { own, cnt[6] }`, text `own.bag` (one hex digit per colour) | matches, no change |
| `x ∈ M` | `row.cnt[x] > 0` (`row.present(x)`) | matches |
| `x ∉ M` | `row.cnt[x] == 0` | matches |
| output `(x, y)`, x new colour | `Action.paint` | matches |
| y a colour | `Action.target` colour | matches |
| y = `stay` | `Action.target = STAY` (255) | matches |
| y = `stop` | **no equivalent** | add `STOP` (proposed 254) |
| y ∉ M: agent terminates | target absent: agent **stays** (`successors`, empty `options`) | **differs. Decided (D3): keep the engine behaviour, deferred** |
| success: every node visited **and** terminated at s | success: every node visited (`p.vis == full`) | **differs** |
| ξ(fin, M) undefined on purpose | undefined row is an existing outcome (`Outcome::Undefined`) | matches |

## 3. The 11 rules

First matching rule for the node's own colour wins. Notation: `has(c)` = `cnt[c] > 0`.

| Paper rule | Own | Condition (in order) | Action `paint>target` | Notes |
|---|---|---|---|---|
| 1 | 0 `init` | `has(1)` | `5>3` | Test **before** Rule 2. A path neighbour means the probed node would make a chord. |
| 2 | 0 `init` | else `has(3)` | `3>3` | Candidate accepted, return to the head. |
| 3 | 0 `init` | otherwise | `3>stay` | Only reachable at the start node (a probed node always has the head as a `head1` neighbour). |
| 4 | 3 `head1` | `has(0)` and not `has(3)` | `3>0` | Probe an `init` neighbour. Adversary picks among `init` neighbours. |
| 5 | 3 `head1` | otherwise | `4>stay` | Move to the decide phase. |
| 6 | 4 `head2` | `has(5)` | `4>5` | Go and reset a `neigh` neighbour. |
| 7 | 4 `head2` | else `has(3)` | `1>3` | Commit to a candidate: this node becomes `path`, agent moves to the candidate. |
| 8 | 4 `head2` | else `has(1)` | `2>1` | Backtrack: this node becomes `fin`, agent moves to its `path` neighbour. |
| 9 | 4 `head2` | otherwise | `2>stop` | Terminate. Needs the new `stop` action. |
| 10 | 1 `path` | always | `3>stay` | Agent arrives back on a `path` node: it becomes the head again. |
| 11 | 5 `neigh` | always | `0>4` | Reset to `init`, return to the `head2` node. |
| (none) | 2 `fin` | always | undefined | The paper defines no rule: the agent is on `fin` only after terminating. |

Reference implementation (JavaScript form for the page's algorithm editor; the Rust `Formula` uses the same order):

```js
// A_Gen6, colours: 0 init, 1 path, 2 fin, 3 head1, 4 head2, 5 neigh
if (own === 0) {
  if (has(1)) return { paint: 5, target: 3 };      // Rule 1
  if (has(3)) return { paint: 3, target: 3 };      // Rule 2
  return { paint: 3, target: 'stay' };             // Rule 3
}
if (own === 3) {
  if (has(0) && !has(3)) return { paint: 3, target: 0 };   // Rule 4
  return { paint: 4, target: 'stay' };                     // Rule 5
}
if (own === 4) {
  if (has(5)) return { paint: 4, target: 5 };      // Rule 6
  if (has(3)) return { paint: 1, target: 3 };      // Rule 7
  if (has(1)) return { paint: 2, target: 1 };      // Rule 8
  return { paint: 2, target: 'stop' };             // Rule 9
}
if (own === 1) return { paint: 3, target: 'stay' };  // Rule 10
if (own === 5) return { paint: 0, target: 4 };       // Rule 11
// own === 2 (fin): no rule
```

## 4. Engine changes (Stage 1), all done: E1 to E8

Each item lists where (function or type names in the current `lib.rs` and `index.html`) and why.

| # | Change | Where | Why |
|---|---|---|---|
| E1 | Add `STOP: u8 = 254`; parse `stop` in `Action::parse`; print it in `Action::text` and `target_json` | `Action`, `target_json` | Rule 9 has to terminate |
| E2 | Add `Formula::AGen6` (name `agen6`) with the 11 rules above; require `k >= 6` (return `None` otherwise, like flip-sweeps) | `Formula`, `parse`, `action` | A closed-form rule, same pattern as `Sweep` |
| E3 | Add a *paper model* switch to `Rule` (proposed: `terminating: bool`, set by `default agen6` or a request key). It changes the **success condition only** (E5), not the absent-target behaviour | `Rule`, `parse_request`, `handle` | Existing rules keep "all vertices visited" as their win condition; A_Gen6 needs "stopped on s" (D3, D4) |
| E4 | `successors`: a `STOP` action gives one terminal position. **Deferred: a colour target absent from the neighbours does *not* terminate; the agent stays, as now (D3)** | `successors`, `Pos` (add `term: bool`) | Paper Section 2 terminates on `stop`. The `y ∉ M` clause is deferred because A_Gen6 never triggers it |
| E5 | `exact_game`: in paper model, expand every non-terminal position even when `vis == full`; seed the attractor with terminal positions where `cur == s` and `vis == full`; other terminal positions are losses | `exact_game` (the `p.vis != full` test and the `win` seeding) | Definition 1: terminate at s, not merely visit |
| E6 | `walk`: same win condition for the walks (`p.vis == full` currently ends a walk as explored) | `walk` | Keep the quick search consistent |
| E7 | Outcome reporting: distinguish "visited all, but did not stop at s" and "stopped early" from "trapped" | `Outcome`, `outcome_json` | Stage 3 needs separate coverage, return and termination indicators |
| E8 | Page template `agen6` in `ALGO_TEMPLATES`; let `compileAlgo` accept `target: 'stop'` (currently only `'stay'`, colours, undefined) | `index.html` `compileAlgo` | Editor and engine must agree on every row |

## 5. Design decisions (confirm with supervisor)

| # | Decision | Recommendation |
|---|---|---|
| D1 | Colour numbering | Table in section 1 (paper order, `init` = 0). |
| D2 | Encoding of `stop` | `STOP = 254`, next to `STAY = 255`. Text form `stop`. |
| D3 | Should "target absent means terminate" apply? | **Decided: no, keep the engine's current behaviour (absent target means stay), for all rules including A_Gen6.** Existing rules rely on it. For example `Formula::Sweep` returns `mv(nxt(own), own)` when no non-white neighbour exists, and the σ\* test expects exactly 1,601,969 positions. A_Gen6 never targets an absent colour in a correct run, so nothing is lost. The paper's `y ∉ M` clause is **deferred** (part of E4). Consequence: a faulty implementation that targeted a missing colour would loop in place and be reported as trapped, not terminate. |
| D4 | Success condition in paper mode | Terminated, on s, every vertex visited. Anything else terminal is a loss. |
| D5 | Undefined `fin` row | Leave undefined. Reaching it is reported as `Undefined`, which is what the paper's proof says cannot happen. |
| D6 | Where the rules live | Rust `Formula::AGen6` (no table needed) plus a JavaScript template that must match it row for row. A table for the JS side needs up to 325,584 rows at degree 15 (6 colours × C(21,6) bags), so keep large-degree graphs in mind. |

## 6. Invariants worth asserting in tests

From the paper's "regular configuration":

- Exactly one `head1` node, and it is where the agent stands, before each probe cycle.
- No `head2` or `neigh` node exists in a regular configuration.
- Every `path` node lies on a single path from s to the head.
- No two path nodes are adjacent unless consecutive (no chords).
- Rule 3 fires only at the very first step.
- A colour target is never absent in a correct run. With D3 the engine would stay in place if it were, so a violation shows up as a trapped or failed play.
- The agent stops on the start node with every node `fin`.

## 7. Tests to add

Use the hand traces as the expected results.

1. **Row/rule table:** for every row `own.bag` up to degree 4, the Rust rule and the JS template return the same action (agreement test, like the existing template checks).
2. **Path 0-1-2:** step sequence equals the 17 rows of the path sheet (node, `own.bag`, rule, paint, target, move).
3. **Triangle:** 21 rows, including the `neigh` rejection (Rules 1, 6, 11).
4. **Tree (0-1, 0-2, 1-3), both tie choices:** 24 rows each.
5. **Exact game:** `explores` under paper model on the path, triangle and tree, from vertex 0. The trace is the adversary's longest obstruction.
6. **Stop semantics:** a rule that stops early on s reports "stopped before visiting all"; a rule that visits everything but never stops reports "trapped".
7. **Start vertex:** run from every vertex of each graph (Definition 1 quantifies over every s).
8. **Regression:** the two existing tests pass unchanged (D3 protects them).
9. **Native against Wasm:** same fixtures, same JSON.

## 8. Open questions

(D3 is settled, see section 5.)

- Should paper mode be selected by rule name (`default agen6`) or by an explicit request key `model paper`? The second is cleaner if other paper algorithms are added later (A_PF5, the eight-colour algorithm).
- Can the page keep offering "search all executions" with the new success condition, or does it need a separate control?
- Do we also want the triangle-free five-colour variant of A_Gen6 (mentioned in the paper) later?
