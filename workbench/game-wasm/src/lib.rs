//! The colour-exploration game in the browser.
//!
//! One deterministic agent with `k` colours walks a finite connected graph.
//! It sees only a *row* — its own colour and the multiset of its neighbours'
//! colours — and answers with an *action*: paint the current vertex and name
//! a target colour.  The *adversary* chooses which neighbour of that colour
//! the agent reaches; when no neighbour carries the colour the agent stays.
//! A rule explores a rooted graph when every adversarial play visits every
//! vertex.
//!
//! This crate has no dependencies and exports three C-ABI functions for
//! `wasm32-unknown-unknown`: `alloc`, `dealloc` and `run`.  `run` takes a
//! UTF-8 request (a small line protocol, see `handle`) and returns a
//! length-prefixed UTF-8 JSON answer.  Everything the page shows — the row
//! read at a vertex, the rule's action, the adversary's options, one play
//! under an automatic adversary, and the *exact game* that enumerates every
//! position the adversary can reach and decides whether some adversarial
//! execution leaves a vertex unvisited — is computed here.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

// ------------------------------------------------------------------ hashing
// A fixed hasher: no operating-system randomness is needed on the web target
// and the analysis stays reproducible.
#[derive(Default)]
pub struct Fx(u64);
impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.0 = (self.0.rotate_left(5) ^ i as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_u32(&mut self, i: u32) {
        self.0 = (self.0.rotate_left(5) ^ i as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    fn write_u128(&mut self, i: u128) {
        self.write_u64(i as u64);
        self.write_u64((i >> 64) as u64);
    }
}
type Map<K, V> = HashMap<K, V, BuildHasherDefault<Fx>>;

// ------------------------------------------------------------------ rows and actions
pub const MAXK: usize = 6;
pub const MAXN: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Row {
    pub own: u8,
    pub cnt: [u8; MAXK],
}

impl Row {
    pub fn degree(&self) -> u8 {
        self.cnt.iter().sum()
    }
    pub fn present(&self, c: u8) -> bool {
        self.cnt[c as usize] > 0
    }
    /// `own.c0c1...c(k-1)`, one hexadecimal digit per colour count.
    pub fn text(&self, k: u8) -> String {
        let mut s = format!("{}.", self.own);
        for c in 0..k as usize {
            s.push(std::char::from_digit(self.cnt[c] as u32, 16).unwrap_or('f'));
        }
        s
    }
    pub fn parse(s: &str) -> Option<Row> {
        let (own, bag) = s.split_once('.')?;
        let own: u8 = own.trim().parse().ok()?;
        let mut cnt = [0u8; MAXK];
        for (c, ch) in bag.trim().chars().enumerate() {
            if c >= MAXK {
                return None;
            }
            cnt[c] = ch.to_digit(16)? as u8;
        }
        Some(Row { own, cnt })
    }
}

/// `target`: a colour, `STAY` (the agent keeps its place) or `STOP` (the
/// agent terminates on this vertex, after painting it).
pub const STAY: u8 = 255;
pub const STOP: u8 = 254;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Action {
    pub paint: u8,
    pub target: u8,
}

impl Action {
    pub fn text(&self) -> String {
        if self.target == STAY {
            format!("{}>stay", self.paint)
        } else if self.target == STOP {
            format!("{}>stop", self.paint)
        } else {
            format!("{}>{}", self.paint, self.target)
        }
    }
    pub fn parse(s: &str) -> Option<Action> {
        let (p, t) = s.trim().split_once('>')?;
        let paint: u8 = p.trim().parse().ok()?;
        let t = t.trim();
        let target = if t == "stay" || t == "s" || t == "-" {
            STAY
        } else if t == "stop" {
            STOP
        } else {
            // a number is a colour: 254 and 255 are the codes of stop and stay, not colours
            t.parse::<u8>().ok().filter(|&c| c < STOP)?
        };
        Some(Action { paint, target })
    }
}

// ------------------------------------------------------------------ graphs
#[derive(Clone, Debug)]
pub struct Graph {
    pub n: u8,
    pub s: u8,
    pub adj: Vec<u32>,
    pub edges: Vec<(u8, u8)>,
}

impl Graph {
    /// `n s u-v,u-v,...`
    pub fn parse(line: &str) -> Result<Graph, String> {
        let mut it = line.split_whitespace();
        let n: u8 = it.next().ok_or("graph: missing n")?.parse().map_err(|_| "graph: bad n")?;
        let s: u8 = it.next().ok_or("graph: missing start")?.parse().map_err(|_| "graph: bad start")?;
        if n == 0 || n as usize > MAXN {
            return Err(format!("graph: between 1 and {} vertices", MAXN));
        }
        if s >= n {
            return Err("graph: the start is not a vertex".into());
        }
        let mut adj = vec![0u32; n as usize];
        let mut edges = Vec::new();
        if let Some(es) = it.next() {
            for e in es.split(',').filter(|e| !e.trim().is_empty()) {
                let (u, v) = e.split_once('-').ok_or_else(|| format!("graph: bad edge {}", e))?;
                let u: u8 = u.trim().parse().map_err(|_| format!("graph: bad edge {}", e))?;
                let v: u8 = v.trim().parse().map_err(|_| format!("graph: bad edge {}", e))?;
                if u >= n || v >= n || u == v {
                    return Err(format!("graph: bad edge {}", e));
                }
                if adj[u as usize] >> v & 1 == 0 {
                    adj[u as usize] |= 1 << v;
                    adj[v as usize] |= 1 << u;
                    edges.push((u.min(v), u.max(v)));
                }
            }
        }
        let g = Graph { n, s, adj, edges };
        if !g.is_connected() {
            return Err("graph: not connected".into());
        }
        if (0..n).any(|v| g.degree(v) > 15) {
            return Err("graph: a vertex has more than 15 neighbours".into());
        }
        Ok(g)
    }
    pub fn neighbours(&self, v: u8) -> impl Iterator<Item = u8> + '_ {
        let mut m = self.adj[v as usize];
        std::iter::from_fn(move || {
            if m == 0 {
                None
            } else {
                let b = m.trailing_zeros() as u8;
                m &= m - 1;
                Some(b)
            }
        })
    }
    pub fn degree(&self, v: u8) -> u8 {
        self.adj[v as usize].count_ones() as u8
    }
    pub fn full(&self) -> u32 {
        if self.n as usize == 32 {
            u32::MAX
        } else {
            (1u32 << self.n) - 1
        }
    }
    pub fn is_connected(&self) -> bool {
        let mut seen: u32 = 1 << self.s;
        let mut frontier = seen;
        while frontier != 0 {
            let mut next = 0u32;
            let mut m = frontier;
            while m != 0 {
                let v = m.trailing_zeros() as usize;
                m &= m - 1;
                next |= self.adj[v] & !seen;
            }
            seen |= next;
            frontier = next;
        }
        seen == self.full()
    }
}

// ------------------------------------------------------------------ positions
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Pos {
    pub chi: u128,
    pub cur: u8,
    pub vis: u32,
    /// the agent has stopped (paper model): no move follows
    pub term: bool,
}

pub fn colour(chi: u128, v: u8) -> u8 {
    ((chi >> (4 * v as u32)) & 15) as u8
}
pub fn paint(chi: u128, v: u8, c: u8) -> u128 {
    (chi & !(15u128 << (4 * v as u32))) | ((c as u128) << (4 * v as u32))
}
pub fn row_at(g: &Graph, chi: u128, v: u8) -> Row {
    let mut cnt = [0u8; MAXK];
    for u in g.neighbours(v) {
        cnt[colour(chi, u) as usize] += 1;
    }
    Row { own: colour(chi, v), cnt }
}
/// The neighbours the adversary may serve for the target colour `t`.
pub fn options(g: &Graph, chi: u128, v: u8, t: u8) -> Vec<u8> {
    if t == STAY || t == STOP {
        return vec![];
    }
    g.neighbours(v).filter(|&u| colour(chi, u) == t).collect()
}

// ------------------------------------------------------------------ closed-form rules
#[derive(Clone, Copy, Debug)]
pub enum Formula {
    Sweep { chase: u8, target: u8, paint: u8 },
    FlipSweep4,
    FlipSweep4b,
    FlipSweep5,
    FlipSweep5D,
    Eat3,
    Chase3,
    /// paint own colour, chase white, else stay: the engine's default for unlisted rows
    ChaseWhite,
    /// A_Gen6 of Takahashi et al. (arXiv 2505.02789, Algorithm 2): six colours,
    /// simulates a semi-DFS and stops on the start vertex
    AGen6,
}

impl Formula {
    pub fn parse(name: &str) -> Option<Formula> {
        let name = name.trim();
        Some(match name {
            "sigma" | "sigma*" | "sweep(c0,t4,p0)" => Formula::Sweep { chase: 0, target: 4, paint: 0 },
            "flipsweep4" => Formula::FlipSweep4,
            "flipsweep4b" => Formula::FlipSweep4b,
            "flipsweep5" => Formula::FlipSweep5,
            "flipsweep5d" => Formula::FlipSweep5D,
            "eat3" => Formula::Eat3,
            "chase3" => Formula::Chase3,
            "chasewhite" | "chase" => Formula::ChaseWhite,
            "agen6" => Formula::AGen6,
            _ => {
                let body = name.strip_prefix("sweep")?;
                let body = body.trim_start_matches(['(', ':']).trim_end_matches(')');
                let parts: Vec<&str> = body.split(',').collect();
                if parts.len() != 3 {
                    return None;
                }
                let num = |s: &str| -> Option<u8> { s.trim().trim_start_matches(['c', 't', 'p']).parse().ok() };
                let (chase, target, paint) = (num(parts[0])?, num(parts[1])?, num(parts[2])?);
                if chase > 6 || target > 4 || paint > 6 {
                    return None;
                }
                Formula::Sweep { chase, target, paint }
            }
        })
    }

    /// The action on `row` with `k` colours; `None` when the formula does
    /// not fit the palette (a flip-sweep needs its named colours).
    pub fn action(&self, row: &Row, k: u8) -> Option<Action> {
        const W: u8 = 0;
        fn mv(paint: u8, target: u8) -> Action {
            Action { paint, target }
        }
        fn first(row: &Row, paint: u8, prefs: &[u8]) -> Action {
            for &c in prefs {
                if row.present(c) {
                    return mv(paint, c);
                }
            }
            mv(paint, STAY)
        }
        match *self {
            Formula::ChaseWhite => Some(if row.present(W) { mv(row.own, W) } else { mv(row.own, STAY) }),
            Formula::Sweep { chase, target, paint: pm } => {
                if k < 2 {
                    return None;
                }
                let nxt = |c: u8| if c == 0 { 1 } else { c % (k - 1) + 1 };
                let prv = |c: u8| if c <= 1 { k - 1 } else { c - 1 };
                let own = row.own;
                let nonwhite: Vec<u8> = (1..k).filter(|&c| row.present(c)).collect();
                if row.present(W) {
                    let p = match chase {
                        0..=3 => (chase + 1).min(k - 1),
                        4 => nxt(own),
                        5 => nonwhite.first().map(|&c| nxt(c)).unwrap_or(1),
                        _ => nonwhite.last().map(|&c| nxt(c)).unwrap_or(1),
                    };
                    return Some(mv(p, W));
                }
                if nonwhite.is_empty() {
                    return Some(mv(nxt(own), own));
                }
                let mn = nonwhite[0];
                let mx = *nonwhite.last().unwrap();
                let pick = |c: u8| if row.present(c) { c } else { mn };
                let t = match target {
                    0 => mn,
                    1 => mx,
                    2 => pick(own),
                    3 => pick(nxt(own)),
                    _ => pick(prv(own)),
                };
                let p = match pm {
                    0 => nxt(t),
                    1 => nxt(own),
                    2 => t,
                    p => (p - 2).min(k - 1),
                };
                Some(mv(p, t))
            }
            Formula::FlipSweep4 | Formula::FlipSweep4b => {
                if k < 4 {
                    return None;
                }
                const S: u8 = 1;
                const F: u8 = 2;
                const G: u8 = 3;
                let flip = matches!(self, Formula::FlipSweep4b);
                if row.present(W) {
                    return Some(mv(S, W));
                }
                Some(match row.own {
                    W | S => {
                        if row.present(S) {
                            mv(F, S)
                        } else if row.present(F) {
                            mv(G, F)
                        } else {
                            first(row, F, &[G])
                        }
                    }
                    F => {
                        if row.present(S) {
                            mv(if flip { G } else { F }, S)
                        } else if row.present(F) {
                            mv(G, F)
                        } else {
                            first(row, G, &[G])
                        }
                    }
                    _ => {
                        if row.present(S) {
                            mv(if flip { F } else { G }, S)
                        } else if row.present(G) {
                            mv(F, G)
                        } else {
                            first(row, F, &[F])
                        }
                    }
                })
            }
            Formula::FlipSweep5D => {
                if k < 5 {
                    return None;
                }
                const S: u8 = 1;
                const F: u8 = 2;
                const G: u8 = 3;
                const D: u8 = 4;
                if row.present(W) {
                    return Some(mv(S, W));
                }
                if row.degree() == 1 {
                    let c = (0..5u8).find(|&c| row.present(c)).unwrap_or(S);
                    return Some(mv(D, c));
                }
                Some(match row.own {
                    F => {
                        if row.present(S) {
                            mv(F, S)
                        } else if row.present(F) {
                            mv(G, F)
                        } else {
                            first(row, G, &[G])
                        }
                    }
                    G => {
                        if row.present(S) {
                            mv(G, S)
                        } else if row.present(G) {
                            mv(F, G)
                        } else {
                            first(row, F, &[F])
                        }
                    }
                    _ => {
                        if row.present(S) {
                            mv(F, S)
                        } else if row.present(F) {
                            mv(G, F)
                        } else {
                            first(row, F, &[G])
                        }
                    }
                })
            }
            Formula::FlipSweep5 => {
                if k < 5 {
                    return None;
                }
                const S: u8 = 1;
                const F: u8 = 2;
                const G: u8 = 3;
                const P: u8 = 4;
                if row.present(W) {
                    return Some(mv(if row.cnt[0] >= 2 { P } else { S }, W));
                }
                Some(match row.own {
                    F => {
                        if row.present(P) {
                            mv(F, P)
                        } else if row.present(S) {
                            mv(F, S)
                        } else if row.present(F) {
                            mv(G, F)
                        } else {
                            first(row, G, &[G])
                        }
                    }
                    G => {
                        if row.present(P) {
                            mv(G, P)
                        } else if row.present(S) {
                            mv(G, S)
                        } else if row.present(G) {
                            mv(F, G)
                        } else {
                            first(row, F, &[F])
                        }
                    }
                    _ => {
                        if row.present(P) {
                            mv(F, P)
                        } else if row.present(S) {
                            mv(F, S)
                        } else if row.present(F) {
                            mv(G, F)
                        } else {
                            first(row, F, &[G])
                        }
                    }
                })
            }
            Formula::Eat3 => {
                if k < 3 {
                    return None;
                }
                const A: u8 = 1;
                const B: u8 = 2;
                if row.present(W) {
                    return Some(mv(A, W));
                }
                Some(match row.own {
                    W | A => {
                        if row.present(A) {
                            mv(B, A)
                        } else {
                            first(row, B, &[B])
                        }
                    }
                    _ => {
                        if row.present(A) {
                            mv(B, A)
                        } else {
                            first(row, A, &[B])
                        }
                    }
                })
            }
            Formula::Chase3 => {
                if k < 3 {
                    return None;
                }
                const S: u8 = 1;
                const F: u8 = 2;
                if row.present(W) {
                    return Some(mv(S, W));
                }
                Some(if row.present(S) { mv(F, S) } else { first(row, F, &[F]) })
            }
            Formula::AGen6 => {
                if k < 6 {
                    return None;
                }
                // colours: 0 init, 1 path, 2 fin, 3 head1, 4 head2, 5 neigh.
                // Within one own colour the first matching rule wins (the paper's rule numbers in comments).
                const INIT: u8 = 0;
                const PATH: u8 = 1;
                const FIN: u8 = 2;
                const HEAD1: u8 = 3;
                const HEAD2: u8 = 4;
                const NEIGH: u8 = 5;
                Some(match row.own {
                    INIT => {
                        if row.present(PATH) {
                            mv(NEIGH, HEAD1) // rule 1
                        } else if row.present(HEAD1) {
                            mv(HEAD1, HEAD1) // rule 2
                        } else {
                            mv(HEAD1, STAY) // rule 3
                        }
                    }
                    HEAD1 => {
                        if row.present(INIT) && !row.present(HEAD1) {
                            mv(HEAD1, INIT) // rule 4
                        } else {
                            mv(HEAD2, STAY) // rule 5
                        }
                    }
                    HEAD2 => {
                        if row.present(NEIGH) {
                            mv(HEAD2, NEIGH) // rule 6
                        } else if row.present(HEAD1) {
                            mv(PATH, HEAD1) // rule 7
                        } else if row.present(PATH) {
                            mv(FIN, PATH) // rule 8
                        } else {
                            mv(FIN, STOP) // rule 9
                        }
                    }
                    PATH => mv(HEAD1, STAY), // rule 10
                    NEIGH => mv(INIT, HEAD2), // rule 11
                    // FIN: the paper defines no rule, the agent is on a fin vertex only after it has stopped
                    _ => return None,
                })
            }
        }
    }
}

// ------------------------------------------------------------------ the rule under test
pub struct Rule {
    pub k: u8,
    pub table: Map<Row, Action>,
    /// answers rows the table does not list; `None` leaves them undefined
    pub default: Option<Formula>,
    /// the paper's model (Takahashi et al., Section 2): `stop` ends the run, and
    /// exploring means every vertex visited and the agent stopped on the start
    /// vertex.  Off: the classic game, won as soon as every vertex is visited.
    pub terminating: bool,
}

pub enum Answer {
    Table(Action),
    Default(Action),
    Undefined,
}

impl Rule {
    /// The play is over: classic, every vertex is visited; paper model, the agent has stopped.
    pub fn over(&self, g: &Graph, p: &Pos) -> bool {
        if self.terminating {
            p.term
        } else {
            p.vis == g.full()
        }
    }
    /// The agent has won: classic, every vertex is visited; paper model, it stopped on the start
    /// vertex with every vertex visited (Definition 1 of Takahashi et al.).
    pub fn won(&self, g: &Graph, p: &Pos) -> bool {
        if self.terminating {
            p.term && p.cur == g.s && p.vis == g.full()
        } else {
            p.vis == g.full()
        }
    }
    pub fn answer(&self, row: &Row) -> Answer {
        if let Some(a) = self.table.get(row) {
            return Answer::Table(*a);
        }
        match self.default.and_then(|f| f.action(row, self.k)) {
            Some(a) => Answer::Default(a),
            None => Answer::Undefined,
        }
    }
    fn action(&self, row: &Row) -> Option<Action> {
        match self.answer(row) {
            Answer::Table(a) | Answer::Default(a) => Some(a),
            Answer::Undefined => None,
        }
    }
}

// ------------------------------------------------------------------ plays
#[derive(Clone, Debug)]
pub struct Step {
    pub cur: u8,
    pub row: Row,
    pub act: Action,
    pub options: Vec<u8>,
    pub next: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct Trace {
    pub steps: Vec<Step>,
    /// the step whose position recurs after the last step (`None`: the play
    /// ended by exploring or at an undefined row)
    pub repeat_at: Option<usize>,
    pub unvisited: u32,
    /// the vertex the agent terminated on (paper model), `None` if it did not stop
    pub stopped_at: Option<u8>,
}

pub enum Outcome {
    /// the start is in the agent's attractor: every adversary loses.  The
    /// trace is the adversary's longest obstruction.
    Explores { positions: usize, trace: Trace },
    /// a trapping play
    Fails { positions: usize, method: String, trace: Trace },
    /// the rule has no action for this row; `path` reaches it from the start
    Undefined { row: Row, path: Trace },
    /// more positions than the cap
    Overflow { positions: usize },
    /// walks only: every walk explored
    Unrefuted { walks: usize },
    /// play only: the step budget ran out before the play was decided
    Unfinished { trace: Trace },
}

fn successors(g: &Graph, p: &Pos, a: Action, terminating: bool) -> Vec<(Option<u8>, Pos)> {
    let chi = paint(p.chi, p.cur, a.paint);
    if terminating && a.target == STOP {
        // the agent paints its vertex and terminates there
        return vec![(None, Pos { chi, cur: p.cur, vis: p.vis, term: true })];
    }
    let opts = options(g, p.chi, p.cur, a.target);
    if opts.is_empty() {
        vec![(None, Pos { chi, cur: p.cur, vis: p.vis, term: false })]
    } else {
        opts.into_iter().map(|u| (Some(u), Pos { chi, cur: u, vis: p.vis | (1 << u), term: false })).collect()
    }
}

/// The exact game: every position the adversary can reach, the attractor of
/// the fully visited positions, and either the adversary's longest
/// obstruction (the rule explores) or a trapping play (it fails).
pub fn exact_game(g: &Graph, rule: &Rule, cap: usize) -> Outcome {
    let full = g.full();
    let root = Pos { chi: 0, cur: g.s, vis: 1 << g.s, term: false };
    let mut states = vec![root];
    let mut index: Map<Pos, u32> = Map::default();
    index.insert(root, 0);
    let mut parent: Vec<u32> = vec![0];
    let mut succ_start: Vec<u32> = vec![0];
    let mut succ: Vec<u32> = Vec::new();
    let mut i = 0usize;
    while i < states.len() {
        let p = states[i];
        if !rule.over(g, &p) {
            let row = row_at(g, p.chi, p.cur);
            let act = match rule.action(&row) {
                Some(a) => a,
                None => {
                    // the path from the root to this position, as a trace
                    let mut chain = vec![i as u32];
                    let mut j = i as u32;
                    while j != 0 {
                        j = parent[j as usize];
                        chain.push(j);
                    }
                    chain.reverse();
                    let mut steps = Vec::new();
                    for w in 0..chain.len() - 1 {
                        let q = states[chain[w] as usize];
                        let r = row_at(g, q.chi, q.cur);
                        let a = rule.action(&r).unwrap_or(Action { paint: q.cur, target: STAY });
                        let nxt = states[chain[w + 1] as usize];
                        let next = if nxt.cur != q.cur || nxt.vis != q.vis { Some(nxt.cur) } else { None };
                        steps.push(Step { cur: q.cur, row: r, act: a, options: options(g, q.chi, q.cur, a.target), next });
                    }
                    return Outcome::Undefined { row, path: Trace { steps, repeat_at: None, unvisited: full & !p.vis, stopped_at: None } };
                }
            };
            for (_, q) in successors(g, &p, act, rule.terminating) {
                let j = match index.get(&q) {
                    Some(&j) => j,
                    None => {
                        if states.len() >= cap {
                            return Outcome::Overflow { positions: states.len() };
                        }
                        let j = states.len() as u32;
                        states.push(q);
                        parent.push(i as u32);
                        index.insert(q, j);
                        j
                    }
                };
                succ.push(j);
            }
        }
        succ_start.push(succ.len() as u32);
        i += 1;
    }
    let n = states.len();
    let mut pred_start = vec![0u32; n + 1];
    for &j in &succ {
        pred_start[j as usize + 1] += 1;
    }
    for v in 1..=n {
        pred_start[v] += pred_start[v - 1];
    }
    let mut fill = pred_start.clone();
    let mut pred = vec![0u32; succ.len()];
    for p in 0..n {
        for e in succ_start[p]..succ_start[p + 1] {
            let j = succ[e as usize] as usize;
            pred[fill[j] as usize] = p as u32;
            fill[j] += 1;
        }
    }
    let mut remaining: Vec<u32> = (0..n).map(|p| succ_start[p + 1] - succ_start[p]).collect();
    let mut win = vec![false; n];
    let mut rank = vec![0u32; n];
    let mut queue: Vec<u32> = Vec::new();
    for p in 0..n {
        if rule.won(g, &states[p]) {
            win[p] = true;
            queue.push(p as u32);
        }
    }
    let mut head = 0;
    while head < queue.len() {
        let w = queue[head];
        head += 1;
        for e in pred_start[w as usize]..pred_start[w as usize + 1] {
            let p = pred[e as usize] as usize;
            if !win[p] {
                remaining[p] -= 1;
                if remaining[p] == 0 {
                    win[p] = true;
                    rank[p] = rank[w as usize] + 1;
                    queue.push(p as u32);
                }
            }
        }
    }
    let step_of = |p: &Pos, chosen: Option<u8>| -> Step {
        let row = row_at(g, p.chi, p.cur);
        let act = rule.action(&row).unwrap();
        Step { cur: p.cur, row, act, options: options(g, p.chi, p.cur, act.target), next: chosen }
    };
    if win[0] {
        let mut steps = Vec::new();
        let mut at = 0u32;
        while !rule.over(g, &states[at as usize]) {
            let p = states[at as usize];
            let act = rule.action(&row_at(g, p.chi, p.cur)).unwrap();
            let (u, j) = successors(g, &p, act, rule.terminating).into_iter().map(|(u, q)| (u, index[&q])).max_by_key(|&(_, j)| rank[j as usize]).unwrap();
            steps.push(step_of(&p, u));
            at = j;
        }
        let stopped_at = if rule.terminating { Some(states[at as usize].cur) } else { None };
        return Outcome::Explores { positions: n, trace: Trace { steps, repeat_at: None, unvisited: 0, stopped_at } };
    }
    let mut steps = Vec::new();
    let mut seen: Map<u32, usize> = Map::default();
    let mut at = 0u32;
    let repeat_at = loop {
        if let Some(&s) = seen.get(&at) {
            break Some(s);
        }
        seen.insert(at, steps.len());
        let p = states[at as usize];
        if p.term {
            break None; // the agent stopped where it should not have
        }
        let act = rule.action(&row_at(g, p.chi, p.cur)).unwrap();
        let mut choice: Option<(Option<u8>, u32)> = None;
        for (u, q) in successors(g, &p, act, rule.terminating) {
            let j = index[&q];
            if win[j as usize] {
                continue;
            }
            let better = match choice {
                None => true,
                Some((_, cj)) => q.vis != p.vis && states[cj as usize].vis == p.vis,
            };
            if better {
                choice = Some((u, j));
            }
        }
        let (u, j) = choice.expect("a losing position has a losing successor");
        steps.push(step_of(&p, u));
        at = j;
    };
    let end = states[at as usize];
    let unvisited = full & !end.vis;
    let stopped_at = if end.term { Some(end.cur) } else { None };
    Outcome::Fails { positions: n, method: "exact game".into(), trace: Trace { steps, repeat_at, unvisited, stopped_at } }
}

fn frontier_distances(g: &Graph, vis: u32) -> Vec<u32> {
    let n = g.n as usize;
    let mut d = vec![u32::MAX; n];
    let mut queue: Vec<usize> = Vec::new();
    for v in 0..n {
        if vis >> v & 1 == 0 {
            d[v] = 0;
            queue.push(v);
        }
    }
    let mut h = 0;
    while h < queue.len() {
        let v = queue[h];
        h += 1;
        for u in g.neighbours(v as u8) {
            if d[u as usize] == u32::MAX {
                d[u as usize] = d[v] + 1;
                queue.push(u as usize);
            }
        }
    }
    d
}

/// The automatic adversary's pick among `opts`: a visited vertex when there
/// is one, the one farthest from the unvisited region among those.
pub fn auto_choice(g: &Graph, vis: u32, opts: &[u8]) -> Option<u8> {
    if opts.is_empty() {
        return None;
    }
    let visited: Vec<u8> = opts.iter().copied().filter(|&u| vis >> u & 1 == 1).collect();
    let pool = if visited.is_empty() { opts.to_vec() } else { visited };
    let d = frontier_distances(g, vis);
    pool.iter().copied().max_by_key(|&u| d[u as usize])
}

/// One adversary walk.  Heuristic 0: smallest visited target; 1: visited
/// target farthest from the frontier; 2: largest visited target; other: a
/// seeded random visited target.  `Err` carries an undefined row and the
/// path that reached it.
pub fn walk(g: &Graph, rule: &Rule, heuristic: u8, seed: u64, budget: usize) -> Result<Option<Trace>, (Row, Trace)> {
    let mut rng = seed.max(1);
    let (end, trace) = play_with(g, rule, budget, |g, p, sucs| {
        if sucs.len() == 1 {
            return 0;
        }
        let visited: Vec<usize> = (0..sucs.len()).filter(|&i| sucs[i].1.vis == p.vis).collect();
        let pool: Vec<usize> = if visited.is_empty() { (0..sucs.len()).collect() } else { visited };
        match heuristic {
            0 => pool[0],
            1 => {
                let d = frontier_distances(g, p.vis);
                *pool.iter().max_by_key(|&&i| d[sucs[i].1.cur as usize]).unwrap()
            }
            2 => *pool.last().unwrap(),
            _ => {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                pool[(rng % pool.len() as u64) as usize]
            }
        }
    });
    match end {
        PlayEnd::Won | PlayEnd::Budget => Ok(None),
        PlayEnd::Lost => Ok(Some(trace)),
        PlayEnd::Undefined(row) => Err((row, trace)),
    }
}

/// How a single play ended.
enum PlayEnd {
    /// the agent won (see `Rule::won`)
    Won,
    /// it stopped in the wrong place, or the position repeated
    Lost,
    /// the rule has no action for this row
    Undefined(Row),
    /// the step budget ran out with the play undecided
    Budget,
}

/// One execution of `rule` on `g`: `choose` picks among the adversary's options (the successors,
/// more than one only when several neighbours carry the target colour) and returns an index.
/// The trace holds every step taken, whatever the end.
fn play_with(g: &Graph, rule: &Rule, budget: usize, mut choose: impl FnMut(&Graph, &Pos, &[(Option<u8>, Pos)]) -> usize) -> (PlayEnd, Trace) {
    let full = g.full();
    let mut p = Pos { chi: 0, cur: g.s, vis: 1 << g.s, term: false };
    let mut seen: Map<Pos, usize> = Map::default();
    let mut steps: Vec<Step> = Vec::new();
    loop {
        let trace = |steps: Vec<Step>, repeat_at: Option<usize>, stopped: bool| Trace { steps, repeat_at, unvisited: full & !p.vis, stopped_at: if stopped { Some(p.cur) } else { None } };
        if rule.over(g, &p) {
            let end = if rule.won(g, &p) { PlayEnd::Won } else { PlayEnd::Lost };
            // over only happens in the paper model, by a stop
            return (end, trace(steps, None, p.term));
        }
        // out of budget: decided only after the position above was looked at
        if steps.len() >= budget {
            return (PlayEnd::Budget, trace(steps, None, false));
        }
        if let Some(&s) = seen.get(&p) {
            return (PlayEnd::Lost, trace(steps, Some(s), false));
        }
        seen.insert(p, steps.len());
        let row = row_at(g, p.chi, p.cur);
        let act = match rule.action(&row) {
            Some(a) => a,
            None => return (PlayEnd::Undefined(row), trace(steps, None, false)),
        };
        let sucs = successors(g, &p, act, rule.terminating);
        let opts: Vec<u8> = sucs.iter().filter_map(|(u, _)| *u).collect();
        let (u, q) = sucs[choose(g, &p, &sucs)];
        steps.push(Step { cur: p.cur, row, act, options: opts, next: u });
        p = q;
    }
}

/// One play under an adversary that prefers the vertices in `pref`, in that order (a vertex
/// not listed comes last, the smaller number first).  Reproduces a hand trace.
pub fn play(g: &Graph, rule: &Rule, pref: &[u8], budget: usize) -> Outcome {
    let rank = |u: u8| pref.iter().position(|&x| x == u).unwrap_or(usize::MAX);
    let (end, trace) = play_with(g, rule, budget, |_, _, sucs| (0..sucs.len()).min_by_key(|&i| sucs[i].0.map_or(usize::MAX, rank)).unwrap());
    match end {
        PlayEnd::Won => Outcome::Explores { positions: 0, trace },
        PlayEnd::Lost => Outcome::Fails { positions: 0, method: format!("play: adversary prefers {}", if pref.is_empty() { "the smallest vertex".to_string() } else { pref.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ") }), trace },
        PlayEnd::Undefined(row) => Outcome::Undefined { row, path: trace },
        PlayEnd::Budget => Outcome::Unfinished { trace },
    }
}

pub fn walks(g: &Graph, rule: &Rule, random: usize, budget: usize) -> Outcome {
    let plan: Vec<(u8, u64)> = [(0u8, 0u64), (1, 0), (2, 0)].into_iter().chain((1..=random as u64).map(|s| (3u8, s))).collect();
    for &(h, seed) in &plan {
        match walk(g, rule, h, seed, budget) {
            Ok(Some(trace)) => {
                let method = match h {
                    0 => "walk: smallest visited target".to_string(),
                    1 => "walk: visited target farthest from the frontier".to_string(),
                    2 => "walk: largest visited target".to_string(),
                    _ => format!("walk: random visited target, seed {}", seed),
                };
                return Outcome::Fails { positions: 0, method, trace };
            }
            Ok(None) => {}
            Err((row, path)) => return Outcome::Undefined { row, path },
        }
    }
    Outcome::Unrefuted { walks: plan.len() }
}

// ------------------------------------------------------------------ JSON out
fn js(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}
fn list(v: &[u8]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}
fn mask_list(m: u32) -> String {
    list(&(0..32u8).filter(|&v| m >> v & 1 == 1).collect::<Vec<_>>())
}
fn target_json(t: u8) -> String {
    if t == STAY {
        "-1".into()
    } else if t == STOP {
        "-2".into()
    } else {
        t.to_string()
    }
}
fn trace_json(t: &Trace, k: u8) -> String {
    let steps: Vec<String> = t
        .steps
        .iter()
        .map(|s| format!("{{\"cur\":{},\"row\":{},\"paint\":{},\"target\":{},\"options\":{},\"next\":{}}}", s.cur, js(&s.row.text(k)), s.act.paint, target_json(s.act.target), list(&s.options), s.next.map_or("null".to_string(), |u| u.to_string())))
        .collect();
    format!("{{\"repeat_at\":{},\"unvisited\":{},\"stopped_at\":{},\"steps\":[{}]}}", t.repeat_at.map_or("null".to_string(), |r| r.to_string()), mask_list(t.unvisited), t.stopped_at.map_or("null".to_string(), |v| v.to_string()), steps.join(","))
}
/// Why a play ended badly in the paper model.
fn fail_reason(t: &Trace) -> &'static str {
    match t.stopped_at {
        None => "never_stops",
        Some(_) if t.unvisited != 0 => "stopped_early",
        // a stop on the start vertex with every vertex visited is a win, so it is never a failure
        Some(_) => "stopped_off_start",
    }
}
/// Paper model: the three parts of exploring, reported separately.
fn indicators(t: &Trace, start: u8) -> String {
    format!("{{\"visited_all\":{},\"stopped\":{},\"stopped_at_start\":{}}}", t.unvisited == 0, t.stopped_at.is_some(), t.stopped_at == Some(start))
}
/// `paper`: the outcome is for the paper's model, which adds `model`, and for a trace also
/// `indicators` and (for a failure) `reason`.
fn outcome_json(o: &Outcome, k: u8, paper: bool, start: u8) -> String {
    let model = format!("\"model\":\"{}\"", if paper { "paper" } else { "classic" });
    let extra = |t: &Trace, fails: bool| -> String {
        if !paper {
            return String::new();
        }
        let reason = if fails { format!(",\"reason\":\"{}\"", fail_reason(t)) } else { String::new() };
        format!(",\"indicators\":{}{}", indicators(t, start), reason)
    };
    match o {
        Outcome::Explores { positions, trace } => format!("{{\"status\":\"explores\",{},\"positions\":{}{},\"trace\":{}}}", model, positions, extra(trace, false), trace_json(trace, k)),
        Outcome::Fails { positions, method, trace } => format!("{{\"status\":\"fails\",{},\"positions\":{},\"method\":{}{},\"trace\":{}}}", model, positions, js(method), extra(trace, true), trace_json(trace, k)),
        Outcome::Undefined { row, path } => format!("{{\"status\":\"undefined\",{},\"row\":{},\"path\":{}}}", model, js(&row.text(k)), trace_json(path, k)),
        Outcome::Overflow { positions } => format!("{{\"status\":\"overflow\",{},\"positions\":{}}}", model, positions),
        Outcome::Unrefuted { walks } => format!("{{\"status\":\"unrefuted\",{},\"walks\":{}}}", model, walks),
        Outcome::Unfinished { trace } => format!("{{\"status\":\"unfinished\",{},\"trace\":{}}}", model, trace_json(trace, k)),
    }
}
fn err(msg: &str) -> String {
    format!("{{\"status\":\"error\",\"message\":{}}}", js(msg))
}

// ------------------------------------------------------------------ the request protocol
/// Lines of `key value`.  Keys: `cmd` (`step`, `exact`, `walks`, `play`, `answer`),
/// `k`, `graph` (`n s u-v,...`), `default` (a formula name), `cap`, `walks`,
/// `budget`, `colours` (space-separated, for `step`), `cur`, `vis`
/// (space-separated visited vertices), `row` (for `answer`), `model`
/// (`classic` or `paper`; default `classic`, or `paper` when the default rule
/// is `agen6`); the block
/// between a line `table` and a line `end` lists `row action` pairs.
struct Request {
    cmd: String,
    k: u8,
    graph: Option<Result<Graph, String>>,
    table: Map<Row, Action>,
    bad_rows: Vec<String>,
    default: Option<Formula>,
    bad_default: Option<String>,
    cap: usize,
    walks: usize,
    budget: usize,
    colours: Vec<u8>,
    cur: Option<u8>,
    vis: Vec<u8>,
    row: Option<String>,
    model: String,
    prefer: Vec<u8>,
    bad_prefer: Vec<String>,
}

fn parse_request(input: &str) -> Request {
    let mut r = Request { cmd: String::new(), k: 5, graph: None, table: Map::default(), bad_rows: Vec::new(), default: None, bad_default: None, cap: 1_000_000, walks: 64, budget: 200_000, colours: Vec::new(), cur: None, vis: Vec::new(), row: None, model: String::new(), prefer: Vec::new(), bad_prefer: Vec::new() };
    let mut in_table = false;
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if in_table {
            if line == "end" {
                in_table = false;
                continue;
            }
            let mut it = line.split_whitespace();
            match (it.next().and_then(Row::parse), it.next().and_then(Action::parse)) {
                (Some(row), Some(act)) => {
                    r.table.insert(row, act);
                }
                _ => r.bad_rows.push(line.to_string()),
            }
            continue;
        }
        let (key, val) = match line.split_once(char::is_whitespace) {
            Some((k, v)) => (k, v.trim()),
            None => (line, ""),
        };
        match key {
            "cmd" => r.cmd = val.to_string(),
            "k" => r.k = val.parse().unwrap_or(5),
            "graph" => r.graph = Some(Graph::parse(val)),
            "default" => {
                if val.is_empty() || val == "none" {
                    r.default = None;
                } else {
                    match Formula::parse(val) {
                        Some(f) => r.default = Some(f),
                        None => r.bad_default = Some(val.to_string()),
                    }
                }
            }
            "cap" => r.cap = val.parse().unwrap_or(1_000_000),
            "walks" => r.walks = val.parse().unwrap_or(64),
            "budget" => r.budget = val.parse().unwrap_or(200_000),
            "colours" => r.colours = val.split_whitespace().filter_map(|x| x.parse().ok()).collect(),
            "cur" => r.cur = val.parse().ok(),
            "vis" => r.vis = val.split_whitespace().filter_map(|x| x.parse().ok()).collect(),
            "row" => r.row = Some(val.to_string()),
            "model" => r.model = val.to_string(),
            "prefer" => {
                // a later `prefer` line replaces an earlier one; a token that is not a vertex number is an error
                r.prefer.clear();
                for tok in val.split_whitespace() {
                    match tok.parse::<u8>() {
                        Ok(v) => r.prefer.push(v),
                        Err(_) => r.bad_prefer.push(tok.to_string()),
                    }
                }
            }
            "table" => in_table = true,
            _ => {}
        }
    }
    r
}

/// Whether the request runs the paper's model (see `Rule::terminating`).
/// With no `model` key it is the paper model whenever the rule can stop (`default agen6`,
/// or a table row answering `stop`): judged as the classic game a stop would silently mean
/// "stay" and the verdict would ignore where, or whether, the agent stops.
fn resolve_model(model: &str, default: &Option<Formula>, table_can_stop: bool) -> Result<bool, String> {
    match model {
        "paper" => Ok(true),
        "classic" => Ok(false),
        "" => Ok(table_can_stop || matches!(default, Some(Formula::AGen6))),
        other => Err(format!("unknown model {} (classic or paper)", other)),
    }
}

pub fn handle(input: &str) -> String {
    let r = parse_request(input);
    if !(2..=MAXK as u8).contains(&r.k) {
        return err(&format!("k must be between 2 and {}", MAXK));
    }
    if let Some(bad) = &r.bad_default {
        return err(&format!("unknown default rule {}", bad));
    }
    if matches!(r.default, Some(Formula::AGen6)) && r.k < 6 {
        return err("agen6 needs 6 colours (k 6)");
    }
    if !r.bad_rows.is_empty() {
        return err(&format!("bad table line: {}", r.bad_rows[0]));
    }
    if let Some(tok) = r.bad_prefer.first() {
        return err(&format!("prefer: {} is not a vertex number", tok));
    }
    for (row, act) in &r.table {
        if row.own >= r.k || act.paint >= r.k || (act.target != STAY && act.target != STOP && act.target >= r.k) || row.cnt[r.k as usize..].iter().any(|&c| c > 0) {
            return err(&format!("table entry {} {} uses a colour outside 0..{}", row.text(r.k), act.text(), r.k - 1));
        }
    }
    let table_can_stop = r.table.values().any(|a| a.target == STOP);
    let terminating = match resolve_model(&r.model, &r.default, table_can_stop) {
        Ok(t) => t,
        Err(e) => return err(&e),
    };
    let rule = Rule { k: r.k, table: r.table, default: r.default, terminating };
    if r.cmd == "answer" {
        // the rule's answer on one row, for the rule editor
        let Some(text) = r.row.as_deref().and_then(Row::parse) else { return err("row: expected own.bag") };
        return match rule.answer(&text) {
            Answer::Table(a) => format!("{{\"status\":\"ok\",\"defined\":true,\"source\":\"table\",\"paint\":{},\"target\":{}}}", a.paint, target_json(a.target)),
            Answer::Default(a) => format!("{{\"status\":\"ok\",\"defined\":true,\"source\":\"default\",\"paint\":{},\"target\":{}}}", a.paint, target_json(a.target)),
            Answer::Undefined => "{\"status\":\"ok\",\"defined\":false}".to_string(),
        };
    }
    let g = match r.graph {
        Some(Ok(g)) => g,
        Some(Err(e)) => return err(&e),
        None => return err("graph: missing"),
    };
    match r.cmd.as_str() {
        "step" => {
            // the rule's move from a given position
            let Some(cur) = r.cur else { return err("cur: missing") };
            if cur >= g.n || r.colours.len() != g.n as usize || r.colours.iter().any(|&c| c >= r.k) {
                return err("step: bad position");
            }
            let mut chi = 0u128;
            for (v, &c) in r.colours.iter().enumerate() {
                chi = paint(chi, v as u8, c);
            }
            let mut vis = 1u32 << g.s;
            for &v in &r.vis {
                if v < g.n {
                    vis |= 1 << v;
                }
            }
            let row = row_at(&g, chi, cur);
            let (a, source) = match rule.answer(&row) {
                Answer::Table(a) => (a, "table"),
                Answer::Default(a) => (a, "default"),
                Answer::Undefined => return format!("{{\"status\":\"ok\",\"row\":{},\"degree\":{},\"defined\":false}}", js(&row.text(r.k)), row.degree()),
            };
            let opts = options(&g, chi, cur, a.target);
            let auto = auto_choice(&g, vis, &opts);
            format!("{{\"status\":\"ok\",\"row\":{},\"degree\":{},\"defined\":true,\"source\":\"{}\",\"paint\":{},\"target\":{},\"options\":{},\"auto\":{},\"explored\":{}}}", js(&row.text(r.k)), row.degree(), source, a.paint, target_json(a.target), list(&opts), auto.map_or("null".to_string(), |u| u.to_string()), vis == g.full())
        }
        "exact" => {
            let cap = r.cap.clamp(1_000, 12_000_000);
            outcome_json(&exact_game(&g, &rule, cap), r.k, rule.terminating, g.s)
        }
        "walks" => outcome_json(&walks(&g, &rule, r.walks.min(100_000), r.budget.clamp(10, 5_000_000)), r.k, rule.terminating, g.s),
        "play" => {
            // one execution under an adversary with a stated preference order (`prefer v1 v2 ...`)
            if let Some(&bad) = r.prefer.iter().find(|&&v| v >= g.n) {
                return err(&format!("prefer: vertex {} is not in the graph (0..{})", bad, g.n - 1));
            }
            outcome_json(&play(&g, &rule, &r.prefer, r.budget.clamp(10, 5_000_000)), r.k, rule.terminating, g.s)
        }
        other => err(&format!("unknown cmd {}", other)),
    }
}

// ------------------------------------------------------------------ the C ABI
/// A buffer of `len` bytes the caller fills with the request.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Frees a buffer from `alloc` (`len` as requested).
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

/// Runs a request; returns a buffer holding a little-endian `u32` length
/// followed by that many bytes of UTF-8 JSON.  Free it with `free_result`.
#[no_mangle]
pub unsafe extern "C" fn run(ptr: *const u8, len: usize) -> *mut u8 {
    let input = std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).unwrap_or("");
    let out = handle(input).into_bytes();
    let mut buf: Vec<u8> = Vec::with_capacity(out.len() + 4);
    buf.extend_from_slice(&(out.len() as u32).to_le_bytes());
    buf.extend_from_slice(&out);
    let p = buf.as_mut_ptr();
    std::mem::forget(buf);
    p
}

#[no_mangle]
pub unsafe extern "C" fn free_result(ptr: *mut u8) {
    let n = u32::from_le_bytes([*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)]) as usize;
    drop(Vec::from_raw_parts(ptr, 0, n + 4));
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKETCH1: &str = "17 0 0-1,1-2,0-2,0-3,2-3,0-4,4-5,3-6,4-6,5-6,5-7,6-8,7-8,5-9,9-14,9-15,9-10,10-16,5-10,9-11,10-11,5-11,7-11,11-12,11-13,12-13";

    fn rule(k: u8, f: &str) -> Rule {
        Rule { k, table: Map::default(), default: Formula::parse(f), terminating: false }
    }

    #[test]
    fn sigma_star_explores_sketch_one_and_flipsweep_does_not() {
        let g = Graph::parse(SKETCH1).unwrap();
        match exact_game(&g, &rule(5, "sigma"), 3_000_000) {
            Outcome::Explores { positions, trace } => {
                assert_eq!(positions, 1_601_969);
                assert_eq!(trace.steps.len(), 55);
            }
            _ => panic!("sigma* must explore Sketch I"),
        }
        match exact_game(&g, &rule(5, "flipsweep5d"), 3_000_000) {
            Outcome::Fails { positions, trace, .. } => {
                assert_eq!(positions, 1_291_525);
                assert_eq!(trace.unvisited, (1 << 14) | (1 << 15) | (1 << 16));
            }
            _ => panic!("flipsweep5d must fail on Sketch I"),
        }
    }

    #[test]
    fn stop_action_parses_prints_and_validates() {
        let a = Action::parse("2>stop").unwrap();
        assert_eq!(a, Action { paint: 2, target: STOP });
        assert_eq!(a.text(), "2>stop");
        assert_eq!(Action::parse("3>stay").unwrap().text(), "3>stay");
        assert_eq!(Action::parse("3>7").unwrap().target, 7);
        // the numeric codes of the two special targets are not colours
        assert_eq!(Action::parse("2>254"), None);
        assert_eq!(Action::parse("2>255"), None);
        assert_eq!(Action::parse("2>5").unwrap().target, 5);
        assert_eq!(Action::parse("2>s").unwrap().target, STAY);
        // a table row may answer `stop` (not rejected as an out-of-range colour); the answer reports target -2
        let out = handle("cmd answer\nk 6\nrow 4.000000\ntable\n4.000000 2>stop\nend\n");
        assert!(out.contains("\"defined\":true") && out.contains("\"paint\":2") && out.contains("\"target\":-2"), "{}", out);
        // the adversary is never offered a move for stop
        assert!(options(&Graph::parse("2 0 0-1").unwrap(), 0, 0, STOP).is_empty());
    }

    // (row read, action taken) at each step, from workbench/notes/agen6-hand-trace.xlsx
    const PATH_TRACE: [(&str, &str); 17] = [
        ("0.100000", "3>stay"), ("3.100000", "3>0"), ("0.100100", "3>3"), ("3.000100", "4>stay"),
        ("4.000100", "1>3"), ("3.110000", "3>0"), ("0.000100", "3>3"), ("3.010100", "4>stay"),
        ("4.010100", "1>3"), ("3.010000", "4>stay"), ("4.010000", "2>1"), ("1.011000", "3>stay"),
        ("3.011000", "4>stay"), ("4.011000", "2>1"), ("1.001000", "3>stay"), ("3.001000", "4>stay"),
        ("4.001000", "2>stop"),
    ];

    // (row read, action taken) at each step, from workbench/notes/agen6-hand-trace.xlsx
    const TRIANGLE_TRACE: [(&str, &str); 21] = [
        ("0.200000", "3>stay"), ("3.200000", "3>0"), ("0.100100", "3>3"), ("3.100100", "4>stay"),
        ("4.100100", "1>3"), ("3.110000", "3>0"), ("0.010100", "5>3"), ("3.010001", "4>stay"),
        ("4.010001", "4>5"), ("5.010010", "0>4"), ("4.110000", "2>1"), ("1.101000", "3>stay"),
        ("3.101000", "3>0"), ("0.001100", "3>3"), ("3.001100", "4>stay"), ("4.001100", "1>3"),
        ("3.011000", "4>stay"), ("4.011000", "2>1"), ("1.002000", "3>stay"), ("3.002000", "4>stay"),
        ("4.002000", "2>stop"),
    ];

    // (row read, action taken) at each step, from workbench/notes/agen6-hand-trace.xlsx
    const TREE_TRACE: [(&str, &str); 24] = [
        ("0.200000", "3>stay"), ("3.200000", "3>0"), ("0.100100", "3>3"), ("3.100100", "4>stay"),
        ("4.100100", "1>3"), ("3.110000", "3>0"), ("0.000100", "3>3"), ("3.010100", "4>stay"),
        ("4.010100", "1>3"), ("3.010000", "4>stay"), ("4.010000", "2>1"), ("1.011000", "3>stay"),
        ("3.011000", "4>stay"), ("4.011000", "2>1"), ("1.101000", "3>stay"), ("3.101000", "3>0"),
        ("0.000100", "3>3"), ("3.001100", "4>stay"), ("4.001100", "1>3"), ("3.010000", "4>stay"),
        ("4.010000", "2>1"), ("1.002000", "3>stay"), ("3.002000", "4>stay"), ("4.002000", "2>stop"),
    ];

    /// Runs A_Gen6 the way the paper's model says, one action at a time, and
    /// returns the (row, action) log, the final colouring and the final vertex.
    /// `pref` lists the vertices in the adversary's order of preference.
    fn run_agen6(g: &Graph, pref: &[u8]) -> (Vec<(String, String)>, u128, u8) {
        let f = Formula::AGen6;
        let (mut chi, mut cur) = (0u128, g.s);
        let mut log = Vec::new();
        for _ in 0..1000 {
            let row = row_at(g, chi, cur);
            let act = f.action(&row, 6).expect("A_Gen6 has no rule for this row");
            log.push((row.text(6), act.text()));
            let opts = options(g, chi, cur, act.target);
            chi = paint(chi, cur, act.paint);
            if act.target == STOP {
                return (log, chi, cur);
            }
            if act.target != STAY {
                assert!(!opts.is_empty(), "A_Gen6 targeted a colour no neighbour has");
                cur = *opts.iter().min_by_key(|u| pref.iter().position(|p| p == *u).unwrap()).unwrap();
            }
        }
        panic!("A_Gen6 did not stop");
    }

    fn all_fin_at_start(g: &Graph, chi: u128, cur: u8) {
        assert_eq!(cur, g.s, "must stop on the start vertex");
        for v in 0..g.n {
            assert_eq!(colour(chi, v), 2, "vertex {} is not fin", v);
        }
    }

    fn check_trace(g: &Graph, pref: &[u8], expected: &[(&str, &str)]) {
        let (log, chi, cur) = run_agen6(g, pref);
        let got: Vec<(&str, &str)> = log.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        assert_eq!(got, expected);
        all_fin_at_start(g, chi, cur);
    }

    #[test]
    fn agen6_reproduces_the_hand_traces() {
        check_trace(&Graph::parse("3 0 0-1,1-2").unwrap(), &[0, 1, 2], &PATH_TRACE);
        check_trace(&Graph::parse("3 0 0-1,1-2,0-2").unwrap(), &[0, 1, 2], &TRIANGLE_TRACE);
        check_trace(&Graph::parse("4 0 0-1,0-2,1-3").unwrap(), &[1, 2, 3, 0], &TREE_TRACE);
    }

    #[test]
    fn agen6_finishes_the_tree_under_the_other_tie_choice() {
        let g = Graph::parse("4 0 0-1,0-2,1-3").unwrap();
        let (log, chi, cur) = run_agen6(&g, &[2, 1, 3, 0]);
        assert_eq!(log.len(), 24);
        all_fin_at_start(&g, chi, cur);
    }

    #[test]
    fn agen6_rules_priority_and_gaps() {
        let ans = |k: u8, row: &str| handle(&format!("cmd answer\nk {}\nrow {}\ndefault agen6\n", k, row));
        // rule 3 (start vertex), rule 1 before rule 2, rule 4 and 5, rule 9
        assert!(ans(6, "0.100000").contains("\"paint\":3,\"target\":-1"));
        assert!(ans(6, "0.010100").contains("\"paint\":5,\"target\":3"));
        assert!(ans(6, "3.100000").contains("\"paint\":3,\"target\":0"));
        assert!(ans(6, "3.110100").contains("\"paint\":4,\"target\":-1"));
        assert!(ans(6, "4.001000").contains("\"paint\":2,\"target\":-2"));
        // rule 6 before rule 7 before rule 8
        assert!(ans(6, "4.010101").contains("\"paint\":4,\"target\":5"));
        assert!(ans(6, "4.010100").contains("\"paint\":1,\"target\":3"));
        // the paper defines no rule for fin; A_Gen6 with fewer than six colours is an error
        assert!(ans(6, "2.000000").contains("\"defined\":false"));
        assert!(ans(5, "0.10000").contains("6 colours"), "A_Gen6 needs six colours: an error, not an undefined row");
    }

    fn paper_rule_from_table(k: u8, rows: &[(&str, &str)]) -> Rule {
        let mut table = Map::default();
        for (r, a) in rows {
            table.insert(Row::parse(r).unwrap(), Action::parse(a).unwrap());
        }
        Rule { k, table, default: None, terminating: true }
    }

    #[test]
    fn exact_game_explores_with_agen6_from_every_start() {
        let rule = Rule { k: 6, table: Map::default(), default: Some(Formula::AGen6), terminating: true };
        for (name, n, edges) in [("path", 3u8, "0-1,1-2"), ("triangle", 3, "0-1,1-2,0-2"), ("tree", 4, "0-1,0-2,1-3")] {
            for start in 0..n {
                let g = Graph::parse(&format!("{} {} {}", n, start, edges)).unwrap();
                match exact_game(&g, &rule, 1_000_000) {
                    Outcome::Explores { trace, .. } => {
                        assert_eq!(trace.stopped_at, Some(start), "{} from {} must stop on the start", name, start);
                        assert_eq!(trace.unvisited, 0);
                        assert_eq!(trace.steps.last().unwrap().act.target, STOP);
                    }
                    _ => panic!("A_Gen6 must explore the {} from vertex {}", name, start),
                }
            }
        }
        // the path from its end vertex has no choice for the adversary: 17 actions, as in the hand trace
        let g = Graph::parse("3 0 0-1,1-2").unwrap();
        match exact_game(&g, &rule, 1_000_000) {
            Outcome::Explores { trace, .. } => assert_eq!(trace.steps.len(), 17),
            _ => panic!(),
        }
    }

    #[test]
    fn paper_model_wants_a_stop_where_the_classic_game_does_not() {
        // sigma* visits every vertex of a short path but never stops
        let g = Graph::parse("4 0 0-1,1-2,2-3").unwrap();
        let mut r = rule(5, "sigma");
        assert!(matches!(exact_game(&g, &r, 100_000), Outcome::Explores { .. }));
        r.terminating = true;
        match exact_game(&g, &r, 100_000) {
            Outcome::Fails { trace, .. } => {
                assert!(trace.repeat_at.is_some(), "it circles forever");
                assert_eq!(trace.stopped_at, None);
            }
            _ => panic!("a rule that never stops cannot explore in the paper model"),
        }
    }

    #[test]
    fn a_stop_off_the_start_vertex_is_a_failure() {
        // two vertices: the agent walks to vertex 1, visits everything, and stops there
        let g = Graph::parse("2 0 0-1").unwrap();
        let r = paper_rule_from_table(2, &[("0.10", "1>0"), ("0.01", "0>stop")]);
        match exact_game(&g, &r, 1000) {
            Outcome::Fails { trace, .. } => {
                assert_eq!(trace.stopped_at, Some(1));
                assert_eq!(trace.unvisited, 0, "every vertex was visited, but it stopped in the wrong place");
                assert_eq!(trace.repeat_at, None);
            }
            _ => panic!("stopping away from the start must fail"),
        }
    }

    #[test]
    fn a_stop_before_every_vertex_is_visited_is_a_failure() {
        let g = Graph::parse("2 0 0-1").unwrap();
        let r = paper_rule_from_table(2, &[("0.10", "1>stop")]);
        match exact_game(&g, &r, 1000) {
            Outcome::Fails { trace, .. } => {
                assert_eq!(trace.stopped_at, Some(0));
                assert_eq!(trace.unvisited, 1 << 1);
            }
            _ => panic!("stopping early must fail"),
        }
    }

    #[test]
    fn a_rule_that_returns_and_stops_on_the_start_explores() {
        // vertex 0 -> 1 (paints 0 white->1), vertex 1 sends the agent back, vertex 0 stops
        let g = Graph::parse("2 0 0-1").unwrap();
        let r = paper_rule_from_table(2, &[("0.10", "1>0"), ("0.01", "0>1"), ("1.10", "1>stop")]);
        match exact_game(&g, &r, 1000) {
            Outcome::Explores { trace, .. } => {
                assert_eq!(trace.stopped_at, Some(0));
                assert_eq!(trace.steps.len(), 3);
            }
            _ => panic!("visiting everything and stopping on the start must explore"),
        }
    }

    #[test]
    fn walks_use_the_paper_win_condition_too() {
        let a6 = Rule { k: 6, table: Map::default(), default: Some(Formula::AGen6), terminating: true };
        for (n, edges) in [(3u8, "0-1,1-2"), (3, "0-1,1-2,0-2"), (4, "0-1,0-2,1-3")] {
            let g = Graph::parse(&format!("{} 0 {}", n, edges)).unwrap();
            assert!(matches!(walks(&g, &a6, 8, 10_000), Outcome::Unrefuted { .. }), "A_Gen6 survives every walk on {}", edges);
        }
        // a stop away from the start is caught by a walk
        let g = Graph::parse("2 0 0-1").unwrap();
        let wrong = paper_rule_from_table(2, &[("0.10", "1>0"), ("0.01", "0>stop")]);
        match walks(&g, &wrong, 2, 1000) {
            Outcome::Fails { trace, .. } => {
                assert_eq!(trace.stopped_at, Some(1));
                assert_eq!(trace.repeat_at, None);
            }
            _ => panic!("the walk must see the stop on vertex 1"),
        }
        // a rule that never stops is caught by a repeat
        let g = Graph::parse("4 0 0-1,1-2,2-3").unwrap();
        let mut sigma = rule(5, "sigma");
        assert!(matches!(walks(&g, &sigma, 2, 10_000), Outcome::Unrefuted { .. }), "classic: it visits everything");
        sigma.terminating = true;
        assert!(matches!(walks(&g, &sigma, 2, 10_000), Outcome::Fails { .. }), "paper model: it never stops");
    }

    #[test]
    fn results_report_the_paper_conditions_separately() {
        // A_Gen6 explores: stopped on the start, everything visited
        let out = handle("cmd exact\nk 6\ngraph 3 0 0-1,1-2\ndefault agen6\n");
        assert!(out.contains("\"status\":\"explores\"") && out.contains("\"model\":\"paper\""), "{}", out);
        assert!(out.contains("\"indicators\":{\"visited_all\":true,\"stopped\":true,\"stopped_at_start\":true}"), "{}", out);
        assert!(out.contains("\"stopped_at\":0"), "{}", out);
        // visits everything but stops on the wrong vertex
        let off = handle("cmd exact\nk 2\nmodel paper\ngraph 2 0 0-1\ntable\n0.10 1>0\n0.01 0>stop\nend\n");
        assert!(off.contains("\"reason\":\"stopped_off_start\""), "{}", off);
        assert!(off.contains("\"indicators\":{\"visited_all\":true,\"stopped\":true,\"stopped_at_start\":false}"), "{}", off);
        // stops before visiting everything
        let early = handle("cmd exact\nk 2\nmodel paper\ngraph 2 0 0-1\ntable\n0.10 1>stop\nend\n");
        assert!(early.contains("\"reason\":\"stopped_early\"") && early.contains("\"visited_all\":false"), "{}", early);
        // never stops
        let never = handle("cmd exact\nk 5\nmodel paper\ngraph 4 0 0-1,1-2,2-3\ndefault sigma\n");
        assert!(never.contains("\"reason\":\"never_stops\"") && never.contains("\"stopped\":false"), "{}", never);
        // a walk reports the same way
        let walk = handle("cmd walks\nk 2\nmodel paper\ngraph 2 0 0-1\ntable\n0.10 1>0\n0.01 0>stop\nend\n");
        assert!(walk.contains("\"reason\":\"stopped_off_start\""), "{}", walk);
        // the classic answer keeps its shape: no indicators, no reason
        let classic = handle("cmd exact\nk 5\ngraph 4 0 0-1,1-2,2-3\ndefault sigma\n");
        assert!(classic.contains("\"model\":\"classic\"") && !classic.contains("indicators") && !classic.contains("reason"), "{}", classic);
    }

    #[test]
    fn agen6_rows_match_the_page_template() {
        // testdata/agen6_rows_deg4.txt is produced by the page's JavaScript template
        // (`node testdata/check_agen6_template.mjs --write`); every row up to degree 4, six colours.
        fn bags(c: usize, left: u8, cnt: &mut [u8; MAXK], out: &mut Vec<[u8; MAXK]>) {
            if c == MAXK {
                if cnt.iter().sum::<u8>() >= 1 {
                    out.push(*cnt);
                }
                return;
            }
            for x in 0..=left {
                cnt[c] = x;
                bags(c + 1, left - x, cnt, out);
            }
            cnt[c] = 0;
        }
        let mut all = Vec::new();
        bags(0, 4, &mut [0; MAXK], &mut all);
        let mut lines = Vec::new();
        for own in 0..6u8 {
            for cnt in &all {
                let row = Row { own, cnt: *cnt };
                lines.push(match Formula::AGen6.action(&row, 6) {
                    Some(a) => format!("{} {}", row.text(6), a.text()),
                    None => format!("{} undefined", row.text(6)),
                });
            }
        }
        lines.sort();
        let fixture: Vec<&str> = include_str!("../testdata/agen6_rows_deg4.txt").lines().collect();
        assert_eq!(lines.len(), fixture.len());
        for (mine, theirs) in lines.iter().zip(&fixture) {
            assert_eq!(mine, theirs);
        }
    }

    #[test]
    fn a_walk_judges_the_position_it_ends_on_even_when_the_budget_runs_out() {
        // the wrong stop (on vertex 1) is the position reached after exactly 2 actions
        let g = Graph::parse("2 0 0-1").unwrap();
        let wrong = paper_rule_from_table(2, &[("0.10", "1>0"), ("0.01", "0>stop")]);
        for budget in [2, 3, 10] {
            match walk(&g, &wrong, 0, 0, budget) {
                Ok(Some(trace)) => assert_eq!(trace.stopped_at, Some(1), "budget {}", budget),
                _ => panic!("budget {}: the wrong stop must be reported", budget),
            }
        }
        // a correct stop on the last budgeted step is still an exploration
        let good = paper_rule_from_table(2, &[("0.10", "1>0"), ("0.01", "0>1"), ("1.10", "1>stop")]);
        assert!(matches!(walk(&g, &good, 0, 0, 3), Ok(None)));
        // running out of budget before anything is decided is not a verdict
        assert!(matches!(walk(&g, &good, 0, 0, 1), Ok(None)));
    }

    #[test]
    fn a_table_that_can_stop_is_judged_by_the_paper_model_without_being_told() {
        let table = "table\n0.10 1>0\n0.01 0>stop\nend\n";
        // no model key: the stop on vertex 1 is a failure, not a classic 'explores'
        let auto = handle(&format!("cmd exact\nk 2\ngraph 2 0 0-1\n{}", table));
        assert!(auto.contains("\"model\":\"paper\"") && auto.contains("\"reason\":\"stopped_off_start\""), "{}", auto);
        // asking for classic is still honoured: there the stop is only a stay and coverage wins
        let classic = handle(&format!("cmd exact\nk 2\nmodel classic\ngraph 2 0 0-1\n{}", table));
        assert!(classic.contains("\"model\":\"classic\"") && classic.contains("\"status\":\"explores\""), "{}", classic);
        // a table without stop stays classic
        let plain = handle("cmd exact\nk 2\ngraph 2 0 0-1\ntable\n0.10 1>0\n0.01 0>stay\nend\n");
        assert!(plain.contains("\"model\":\"classic\""), "{}", plain);
    }

    #[test]
    fn agen6_with_too_few_colours_is_an_error_not_a_silent_undefined() {
        for k in 2..6 {
            let out = handle(&format!("cmd exact\nk {}\ngraph 3 0 0-1,1-2\ndefault agen6\n", k));
            assert!(out.contains("\"status\":\"error\"") && out.contains("6 colours"), "k {}: {}", k, out);
        }
        let ok = handle("cmd exact\nk 6\ngraph 3 0 0-1,1-2\ndefault agen6\n");
        assert!(ok.contains("\"status\":\"explores\""), "{}", ok);
    }

    #[test]
    fn play_reproduces_the_hand_traces_under_a_stated_adversary() {
        // the traces of workbench/notes/agen6-hand-trace.xlsx, through the request protocol
        let steps = |out: &str| -> Vec<(String, String)> {
            // rows and actions in order, from the JSON of the trace
            let mut v = Vec::new();
            for chunk in out.split("{\"cur\":").skip(1) {
                let row = chunk.split("\"row\":\"").nth(1).unwrap().split('"').next().unwrap().to_string();
                let paint = chunk.split("\"paint\":").nth(1).unwrap().split(',').next().unwrap().to_string();
                let target: i32 = chunk.split("\"target\":").nth(1).unwrap().split(',').next().unwrap().parse().unwrap();
                let t = match target { -1 => "stay".to_string(), -2 => "stop".to_string(), c => c.to_string() };
                v.push((row, format!("{}>{}", paint, t)));
            }
            v
        };
        let check = |graph: &str, prefer: &str, expected: &[(&str, &str)]| {
            let out = handle(&format!("cmd play\nk 6\ngraph {}\ndefault agen6\nprefer {}\n", graph, prefer));
            assert!(out.contains("\"status\":\"explores\"") && out.contains("\"model\":\"paper\""), "{}", out);
            let got = steps(&out);
            let want: Vec<(String, String)> = expected.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
            assert_eq!(got, want, "{}", graph);
        };
        check("3 0 0-1,1-2", "0 1 2", &PATH_TRACE);
        check("3 0 0-1,1-2,0-2", "0 1 2", &TRIANGLE_TRACE);
        check("4 0 0-1,0-2,1-3", "1 2 3 0", &TREE_TRACE);
    }

    #[test]
    fn play_reports_failures_undefined_rows_and_unfinished_plays() {
        let wrong = "table\n0.10 1>0\n0.01 0>stop\nend\n";
        // stops on vertex 1
        let out = handle(&format!("cmd play\nk 2\ngraph 2 0 0-1\n{}", wrong));
        assert!(out.contains("\"status\":\"fails\"") && out.contains("\"reason\":\"stopped_off_start\"") && out.contains("\"stopped_at\":1"), "{}", out);
        // no rule for the first row
        let undef = handle("cmd play\nk 2\ngraph 2 0 0-1\ndefault none\n");
        assert!(undef.contains("\"status\":\"undefined\""), "{}", undef);
        // sigma* never stops: with a small budget the play is unfinished, with a large one it is caught repeating
        let g = "graph 4 0 0-1,1-2,2-3";
        let unf = handle(&format!("cmd play\nk 5\nmodel paper\n{}\ndefault sigma\nbudget 10\n", g));
        assert!(unf.contains("\"status\":\"unfinished\""), "{}", unf);
        let rep = handle(&format!("cmd play\nk 5\nmodel paper\n{}\ndefault sigma\nbudget 1000\n", g));
        assert!(rep.contains("\"status\":\"fails\"") && rep.contains("\"reason\":\"never_stops\"") && !rep.contains("\"repeat_at\":null"), "{}", rep);
        // a vertex outside the graph in `prefer` is an error
        assert!(handle("cmd play\nk 6\ngraph 3 0 0-1,1-2\ndefault agen6\nprefer 0 7\n").contains("not in the graph"));
        // and so is a token that is not a vertex number at all: it must not be dropped silently
        for bad in ["1 two 0", "-1", "300", "1,2", "0x1"] {
            let out = handle(&format!("cmd play\nk 6\ngraph 3 0 0-1,1-2\ndefault agen6\nprefer {}\n", bad));
            assert!(out.contains("\"status\":\"error\"") && out.contains("is not a vertex number"), "prefer {}: {}", bad, out);
        }
        // a later prefer line replaces the earlier one
        let two = handle("cmd play\nk 6\ngraph 4 0 0-1,0-2,1-3\ndefault agen6\nprefer 2 1 3 0\nprefer 1 2 3 0\n");
        let one = handle("cmd play\nk 6\ngraph 4 0 0-1,0-2,1-3\ndefault agen6\nprefer 1 2 3 0\n");
        assert_eq!(two, one);
        // a preference for the other neighbour changes the order of a tie, not the outcome
        let a = handle("cmd play\nk 6\ngraph 4 0 0-1,0-2,1-3\ndefault agen6\nprefer 1 2 3 0\n");
        let b = handle("cmd play\nk 6\ngraph 4 0 0-1,0-2,1-3\ndefault agen6\nprefer 2 1 3 0\n");
        assert!(a.contains("\"status\":\"explores\"") && b.contains("\"status\":\"explores\"") && a != b);
    }

    #[test]
    fn stop_makes_a_terminal_position_only_in_the_paper_model() {
        let g = Graph::parse("2 0 0-1").unwrap();
        let p = Pos { chi: 0, cur: 0, vis: 1, term: false };
        let stop = Action { paint: 2, target: STOP };
        let paper = successors(&g, &p, stop, true);
        assert_eq!(paper.len(), 1);
        let (moved, q) = paper[0];
        assert_eq!(moved, None);
        assert!(q.term && q.cur == 0 && q.vis == 1);
        assert_eq!(colour(q.chi, 0), 2, "the agent paints before it stops");
        // classic: stop still means no move, and the agent goes on
        assert!(!successors(&g, &p, stop, false)[0].1.term);
        // a colour target with no neighbour of that colour stays put in both models
        let absent = Action { paint: 1, target: 3 };
        assert!(!successors(&g, &p, absent, true)[0].1.term);
    }

    #[test]
    fn model_defaults_to_classic_and_agen6_implies_paper() {
        assert_eq!(resolve_model("", &Formula::parse("sigma"), false), Ok(false));
        assert_eq!(resolve_model("", &None, false), Ok(false));
        assert_eq!(resolve_model("", &Formula::parse("agen6"), false), Ok(true));
        assert_eq!(resolve_model("paper", &None, false), Ok(true));
        assert_eq!(resolve_model("classic", &Formula::parse("agen6"), false), Ok(false));
        assert!(resolve_model("nonsense", &None, false).is_err());
        // a rule that can stop is judged by the paper model unless `classic` is asked for
        assert_eq!(resolve_model("", &None, true), Ok(true));
        assert_eq!(resolve_model("classic", &None, true), Ok(false));
        assert!(handle("cmd answer
k 6
row 0.100000
model nonsense
").contains("unknown model"));
        assert!(!handle("cmd answer
k 6
row 0.100000
model paper
default agen6
").contains("error"));
    }

    #[test]
    fn protocol_round_trip() {
        let req = format!("cmd step\nk 5\ngraph {}\ncolours {}\ncur 0\nvis 0\ndefault sigma\ntable\n0.40000 2>0\nend\n", SKETCH1, vec!["0"; 17].join(" "));
        let out = handle(&req);
        assert!(out.contains("\"source\":\"table\"") && out.contains("\"paint\":2"), "{}", out);
        let out = handle(&format!("cmd exact\nk 5\ngraph {}\ncap 100000\n", "4 0 0-1,1-2,2-3"));
        assert!(out.contains("\"status\":\"undefined\""), "{}", out);
        let out = handle(&format!("cmd walks\nk 3\ngraph {}\ndefault chase3\n", SKETCH1));
        assert!(out.contains("\"status\":\"fails\""), "{}", out);
        assert!(handle("cmd exact\nk 9\n").contains("error"));
    }
}
