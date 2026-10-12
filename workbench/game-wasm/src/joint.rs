//! `a2c4`: a four-colour strategy for two agents that act together (a colleague's rule sheet, eleven numbered rules).
//!
//! The model differs from `a2c5` (see `two.rs`) in one thing: the scheduler.  Here the two agents see each other, a
//! rule reads BOTH of their views at once, and one rule fires for the pair: both agents act in the same step.  The
//! adversary only chooses where a move goes ("move to a neighbour of colour c").  The agents are identical; they are
//! labelled A (agent 0) and B (agent 1) for the drawing, and when the pair stands on one vertex the first role of a
//! rule is played by A.  Colours: 0 init (I), 1 head or candidate (H), 2 path (P), 3 fin (F).
//!
//! A rule is `together on a vertex of colour c` (one condition on the neighbours of that vertex) or `adjacent, on
//! vertices of colours c and d` (one condition on the neighbours of each vertex, the partner's own vertex left out).
//! A condition is about the colours present: `E(c)` some neighbour has colour c, `NoneOf(..)` none has any of them.
//! The first rule that matches fires: rule 11 first (it has priority), then 1 to 10, and inside a numbered rule the
//! cases a, b, c in the order written.  An action of a role is: paint the vertex and stay (`Ps`), move to a
//! neighbour of a colour without painting (`Mv`), paint and move onto the partner (`Pmp`), move onto the partner
//! without painting (`Mp`), or paint F and terminate (`Term`: both agents stop).
//!
//! A move to a colour nobody has cannot be carried out; the pair is then stuck (the rules never ask for it: see the
//! tests).  Checked against every connected graph of up to eight vertices (see the tests and the README).

use super::two::{observe, over, Pos2, Step2, PARTNER};
use super::*;

pub const NAME: &str = "a2c4";
pub const NEED: u8 = 4;

const I: u8 = 0;
const H: u8 = 1;
const P: u8 = 2;
const F: u8 = 3;
const ANYC: u8 = 255;

/// The tag of rule id `n` (the sheet's number and case): the ids run 1..=19 in the order the sheet is written.
pub const LABELS: [&str; 20] = ["", "R1", "R2", "R3a", "R3b", "R3c", "R4a", "R4b", "R4c", "R5a", "R5b", "R6a", "R6b", "R6c", "R7", "R8", "R9a", "R9b", "R10", "R11"];

const fn bit(c: u8) -> u8 {
    1 << c
}

#[derive(Clone, Copy)]
enum Cond {
    Any,
    /// some neighbour has colour c
    E(u8),
    /// no neighbour has any colour of the mask
    NoneOf(u8),
    /// no H neighbour and some I neighbour
    NoHSomeI,
}
impl Cond {
    fn ok(self, m: u8) -> bool {
        match self {
            Cond::Any => true,
            Cond::E(c) => m >> c & 1 == 1,
            Cond::NoneOf(mask) => m & mask == 0,
            Cond::NoHSomeI => m >> H & 1 == 0 && m >> I & 1 == 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Act {
    Ps(u8),
    Mv(u8),
    Pmp(u8),
    Mp,
    Term,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Tog,
    Apart,
}

struct Rule {
    id: u8,
    kind: Kind,
    c: u8,
    d: u8,
    c1: Cond,
    c2: Cond,
    acts: (Act, Act),
}

const fn tog(id: u8, c: u8, c1: Cond, a1: Act, a2: Act) -> Rule {
    Rule { id, kind: Kind::Tog, c, d: ANYC, c1, c2: Cond::Any, acts: (a1, a2) }
}
const fn apart(id: u8, c: u8, d: u8, c1: Cond, c2: Cond, a1: Act, a2: Act) -> Rule {
    Rule { id, kind: Kind::Apart, c, d, c1, c2, acts: (a1, a2) }
}

/// In the order they are tried: rule 11 first, then the sheet from the top.
static RULES: [Rule; 19] = [
    tog(19, ANYC, Cond::NoneOf(bit(I) | bit(H) | bit(P)), Act::Term, Act::Term), // 11: only fin around, both paint it fin and terminate
    tog(1, I, Cond::Any, Act::Ps(H), Act::Mv(I)),                                   // 1: one paints H and stays, the other enters an init neighbour
    apart(2, H, I, Cond::Any, Cond::Any, Act::Ps(H), Act::Pmp(H)),                  // 2: the init vertex becomes H, its agent goes back
    tog(3, H, Cond::E(H), Act::Ps(P), Act::Mv(H)),                                  // 3a
    tog(4, H, Cond::NoHSomeI, Act::Ps(H), Act::Mv(I)),                              // 3b
    tog(5, H, Cond::NoneOf(bit(H) | bit(I)), Act::Ps(P), Act::Ps(P)),              // 3c
    apart(6, P, H, Cond::Any, Cond::NoneOf(bit(I)), Act::Ps(P), Act::Ps(F)),        // 4a
    apart(7, P, H, Cond::Any, Cond::E(P), Act::Ps(P), Act::Mp),                     // 4b
    apart(8, P, H, Cond::Any, Cond::NoneOf(bit(P)), Act::Mp, Act::Ps(H)),           // 4c
    tog(9, P, Cond::E(I), Act::Ps(P), Act::Mv(I)),                                  // 5a
    tog(10, P, Cond::NoneOf(bit(I)), Act::Ps(F), Act::Mv(P)),                       // 5b
    apart(11, P, I, Cond::Any, Cond::NoneOf(bit(I)), Act::Ps(P), Act::Ps(F)),       // 6a
    apart(12, P, I, Cond::Any, Cond::E(P), Act::Ps(P), Act::Pmp(H)),                // 6b
    apart(13, P, I, Cond::Any, Cond::NoneOf(bit(P)), Act::Ps(P), Act::Ps(H)),       // 6c
    apart(14, P, F, Cond::Any, Cond::Any, Act::Ps(F), Act::Ps(I)),                  // 7
    apart(15, F, I, Cond::Any, Cond::Any, Act::Ps(F), Act::Pmp(F)),                 // 8
    tog(16, F, Cond::E(H), Act::Ps(F), Act::Mv(H)),                                 // 9a
    tog(17, F, Cond::NoneOf(bit(H)), Act::Ps(H), Act::Ps(H)),                       // 9b
    apart(18, F, H, Cond::Any, Cond::Any, Act::Ps(F), Act::Pmp(I)),                 // 10
];

/// The colours present among the neighbours of `v`, the vertex `excl` left out, as a bit mask over the colours.
fn nbm(g: &Graph, chi: u128, v: u8, excl: Option<u8>) -> u8 {
    let mut m = 0u8;
    for u in g.neighbours(v) {
        if Some(u) != excl {
            m |= bit(colour(chi, u));
        }
    }
    m
}

/// The first rule that matches the pair, with the vertices of its two roles (`u` plays the first, `w` the second).
fn find(g: &Graph, p: &Pos2) -> Option<(&'static Rule, u8, u8)> {
    let (a, b) = (p.a, p.b);
    for r in RULES.iter() {
        if r.kind == Kind::Tog {
            if a == b && (r.c == ANYC || colour(p.chi, a) == r.c) && r.c1.ok(nbm(g, p.chi, a, None)) {
                return Some((r, a, b));
            }
        } else if a != b && g.adj[a as usize] >> b & 1 == 1 {
            for (u, w) in [(a, b), (b, a)] {
                if colour(p.chi, u) == r.c && colour(p.chi, w) == r.d && r.c1.ok(nbm(g, p.chi, u, Some(w))) && r.c2.ok(nbm(g, p.chi, w, Some(u))) {
                    return Some((r, u, w));
                }
            }
        }
    }
    None
}

/// What one role does in one step: the agent that plays it, where it stands, the row it reads, the action (the
/// engine's `Action`; `paints` is false when the role does not repaint its vertex) and the vertices the adversary may
/// serve.
#[derive(Clone, Debug)]
pub struct Role {
    pub agent: u8,
    pub cur: u8,
    pub other: u8,
    pub row: String,
    pub act: Action,
    pub paints: bool,
    pub options: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Joint {
    pub id: u8,
    pub roles: [Role; 2],
    pub term: bool,
    /// a role must move to a colour that no neighbour has: the pair cannot act
    pub blocked: bool,
}

impl Joint {
    /// The options of the role that has a choice to make (a move to a colour), else empty.
    pub fn choose(&self) -> Vec<u8> {
        self.roles.iter().find(|r| r.act.target < NEED).map(|r| r.options.clone()).unwrap_or_default()
    }
}

fn role(g: &Graph, p: &Pos2, agent: u8, cur: u8, other: u8, act: Act) -> Role {
    let own = colour(p.chi, cur);
    let (action, paints, options) = match act {
        Act::Ps(c) => (Action { paint: c, target: STAY }, true, vec![]),
        Act::Term => (Action { paint: F, target: STOP }, true, vec![]),
        Act::Pmp(c) => (Action { paint: c, target: PARTNER }, true, vec![other]),
        Act::Mp => (Action { paint: own, target: PARTNER }, false, vec![other]),
        Act::Mv(c) => (Action { paint: own, target: c }, false, g.neighbours(cur).filter(|&u| colour(p.chi, u) == c).collect()),
    };
    Role { agent, cur, other, row: observe(g, p.chi, cur, other).text(NEED), act: action, paints, options }
}

/// The rule that fires for the pair in `p` (None: no rule matches), with both roles worked out.
pub fn joint_of(g: &Graph, p: &Pos2) -> Option<Joint> {
    if over(p) {
        return None;
    }
    let (r, u, w) = find(g, p)?;
    let (i0, i1) = if p.a == p.b || p.a == u { (0u8, 1u8) } else { (1u8, 0u8) };
    let roles = [role(g, p, i0, u, w, r.acts.0), role(g, p, i1, w, u, r.acts.1)];
    let blocked = roles.iter().any(|x| x.act.target < NEED && x.options.is_empty());
    Some(Joint { id: r.id, term: r.acts.0 == Act::Term || r.acts.1 == Act::Term, roles, blocked })
}

/// Every position the pair can reach by doing `j` in `p`, with the vertex each role went to (None: it stayed).
pub fn apply(p: &Pos2, j: &Joint) -> Vec<([Option<u8>; 2], Pos2)> {
    let mut q = *p;
    for r in &j.roles {
        if r.paints {
            q.chi = paint(q.chi, r.cur, r.act.paint);
        }
    }
    if j.term {
        q.ta = true;
        q.tb = true;
        return vec![([None, None], q)];
    }
    let dests = |r: &Role| -> Vec<Option<u8>> {
        match r.act.target {
            STAY | STOP => vec![None],
            _ => r.options.iter().map(|&u| Some(u)).collect(),
        }
    };
    let mut out = vec![];
    for d0 in dests(&j.roles[0]) {
        for d1 in dests(&j.roles[1]) {
            let mut r = q;
            for (role, d) in j.roles.iter().zip([d0, d1]) {
                if let Some(u) = d {
                    if role.agent == 0 {
                        r.a = u;
                    } else {
                        r.b = u;
                    }
                    r.vis |= 1 << u;
                }
            }
            out.push(([d0, d1], r));
        }
    }
    out
}

/// Every position the adversary can take the pair to from `p` (empty when no rule matches or a move cannot be done).
pub fn successors(g: &Graph, p: &Pos2) -> Vec<Pos2> {
    match joint_of(g, p) {
        Some(j) if !j.blocked => apply(p, &j).into_iter().map(|(_, q)| q).collect(),
        _ => vec![],
    }
}

fn half(r: &Role, rule: u8, next: Option<u8>) -> Step2 {
    Step2 { agent: r.agent, cur: r.cur, other: r.other, row: r.row.clone(), act: r.act, rule, options: r.options.clone(), next, paints: r.paints, second: None }
}

/// The step of the pair for `j` that went on to `dests`.
pub fn step_of(j: &Joint, dests: [Option<u8>; 2]) -> Step2 {
    let mut s = half(&j.roles[0], j.id, dests[0]);
    s.second = Some(Box::new(half(&j.roles[1], j.id, dests[1])));
    s
}

/// The step that takes `p` to its successor `q`.
pub fn step_between(g: &Graph, p: &Pos2, q: &Pos2) -> Step2 {
    let j = joint_of(g, p).expect("p has a step");
    for (d, r) in apply(p, &j) {
        if r == *q {
            return step_of(&j, d);
        }
    }
    unreachable!("q is not a successor of p")
}

/// The adversary's pick for a play that prefers the vertices of `pref` (a vertex not listed comes last, the smaller
/// number first): the successor reached when the role that chooses is served the preferred vertex.
pub fn play_step(g: &Graph, p: &Pos2, pref: &[u8]) -> Option<(Step2, Pos2)> {
    let j = joint_of(g, p)?;
    if j.blocked {
        return None;
    }
    let rank = |u: Option<u8>| u.map_or(usize::MAX, |u| pref.iter().position(|&x| x == u).unwrap_or(usize::MAX - 1));
    let chooser = j.roles.iter().position(|r| r.act.target < NEED);
    let (d, q) = apply(p, &j).into_iter().min_by_key(|(d, _)| chooser.map_or(0, |c| rank(d[c]))).unwrap();
    Some((step_of(&j, d), q))
}

#[cfg(test)]
#[path = "joint_tests.rs"]
mod tests;
