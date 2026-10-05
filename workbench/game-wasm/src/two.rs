//! Two agents in the colour-exploration game, and the five-colour algorithm `a2c5` for them.
//!
//! The model (see "Two agents" in workbench/README.md; these are assumptions, not a published model):
//!  * two identical oblivious agents start together on the start vertex; every vertex starts in colour 0;
//!  * an agent sees its own vertex colour, whether the other agent is on that vertex, and for each neighbour its
//!    colour and whether the other agent is there; it may ask to move to a neighbour of a colour, to the
//!    neighbour that holds the other agent (`PARTNER`), to stay, to stop, or to wait (do nothing);
//!  * the scheduler is sequential and fair: the adversary activates one agent per step (an agent whose rule says
//!    WAIT has nothing to do, so a fair scheduler cannot starve the other) and picks the destination among equal
//!    candidates;
//!  * the agents explore when every vertex is visited and both have stopped on the start vertex.
//!
//! Identical agents standing on the same vertex see the same row and act the same way, so when they are together
//! only agent 0 is stepped: the other choice is the mirror image.  That keeps the agents labelled (agent 0 is the
//! one that works, agent 1 waits and follows) without changing any verdict.
//!
//! `a2c5` is the semi-DFS simulation of A_US5/A_Gen6 with the five colours of A_US5 (0 init, 1 path, 2 neigh, 3 fin,
//! 4 head); agent 1 marks the head, which is what lets a rejected neighbour find its way back when two vertices
//! carry the head colour.  Checked against every connected graph of up to eight vertices (see the tests).

use super::*;

pub const NAME: &str = "a2c5";
pub const NEED: u8 = 5;

const INIT: u8 = 0;
const PATH: u8 = 1;
const NEIGH: u8 = 2;
const FIN: u8 = 3;
const HEAD: u8 = 4;

/// Targets that exist only with two agents (STAY = 255 and STOP = 254 are the engine's).
pub const WAIT: u8 = 253;
pub const PARTNER: u8 = 252;

/// What one agent sees.  `cnt[c][1]` counts the neighbours of colour `c` that hold the other agent (0 or 1).
#[derive(Clone, Copy, Debug)]
pub struct Obs2 {
    pub own: u8,
    pub co: bool,
    pub cnt: [[u8; 2]; MAXK],
}

impl Obs2 {
    fn has(&self, c: u8) -> bool {
        self.cnt[c as usize][0] + self.cnt[c as usize][1] > 0
    }
    fn occ(&self, c: u8) -> bool {
        self.cnt[c as usize][1] > 0
    }
    fn deg(&self) -> u8 {
        self.cnt.iter().map(|x| x[0] + x[1]).sum()
    }
    /// `own.bag@p`: the colour, one hex digit per colour counting ALL neighbours of that colour, then where the
    /// other agent is: `h` on this vertex, the colour digit of the neighbour holding it, or `-` not next to me.
    pub fn text(&self, k: u8) -> String {
        let mut s = format!("{}.", self.own);
        for c in 0..k as usize {
            s.push(std::char::from_digit((self.cnt[c][0] + self.cnt[c][1]) as u32, 16).unwrap_or('f'));
        }
        s.push('@');
        if self.co {
            s.push('h');
        } else if let Some(c) = (0..k as usize).find(|&c| self.cnt[c][1] > 0) {
            s.push(std::char::from_digit(c as u32, 10).unwrap());
        } else {
            s.push('-');
        }
        s
    }
    pub fn parse(s: &str) -> Option<Obs2> {
        let (head, partner) = s.split_once('@')?;
        let row = Row::parse(head)?;
        let mut cnt = [[0u8; 2]; MAXK];
        for c in 0..MAXK {
            cnt[c][0] = row.cnt[c];
        }
        let mut co = false;
        match partner.trim() {
            "h" => co = true,
            "-" => {}
            d => {
                let c: usize = d.parse().ok()?;
                if c >= MAXK || cnt[c][0] == 0 {
                    return None;
                }
                cnt[c][0] -= 1;
                cnt[c][1] = 1;
            }
        }
        Some(Obs2 { own: row.own, co, cnt })
    }
}

/// The twenty-one rules of `a2c5`: the action on an observation and the number of the rule that gave it; `None`
/// for an observation the algorithm never meets (an init vertex that is neither the start nor a probe).
/// Colours: 0 init, 1 path, 2 neigh, 3 fin, 4 head.
pub fn a2c5(o: &Obs2) -> Option<(Action, u8)> {
    let mv = |paint: u8, target: u8| Action { paint, target };
    let has = |c: u8| o.has(c);
    Some(match (o.own, o.co) {
        // both agents on my vertex
        (INIT, true) => {
            if o.deg() == 0 {
                (mv(FIN, STOP), 1) // a one-vertex graph
            } else if o.cnt[INIT as usize][0] == o.deg() {
                (mv(PATH, INIT), 2) // the first step: leave the start, the first neighbour is accepted blindly
            } else {
                return None;
            }
        }
        (HEAD, true) => {
            if has(HEAD) {
                if has(NEIGH) {
                    (mv(HEAD, NEIGH), 3) // a candidate waits: restore a rejected neighbour first
                } else {
                    (mv(PATH, HEAD), 4) // commit to the candidate
                }
            } else if has(PATH) {
                if has(INIT) {
                    (mv(HEAD, INIT), 5) // probe an init neighbour
                } else {
                    (mv(FIN, STAY), 6) // no candidate at all: finish this vertex, which flags the backtrack
                }
            } else if has(INIT) {
                (mv(PATH, INIT), 7) // the head is the start vertex: enter an init neighbour
            } else {
                (mv(FIN, STOP), 8) // nothing left at the start: stop
            }
        }
        (FIN, true) => {
            if has(NEIGH) {
                (mv(FIN, NEIGH), 9) // restore a rejected neighbour
            } else if has(PATH) {
                (mv(FIN, PATH), 10) // back up to the predecessor
            } else {
                (mv(FIN, STOP), 11) // the start vertex, finished: stop
            }
        }
        // I am alone on my vertex
        (INIT, false) => {
            if o.occ(HEAD) {
                if has(PATH) {
                    (mv(NEIGH, PARTNER), 12) // probed: it touches the path, a chord: mark it neigh, go back to my partner
                } else {
                    (mv(HEAD, PARTNER), 13) // probed: it can join the path: become the candidate, go back
                }
            } else if o.occ(PATH) && !has(HEAD) {
                (mv(HEAD, STAY), 14) // entered from the start vertex: become the head
            } else {
                return None;
            }
        }
        (NEIGH, false) => (mv(INIT, PARTNER), 15), // restore the mark and go back to my partner
        (HEAD, false) => (mv(HEAD, WAIT), 16),     // my partner is away or on its way to me
        (PATH, false) => {
            if has(HEAD) {
                (mv(PATH, HEAD), 17) // follow my partner to the new head
            } else if o.occ(FIN) {
                (mv(HEAD, STAY), 18) // the predecessor, reached while backtracking: become the head
            } else {
                (mv(PATH, WAIT), 19)
            }
        }
        (FIN, false) => {
            if has(HEAD) {
                (mv(FIN, HEAD), 20) // follow my partner to the head
            } else {
                (mv(FIN, WAIT), 21)
            }
        }
        _ => return None,
    })
}

// ------------------------------------------------------------------ positions
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Pos2 {
    pub chi: u128,
    pub a: u8,
    pub b: u8,
    pub ta: bool,
    pub tb: bool,
    pub vis: u32,
}

impl Pos2 {
    fn at(&self, i: u8) -> u8 {
        if i == 0 {
            self.a
        } else {
            self.b
        }
    }
    fn stopped(&self, i: u8) -> bool {
        if i == 0 {
            self.ta
        } else {
            self.tb
        }
    }
    fn together(&self) -> bool {
        self.a == self.b && self.ta == self.tb
    }
}

fn over(p: &Pos2) -> bool {
    p.ta && p.tb
}
fn won(g: &Graph, p: &Pos2) -> bool {
    over(p) && p.a == g.s && p.b == g.s && p.vis == g.full()
}

fn observe(g: &Graph, chi: u128, me: u8, other: u8) -> Obs2 {
    let mut cnt = [[0u8; 2]; MAXK];
    for u in g.neighbours(me) {
        cnt[colour(chi, u) as usize][(u == other) as usize] += 1;
    }
    Obs2 { own: colour(chi, me), co: me == other, cnt }
}

enum Act2 {
    Stopped,
    Waits,
    Acts(Obs2, Action, u8),
}

/// `Err` carries an observation the algorithm has no rule for.
fn act_of(g: &Graph, p: &Pos2, i: u8) -> Result<Act2, Obs2> {
    if p.stopped(i) {
        return Ok(Act2::Stopped);
    }
    let o = observe(g, p.chi, p.at(i), p.at(1 - i));
    match a2c5(&o) {
        None => Err(o),
        Some((a, rule)) => {
            if a.target == WAIT || (a.target == STAY && a.paint == o.own) {
                Ok(Act2::Waits)
            } else {
                Ok(Act2::Acts(o, a, rule))
            }
        }
    }
}

/// The positions agent `i` can reach by doing `a`, with the destination the adversary served (`None`: it stayed or
/// stopped).  As in the one-agent engine, a request for a colour nobody has leaves the agent where it is.
fn apply(g: &Graph, p: &Pos2, i: u8, a: Action) -> Vec<(Option<u8>, Pos2)> {
    let me = p.at(i);
    let mut q = *p;
    q.chi = paint(p.chi, me, a.paint);
    let moved = |u: u8| {
        let mut r = q;
        if i == 0 {
            r.a = u;
        } else {
            r.b = u;
        }
        r.vis |= 1 << u;
        (Some(u), r)
    };
    match a.target {
        STOP => {
            if i == 0 {
                q.ta = true;
            } else {
                q.tb = true;
            }
            vec![(None, q)]
        }
        STAY | WAIT => vec![(None, q)],
        PARTNER => {
            let other = p.at(1 - i);
            if g.adj[me as usize] >> other & 1 == 1 {
                vec![moved(other)]
            } else {
                vec![(None, q)]
            }
        }
        t => {
            let opts: Vec<u8> = g.neighbours(me).filter(|&u| colour(p.chi, u) == t).collect();
            if opts.is_empty() {
                vec![(None, q)]
            } else {
                opts.into_iter().map(moved).collect()
            }
        }
    }
}

/// Every step the adversary can take from `p`: which agent it activates (when the agents are together only agent 0:
/// the other choice is the mirror image) and where the agent goes.
fn successors2(g: &Graph, p: &Pos2) -> Result<Vec<(u8, Option<u8>, Pos2)>, Obs2> {
    let mut out = vec![];
    for i in 0..2u8 {
        if i == 1 && p.together() {
            continue;
        }
        if let Act2::Acts(_, a, _) = act_of(g, p, i)? {
            for (d, q) in apply(g, p, i, a) {
                out.push((i, d, q));
            }
        }
    }
    Ok(out)
}

/// The agent that acts in `p` for the player and `step`: agent 0 if it has something to do, else agent 1.
fn enabled(g: &Graph, p: &Pos2) -> Result<Option<(u8, Obs2, Action, u8)>, Obs2> {
    for i in 0..2u8 {
        if let Act2::Acts(o, a, rule) = act_of(g, p, i)? {
            return Ok(Some((i, o, a, rule)));
        }
    }
    Ok(None)
}

// ------------------------------------------------------------------ plays and traces
#[derive(Clone, Debug)]
pub struct Step2 {
    pub agent: u8,
    pub cur: u8,
    pub other: u8,
    pub row: String,
    pub act: Action,
    pub rule: u8,
    pub options: Vec<u8>,
    pub next: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct Trace2 {
    pub steps: Vec<Step2>,
    pub repeat_at: Option<usize>,
    pub end: Pos2,
    /// the last position has no agent that can act and the agents have not both stopped
    pub stuck: bool,
}

pub enum Outcome2 {
    Explores { positions: usize, trace: Trace2 },
    Fails { positions: usize, method: String, trace: Trace2 },
    Undefined { row: String, path: Trace2 },
    Overflow { positions: usize },
    Unfinished { trace: Trace2 },
}

fn step_from(g: &Graph, p: &Pos2, i: u8, o: &Obs2, a: Action, rule: u8, next: Option<u8>) -> Step2 {
    // the vertices the adversary may serve: the neighbours of the requested colour, or the one vertex of the partner
    // (a single option, so there is no choice to make)
    let options = match a.target {
        STOP | STAY | WAIT => vec![],
        PARTNER => {
            let other = p.at(1 - i);
            if g.adj[p.at(i) as usize] >> other & 1 == 1 {
                vec![other]
            } else {
                vec![]
            }
        }
        t => g.neighbours(p.at(i)).filter(|&u| colour(p.chi, u) == t).collect(),
    };
    Step2 { agent: i, cur: p.at(i), other: p.at(1 - i), row: o.text(NEED), act: a, rule, options, next }
}

/// The step that takes `p` to `q`, one of its successors.
fn step_between(g: &Graph, p: &Pos2, q: &Pos2) -> Step2 {
    for i in 0..2u8 {
        if i == 1 && p.together() {
            continue;
        }
        if let Ok(Act2::Acts(o, a, rule)) = act_of(g, p, i) {
            for (d, r) in apply(g, p, i, a) {
                if r == *q {
                    return step_from(g, p, i, &o, a, rule, d);
                }
            }
        }
    }
    unreachable!("q is not a successor of p")
}

fn trace_of(g: &Graph, chain: &[Pos2], repeat_at: Option<usize>) -> Trace2 {
    let steps: Vec<Step2> = (0..chain.len().saturating_sub(1)).map(|w| step_between(g, &chain[w], &chain[w + 1])).collect();
    let end = *chain.last().unwrap();
    let stuck = !over(&end) && repeat_at.is_none() && matches!(successors2(g, &end), Ok(v) if v.is_empty());
    Trace2 { steps, repeat_at, end, stuck }
}

fn root(g: &Graph) -> Pos2 {
    Pos2 { chi: 0, a: g.s, b: g.s, ta: false, tb: false, vis: 1 << g.s }
}

/// The exact game: every position the adversary can reach (both the choice of agent and of destination), the
/// positions from which every adversary loses, and either the longest obstruction (explores) or a trapping play.
pub fn exact_game2(g: &Graph, cap: usize) -> Outcome2 {
    let start = root(g);
    let mut states = vec![start];
    let mut index: Map<Pos2, u32> = Map::default();
    index.insert(start, 0);
    let mut parent: Vec<u32> = vec![0];
    let mut succ_start: Vec<u32> = vec![0];
    let mut succ: Vec<u32> = Vec::new();
    let mut i = 0usize;
    while i < states.len() {
        let p = states[i];
        if !over(&p) {
            match successors2(g, &p) {
                Err(o) => {
                    let mut chain = vec![i as u32];
                    let mut j = i as u32;
                    while j != 0 {
                        j = parent[j as usize];
                        chain.push(j);
                    }
                    chain.reverse();
                    let ps: Vec<Pos2> = chain.iter().map(|&c| states[c as usize]).collect();
                    return Outcome2::Undefined { row: o.text(NEED), path: trace_of(g, &ps, None) };
                }
                Ok(list) => {
                    for (_, _, q) in list {
                        let j = match index.get(&q) {
                            Some(&j) => j,
                            None => {
                                if states.len() >= cap {
                                    return Outcome2::Overflow { positions: states.len() };
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
        if won(g, &states[p]) {
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
    let succs = |at: u32| -> Vec<u32> { (succ_start[at as usize]..succ_start[at as usize + 1]).map(|e| succ[e as usize]).collect() };
    if win[0] {
        let mut chain = vec![states[0]];
        let mut at = 0u32;
        while !over(&states[at as usize]) {
            let next = succs(at).into_iter().max_by_key(|&j| rank[j as usize]).unwrap();
            chain.push(states[next as usize]);
            at = next;
        }
        return Outcome2::Explores { positions: n, trace: trace_of(g, &chain, None) };
    }
    // a trapping play: stay among losing positions, prefer a step that visits something new, until a position repeats
    let mut chain = vec![states[0]];
    let mut seen: Map<u32, usize> = Map::default();
    let mut at = 0u32;
    let repeat_at = loop {
        if let Some(&s) = seen.get(&at) {
            break Some(s);
        }
        seen.insert(at, chain.len() - 1);
        let p = states[at as usize];
        if over(&p) {
            break None; // both stopped, in the wrong place
        }
        let mut choice: Option<u32> = None;
        for j in succs(at) {
            if win[j as usize] {
                continue;
            }
            let better = match choice {
                None => true,
                Some(c) => states[j as usize].vis != p.vis && states[c as usize].vis == p.vis,
            };
            if better {
                choice = Some(j);
            }
        }
        match choice {
            None => break None, // stuck: no agent can act
            Some(j) => {
                chain.push(states[j as usize]);
                at = j;
            }
        }
    };
    Outcome2::Fails { positions: n, method: "exact game".into(), trace: trace_of(g, &chain, repeat_at) }
}

/// One play under an adversary that prefers the vertices in `pref` (a vertex not listed comes last, the smaller
/// number first).  Agent 0 acts when the agents are together.
pub fn play2(g: &Graph, pref: &[u8], budget: usize) -> Outcome2 {
    let rank = |u: Option<u8>| u.map_or(usize::MAX, |u| pref.iter().position(|&x| x == u).unwrap_or(usize::MAX - 1));
    let mut p = root(g);
    let mut chain = vec![p];
    let mut seen: Map<Pos2, usize> = Map::default();
    loop {
        if over(&p) {
            let t = trace_of(g, &chain, None);
            return if won(g, &p) { Outcome2::Explores { positions: 0, trace: t } } else { Outcome2::Fails { positions: 0, method: play_method(pref), trace: t } };
        }
        if chain.len() > budget {
            return Outcome2::Unfinished { trace: trace_of(g, &chain, None) };
        }
        if let Some(&s) = seen.get(&p) {
            return Outcome2::Fails { positions: 0, method: play_method(pref), trace: trace_of(g, &chain, Some(s)) };
        }
        seen.insert(p, chain.len() - 1);
        match enabled(g, &p) {
            Err(o) => return Outcome2::Undefined { row: o.text(NEED), path: trace_of(g, &chain, None) },
            Ok(None) => return Outcome2::Fails { positions: 0, method: play_method(pref), trace: trace_of(g, &chain, None) },
            Ok(Some((i, _, a, _))) => {
                let outs = apply(g, &p, i, a);
                let (_, q) = outs.into_iter().min_by_key(|(d, _)| rank(*d)).unwrap();
                p = q;
                chain.push(p);
            }
        }
    }
}

fn play_method(pref: &[u8]) -> String {
    format!("play: adversary prefers {}", if pref.is_empty() { "the smallest vertex".to_string() } else { pref.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" ") })
}

// ------------------------------------------------------------------ JSON
fn target_json2(t: u8) -> String {
    match t {
        STAY => "-1".into(),
        STOP => "-2".into(),
        WAIT => "-3".into(),
        PARTNER => "-4".into(),
        c => c.to_string(),
    }
}

fn trace_json2(g: &Graph, t: &Trace2) -> String {
    let steps: Vec<String> = t
        .steps
        .iter()
        .map(|s| {
            format!(
                "{{\"agent\":{},\"cur\":{},\"other\":{},\"row\":{},\"paint\":{},\"target\":{},\"options\":{},\"next\":{},\"rule\":{}}}",
                s.agent,
                s.cur,
                s.other,
                js(&s.row),
                s.act.paint,
                target_json2(s.act.target),
                list(&s.options),
                s.next.map_or("null".to_string(), |u| u.to_string()),
                s.rule
            )
        })
        .collect();
    let stopped_at = if over(&t.end) && t.end.a == t.end.b { t.end.a.to_string() } else { "null".to_string() };
    format!(
        "{{\"repeat_at\":{},\"unvisited\":{},\"stopped_at\":{},\"stuck\":{},\"agents\":[{},{}],\"steps\":[{}]}}",
        t.repeat_at.map_or("null".to_string(), |r| r.to_string()),
        mask_list(g.full() & !t.end.vis),
        stopped_at,
        t.stuck,
        t.end.a,
        t.end.b,
        steps.join(",")
    )
}

/// Why a play ended badly: `never_stops`, `stopped_early`, `stopped_off_start`, or `stuck` (neither agent can act).
fn fail_reason2(g: &Graph, t: &Trace2) -> &'static str {
    let e = &t.end;
    if over(e) {
        if e.vis != g.full() {
            "stopped_early"
        } else {
            "stopped_off_start"
        }
    } else if t.stuck {
        "stuck"
    } else {
        "never_stops"
    }
}

fn indicators2(g: &Graph, t: &Trace2) -> String {
    let e = &t.end;
    format!("{{\"visited_all\":{},\"stopped\":{},\"stopped_at_start\":{}}}", e.vis == g.full(), over(e), over(e) && e.a == g.s && e.b == g.s)
}

fn outcome_json2(g: &Graph, o: &Outcome2) -> String {
    let head = "\"model\":\"paper\",\"agents\":2";
    match o {
        Outcome2::Explores { positions, trace } => format!("{{\"status\":\"explores\",{},\"positions\":{},\"indicators\":{},\"trace\":{}}}", head, positions, indicators2(g, trace), trace_json2(g, trace)),
        Outcome2::Fails { positions, method, trace } => format!("{{\"status\":\"fails\",{},\"positions\":{},\"method\":{},\"indicators\":{},\"reason\":\"{}\",\"trace\":{}}}", head, positions, js(method), indicators2(g, trace), fail_reason2(g, trace), trace_json2(g, trace)),
        Outcome2::Undefined { row, path } => format!("{{\"status\":\"undefined\",{},\"row\":{},\"path\":{}}}", head, js(row), trace_json2(g, path)),
        Outcome2::Overflow { positions } => format!("{{\"status\":\"overflow\",{},\"positions\":{}}}", head, positions),
        Outcome2::Unfinished { trace } => format!("{{\"status\":\"unfinished\",{},\"trace\":{}}}", head, trace_json2(g, trace)),
    }
}

// ------------------------------------------------------------------ the request
/// A request with `agents 2`.  Keys as for one agent, plus `cur2` (the second agent's vertex) and `term` (two flags,
/// 1 = that agent has stopped) for `step`; the rule is the built-in `default a2c5`; `walks` is not offered.
pub fn handle(r: &Request) -> String {
    if r.k < NEED {
        return err(&format!("{} needs {} colours (k {})", NAME, NEED, NEED));
    }
    if r.default_raw != NAME {
        return err(&format!("two agents have one rule, the built-in {} (default {})", NAME, NAME));
    }
    if !r.table.is_empty() || !r.bad_rows.is_empty() {
        return err("two agents: rule tables are not supported, the built-in rule is a2c5");
    }
    if r.model == "classic" {
        return err("two agents are judged by the paper model (every vertex visited, both stopped on the start)");
    }
    if r.cmd == "answer" {
        let Some(o) = r.row.as_deref().and_then(Obs2::parse) else { return err("row: expected own.bag@p, e.g. 4.20100@h") };
        return match a2c5(&o) {
            Some((a, rule)) => format!("{{\"status\":\"ok\",\"defined\":true,\"source\":\"default\",\"paint\":{},\"target\":{},\"rule\":{}}}", a.paint, target_json2(a.target), rule),
            None => "{\"status\":\"ok\",\"defined\":false}".to_string(),
        };
    }
    let g = match &r.graph {
        Some(Ok(g)) => g,
        Some(Err(e)) => return err(e),
        None => return err("graph: missing"),
    };
    match r.cmd.as_str() {
        "step" => {
            let (Some(cur), Some(cur2)) = (r.cur, r.cur2) else { return err("cur and cur2: missing") };
            if cur >= g.n || cur2 >= g.n || r.colours.len() != g.n as usize || r.colours.iter().any(|&c| c >= NEED) || r.term.len() != 2 {
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
            vis |= 1 << cur | 1 << cur2;
            let p = Pos2 { chi, a: cur, b: cur2, ta: r.term[0] != 0, tb: r.term[1] != 0, vis };
            if over(&p) {
                return "{\"status\":\"ok\",\"over\":true}".to_string();
            }
            match enabled(g, &p) {
                Err(o) => format!("{{\"status\":\"ok\",\"row\":{},\"defined\":false}}", js(&o.text(NEED))),
                Ok(None) => "{\"status\":\"ok\",\"stuck\":true}".to_string(),
                Ok(Some((i, o, a, rule))) => {
                    let st = step_from(g, &p, i, &o, a, rule, None);
                    let auto = auto_choice(g, vis, &st.options);
                    format!(
                        "{{\"status\":\"ok\",\"agent\":{},\"cur\":{},\"other\":{},\"row\":{},\"degree\":{},\"defined\":true,\"source\":\"default\",\"paint\":{},\"target\":{},\"options\":{},\"auto\":{},\"explored\":{},\"rule\":{}}}",
                        i,
                        st.cur,
                        st.other,
                        js(&st.row),
                        o.deg(),
                        a.paint,
                        target_json2(a.target),
                        list(&st.options),
                        auto.map_or("null".to_string(), |u| u.to_string()),
                        vis == g.full(),
                        rule
                    )
                }
            }
        }
        "exact" => outcome_json2(g, &exact_game2(g, r.cap.clamp(1_000, 12_000_000))),
        "play" => {
            if let Some(&bad) = r.prefer.iter().find(|&&v| v >= g.n) {
                return err(&format!("prefer: vertex {} is not in the graph (0..{})", bad, g.n - 1));
            }
            outcome_json2(g, &play2(g, &r.prefer, r.budget.clamp(10, 5_000_000)))
        }
        "walks" => err("walks are not offered for two agents: use exact or play"),
        other => err(&format!("unknown cmd {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(n: usize, start: usize, edges: &[(u8, u8)]) -> Graph {
        let list: Vec<String> = edges.iter().map(|(a, b)| format!("{}-{}", a, b)).collect();
        Graph::parse(&format!("{} {} {}", n, start, list.join(","))).unwrap()
    }

    fn explores(g: &Graph) -> bool {
        matches!(exact_game2(g, 20_000_000), Outcome2::Explores { .. })
    }

    /// (observation, action, rule number): each row derived by hand from the rule's wording.
    #[test]
    fn rules_on_hand_derived_rows() {
        let rows: &[(&str, Option<(&str, u8)>)] = &[
            ("0.00000@h", Some(("3>stop", 1))),   // no neighbour: stop
            ("0.30000@h", Some(("1>0", 2))),      // the first step: three init neighbours
            ("0.20100@h", None),                  // together on an init vertex that is not the start: no rule
            ("4.11101@h", Some(("4>2", 3))),      // a candidate (head) and a mark (neigh) around: restore the mark
            ("4.11001@h", Some(("1>4", 4))),      // a candidate, no mark: commit
            ("4.10011@h", Some(("1>4", 4))),      // the same with a fin neighbour as well
            ("4.11010@h", Some(("4>0", 5))),      // path and init, no head: probe
            ("4.01010@h", Some(("3>stay", 6))),   // path, no init, no head: finish (the backtrack flag)
            ("4.20000@h", Some(("1>0", 7))),      // the start as the head: init neighbours only
            ("4.00020@h", Some(("3>stop", 8))),   // nothing but fin around: stop at the start
            ("3.00200@h", Some(("3>2", 9))),      // a finished head with marks left
            ("3.01010@h", Some(("3>1", 10))),     // marks restored: back up to the predecessor
            ("3.00030@h", Some(("3>stop", 11))),  // the finished start
            ("0.01001@4", Some(("2>-4", 12))),    // probe arrival: the head holds the partner, a path vertex too: a chord
            ("0.00001@4", Some(("4>-4", 13))),    // probe arrival with no other path vertex: a candidate
            ("0.01000@1", Some(("4>stay", 14))),  // entered from the start vertex (path, holding the partner)
            ("2.00001@4", Some(("0>-4", 15))),    // a mark: back to the partner
            ("4.01000@1", Some(("4>-3", 16))),    // the head waits while its partner is away
            ("1.10001@-", Some(("1>4", 17))),     // follow to the new head
            ("1.01110@3", Some(("4>stay", 18))),  // the predecessor, partner on a fin neighbour: become the head
            ("1.11000@0", Some(("1>-3", 19))),    // nothing to do: wait
            ("3.01001@-", Some(("3>4", 20))),     // finished: follow to the head
            ("3.01000@-", Some(("3>-3", 21))),    // finished: wait
        ];
        for (text, want) in rows {
            let o = Obs2::parse(text).unwrap_or_else(|| panic!("row {} does not parse", text));
            assert_eq!(o.text(5), *text, "text round trip");
            let got = a2c5(&o).map(|(a, n)| {
                let t = match a.target {
                    STAY => "stay".to_string(),
                    STOP => "stop".to_string(),
                    WAIT => "-3".to_string(),
                    PARTNER => "-4".to_string(),
                    c => c.to_string(),
                };
                (format!("{}>{}", a.paint, t), n)
            });
            assert_eq!(got, want.map(|(a, n)| (a.to_string(), n)), "rule on {}", text);
        }
    }

    #[test]
    fn small_graphs_explore() {
        assert!(explores(&graph(1, 0, &[])));
        assert!(explores(&graph(2, 0, &[(0, 1)])));
        assert!(explores(&graph(3, 1, &[(0, 1), (1, 2)])));
        assert!(explores(&graph(3, 0, &[(0, 1), (1, 2), (0, 2)])));
    }

    /// The diamond, K4 minus an edge, is the smallest graph outside A_US5's class: one agent running A_US5 fails on it
    /// from some starts; two agents running a2c5 explore it from every start.
    #[test]
    fn the_diamond_is_explored_by_two_agents_but_not_by_a_us5() {
        let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3)];
        for s in 0..4 {
            assert!(explores(&graph(4, s, &edges)), "two agents from {}", s);
        }
        let one = Rule { k: 5, table: Map::default(), default: Some(Formula::AUS5), terminating: true };
        let fails = (0..4).any(|s| matches!(exact_game(&graph(4, s, &edges), &one, 1_000_000), Outcome::Fails { .. }));
        assert!(fails, "A_US5 alone should fail on the diamond");
    }

    /// Every connected graph of up to six vertices, every start, against every adversary and both choices of agent,
    /// and the number of positions the exact game enumerates: the totals below come from an independent checker
    /// (a separate implementation of this model), so they hold the two to the same game.
    #[test]
    fn every_connected_graph_up_to_six_vertices_is_explored() {
        let expect = [(2usize, 20usize), (3, 180), (4, 1832), (5, 20226), (6, 300870)];
        for (n, want) in expect {
            let mut total = 0usize;
            for edges in super::paper_tests::connected_graphs(n) {
                for s in 0..n {
                    let g = graph(n, s, &edges);
                    match exact_game2(&g, 20_000_000) {
                        Outcome2::Explores { positions, .. } => total += positions,
                        _ => panic!("two agents fail on {} {:?} from {}", n, edges, s),
                    }
                }
            }
            assert_eq!(total, want, "positions enumerated on the connected graphs with {} vertices", n);
        }
    }

    /// Seven vertices: 853 graphs, 5,971 graph-start pairs, 6,157,922 positions.  Slow in a debug build: `cargo test --release -- --ignored`.
    #[test]
    #[ignore]
    fn every_connected_graph_on_seven_vertices_is_explored() {
        let mut total = 0usize;
        for edges in super::paper_tests::connected_graphs(7) {
            for s in 0..7 {
                match exact_game2(&graph(7, s, &edges), 20_000_000) {
                    Outcome2::Explores { positions, .. } => total += positions,
                    _ => panic!("two agents fail on 7 {:?} from {}", edges, s),
                }
            }
        }
        assert_eq!(total, 6_157_922);
    }

    /// The rules never ask for a colour nobody has or for a partner that is not next door, and never leave the
    /// start with an undefined row: scan every reachable position of a few graphs.
    #[test]
    fn no_request_is_ever_unanswerable() {
        for (n, edges) in [(4usize, vec![(0u8, 1u8), (0, 2), (0, 3), (1, 2), (1, 3)]), (5, vec![(0, 1), (1, 2), (2, 3), (3, 4), (0, 4), (1, 3)]), (5, vec![(0, 1), (0, 2), (0, 3), (0, 4), (1, 2), (3, 4)])] {
            for s in 0..n {
                let g = graph(n, s as usize, &edges);
                let mut seen: std::collections::HashSet<Pos2> = std::collections::HashSet::new();
                let mut todo = vec![root(&g)];
                while let Some(p) = todo.pop() {
                    if !seen.insert(p) || over(&p) {
                        continue;
                    }
                    for i in 0..2u8 {
                        if let Act2::Acts(_, a, _) = act_of(&g, &p, i).expect("a row with no rule") {
                            if a.target == PARTNER {
                                assert_eq!(g.adj[p.at(i) as usize] >> p.at(1 - i) & 1, 1, "partner not adjacent");
                            } else if a.target < NEED {
                                assert!(g.neighbours(p.at(i)).any(|u| colour(p.chi, u) == a.target), "a colour no neighbour has");
                            }
                        }
                    }
                    for (_, _, q) in successors2(&g, &p).unwrap() {
                        todo.push(q);
                    }
                }
            }
        }
    }

    /// The algorithm never fails, so the way a failure is reported is checked on constructed end positions.
    #[test]
    fn failures_are_reported_with_their_reason() {
        let g = graph(3, 0, &[(0, 1), (1, 2)]);
        let end = |a: u8, b: u8, ta: bool, tb: bool, vis: u32| Pos2 { chi: 0, a, b, ta, tb, vis };
        let trace = |end: Pos2, repeat_at: Option<usize>, stuck: bool| Trace2 { steps: vec![], repeat_at, end, stuck };
        // both stopped on the start with a vertex unvisited
        let early = trace(end(0, 0, true, true, 0b011), None, false);
        assert_eq!(fail_reason2(&g, &early), "stopped_early");
        assert_eq!(indicators2(&g, &early), "{\"visited_all\":false,\"stopped\":true,\"stopped_at_start\":true}");
        // both stopped everywhere visited, but not on the start
        let off = trace(end(2, 2, true, true, 0b111), None, false);
        assert_eq!(fail_reason2(&g, &off), "stopped_off_start");
        assert_eq!(indicators2(&g, &off), "{\"visited_all\":true,\"stopped\":true,\"stopped_at_start\":false}");
        // one agent stopped, the other cycles: never stops; neither can act: stuck
        assert_eq!(fail_reason2(&g, &trace(end(0, 1, true, false, 0b111), Some(3), false)), "never_stops");
        assert_eq!(fail_reason2(&g, &trace(end(0, 1, false, false, 0b111), None, true)), "stuck");
        // a one-agent-stopped end is not "stopped" for the indicators
        assert!(indicators2(&g, &trace(end(0, 1, true, false, 0b111), Some(3), false)).contains("\"stopped\":false"));
        let json = outcome_json2(&g, &Outcome2::Fails { positions: 5, method: "exact game".into(), trace: trace(end(0, 1, false, false, 0b111), None, true) });
        assert!(json.contains("\"reason\":\"stuck\"") && json.contains("\"stuck\":true") && json.contains("\"agents\":2"), "{}", json);
    }

    /// A hand trace: the path 0-1-2 from 0, derived by hand from the rules (agent 0 works, agent 1 follows).
    ///  1  A  rule 2   at 0 together: paint 0 path, enter 1
    ///  2  A  rule 14  at 1, entered from the start: become head
    ///  3  B  rule 17  at 0: follow to 1
    ///  4  A  rule 5   at 1 together: probe the init neighbour 2
    ///  5  A  rule 13  at 2, no other path vertex: become the candidate, back to the partner at 1
    ///  6  A  rule 4   at 1 together, candidate waiting: paint 1 path, move to 2
    ///  7  B  rule 17  at 1: follow to 2
    ///  8  A  rule 6   at 2 together, nothing left: become fin (backtrack flag)
    ///  9  A  rule 10  at 2: back up to the predecessor 1
    /// 10  A  rule 18  at 1, partner on a fin neighbour: become head
    /// 11  B  rule 20  at 2 (fin): follow to 1
    /// 12  A  rule 6   at 1 together, nothing left: become fin
    /// 13  A  rule 10  at 1: back up to 0
    /// 14  A  rule 18  at 0: become head
    /// 15  B  rule 20  at 1 (fin): follow to 0
    /// 16  A  rule 8   at 0 together, nothing left: become fin and stop
    /// 17  B  rule 11  at 0 (fin) with A: stop
    #[test]
    fn hand_trace_on_a_path() {
        let g = graph(3, 0, &[(0, 1), (1, 2)]);
        match play2(&g, &[], 1000) {
            Outcome2::Explores { trace, .. } => {
                let rules: Vec<u8> = trace.steps.iter().map(|s| s.rule).collect();
                let agents: Vec<u8> = trace.steps.iter().map(|s| s.agent).collect();
                assert_eq!(rules, vec![2, 14, 17, 5, 13, 4, 17, 6, 10, 18, 20, 6, 10, 18, 20, 8, 11]);
                assert_eq!(agents, vec![0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1]);
                assert!(trace.end.ta && trace.end.tb && trace.end.a == 0 && trace.end.b == 0);
                assert_eq!(trace.end.chi, (3u128) | (3 << 4) | (3 << 8), "all three vertices finished");
            }
            _ => panic!("the play must explore"),
        }
    }
}
