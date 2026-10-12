//! Tests of `a2c4` (see `joint.rs`): the sheet's rules on hand-derived positions, the hand trace of a path, and every
//! connected graph of up to six vertices (seven with `--ignored`) against every adversary.

use super::super::two::{exact_game_m, over, play_m, root, Model, Outcome2, Pos2};
use super::*;

fn graph(n: usize, start: usize, edges: &[(u8, u8)]) -> Graph {
    let list: Vec<String> = edges.iter().map(|(a, b)| format!("{}-{}", a, b)).collect();
    Graph::parse(&format!("{} {} {}", n, start, list.join(","))).unwrap()
}

fn pos(g: &Graph, colours: &[u8], a: u8, b: u8) -> Pos2 {
    let mut chi = 0u128;
    for (v, &c) in colours.iter().enumerate() {
        chi = paint(chi, v as u8, c);
    }
    Pos2 { chi, a, b, ta: false, tb: false, vis: g.full() }
}

fn explores(g: &Graph) -> bool {
    matches!(exact_game_m(g, 20_000_000, Model::Joint), Outcome2::Explores { .. })
}

/// `paint>target` of a role: `P>stay` paints P and stays, `->H` only moves to an H neighbour, `H>partner` paints H and
/// moves onto the partner.
fn word(r: &Role) -> String {
    let names = ["I", "H", "P", "F"];
    let p = if r.paints { names[r.act.paint as usize].to_string() } else { "-".to_string() };
    let t = match r.act.target {
        STAY => "stay".to_string(),
        STOP => "stop".to_string(),
        PARTNER => "partner".to_string(),
        c => names[c as usize].to_string(),
    };
    format!("{}>{}", p, t)
}

type Case<'a> = (&'a [(u8, u8)], Vec<u8>, u8, u8, &'a str, &'a str, &'a str, Vec<u8>);

/// One position for every rule case of the sheet, derived by hand from the sheet: the rule that fires, the two roles'
/// actions (first role, second role) and the vertices the adversary may serve.
#[test]
fn every_rule_case_on_a_hand_derived_position() {
    let (i, h, p, f) = (I, H, P, F);
    let cases: Vec<Case> = vec![
        (&[(0, 1)], vec![i, i], 0, 0, "R1", "H>stay", "->I", vec![1]),                          // the start
        (&[(0, 1)], vec![h, i], 0, 1, "R2", "H>stay", "H>partner", vec![]),                     // H and I side by side: the I vertex becomes H
        (&[(0, 1)], vec![h, h], 0, 0, "R3a", "P>stay", "->H", vec![1]),                         // an H neighbour: commit, go to it
        (&[(0, 1)], vec![h, i], 0, 0, "R3b", "H>stay", "->I", vec![1]),                         // no H neighbour, an I one: probe it
        (&[(0, 1)], vec![h, p], 0, 0, "R3c", "P>stay", "P>stay", vec![]),                       // neither: dead end, become path
        (&[(0, 1)], vec![p, h], 0, 1, "R4a", "P>stay", "F>stay", vec![]),                       // the H side has no I neighbour: finish it
        (&[(0, 1), (1, 2), (1, 3)], vec![p, h, p, i], 0, 1, "R4b", "P>stay", "->partner", vec![]), // the H side touches another P: reject
        (&[(0, 1), (1, 2)], vec![p, h, i], 0, 1, "R4c", "->partner", "H>stay", vec![]),         // an I neighbour, no other P: accept
        (&[(0, 1)], vec![p, i], 0, 0, "R5a", "P>stay", "->I", vec![1]),                         // together on P with an I neighbour: scan
        (&[(0, 1)], vec![p, p], 0, 0, "R5b", "F>stay", "->P", vec![1]),                         // no I neighbour: backtrack
        (&[(0, 1)], vec![p, i], 0, 1, "R6a", "P>stay", "F>stay", vec![]),                       // the I side has no I neighbour: finish it
        (&[(0, 1), (1, 2), (1, 3)], vec![p, i, i, p], 0, 1, "R6b", "P>stay", "H>partner", vec![]), // the I side touches another P: reject
        (&[(0, 1), (1, 2)], vec![p, i, i], 0, 1, "R6c", "P>stay", "H>stay", vec![]),            // it touches no other P: candidate
        (&[(0, 1)], vec![p, f], 0, 1, "R7", "F>stay", "I>stay", vec![]),                        // P and F side by side
        (&[(0, 1)], vec![f, i], 0, 1, "R8", "F>stay", "F>partner", vec![]),                     // F and I side by side
        (&[(0, 1)], vec![f, h], 0, 0, "R9a", "F>stay", "->H", vec![1]),                         // together on F with an H neighbour
        (&[(0, 1)], vec![f, p], 0, 0, "R9b", "H>stay", "H>stay", vec![]),                       // no H neighbour
        (&[(0, 1)], vec![f, h], 0, 1, "R10", "F>stay", "I>partner", vec![]),                    // F and H side by side
        (&[(0, 1)], vec![f, f], 0, 0, "R11", "F>stop", "F>stop", vec![]),                       // only fin around
    ];
    for (edges, colours, a, b, tag, first, second, choose) in cases {
        let g = graph(colours.len(), 0, edges);
        let j = joint_of(&g, &pos(&g, &colours, a, b)).unwrap_or_else(|| panic!("no rule for the {} position", tag));
        assert_eq!(LABELS[j.id as usize], tag, "the rule on the {} position", tag);
        assert_eq!(word(&j.roles[0]), first, "{}: first role", tag);
        assert_eq!(word(&j.roles[1]), second, "{}: second role", tag);
        assert_eq!(j.choose(), choose, "{}: what the adversary picks from", tag);
        assert!(!j.blocked, "{}", tag);
    }
    // apart, the role goes to the agent standing on the vertex of its colour: A on the I vertex plays the second role
    let g = graph(2, 0, &[(0, 1)]);
    let j = joint_of(&g, &pos(&g, &[H, I], 1, 0)).unwrap();
    assert_eq!((j.roles[0].agent, j.roles[0].cur, j.roles[1].agent, j.roles[1].cur), (1, 0, 0, 1));
}

/// Rule 11 has priority: on a single vertex both rule 1 and rule 11 match; 11 fires.
#[test]
fn rule_eleven_comes_first() {
    let g = graph(1, 0, &[]);
    let j = joint_of(&g, &pos(&g, &[I], 0, 0)).unwrap();
    assert_eq!(LABELS[j.id as usize], "R11");
    assert!(explores(&g));
    // the table is ordered with it first
    assert_eq!(RULES[0].id, 19);
}

/// The hand trace of the path 0-1-2 from 0 (no adversary choice anywhere), derived by hand from the sheet.
///  1 R1   at 0: one agent paints 0 H, the other enters 1
///  2 R2   H at 0, I at 1: 1 becomes H, its agent goes back to 0
///  3 R3a  together on 0 (H) with an H neighbour: 0 becomes P, one agent goes to the H neighbour 1
///  4 R4c  P at 0, H at 1 (no other P around 1): the P agent moves onto 1
///  5 R3b  together on 1: no H neighbour, an I one: one agent enters 2
///  6 R2   H at 1, I at 2: 2 becomes H, its agent goes back to 1
///  7 R3a  together on 1: 1 becomes P, one agent goes to the H neighbour 2
///  8 R4a  P at 1, H at 2, no I around 2: 2 becomes F
///  9 R7   P at 1, F at 2: 1 becomes F, 2 becomes I
/// 10 R8   F at 1, I at 2: 2 becomes F and its agent moves onto 1
/// 11 R9b  together on 1 (F), no H neighbour: 1 becomes H
/// 12 R3c  together on 1 (H), no H and no I neighbour: 1 becomes P
/// 13 R5b  together on 1 (P), no I neighbour: 1 becomes F, one agent goes to the P neighbour 0
/// 14 R7   P at 0, F at 1: 0 becomes F, 1 becomes I
/// 15 R8   F at 0, I at 1: 1 becomes F and its agent moves onto 0
/// 16 R11  together on 0, only F around: both paint 0 F and terminate
#[test]
fn hand_trace_on_a_path() {
    let g = graph(3, 0, &[(0, 1), (1, 2)]);
    match play_m(&g, &[], 1000, Model::Joint) {
        Outcome2::Explores { trace, .. } => {
            let tags: Vec<&str> = trace.steps.iter().map(|s| LABELS[s.rule as usize]).collect();
            assert_eq!(tags, vec!["R1", "R2", "R3a", "R4c", "R3b", "R2", "R3a", "R4a", "R7", "R8", "R9b", "R3c", "R5b", "R7", "R8", "R11"]);
            assert!(trace.end.ta && trace.end.tb && trace.end.a == 0 && trace.end.b == 0);
            assert_eq!(trace.end.chi, (3u128) | (3 << 4) | (3 << 8), "all three vertices finished");
            assert!(trace.steps.iter().all(|s| s.second.is_some()), "the second role is in the same step");
        }
        _ => panic!("the play must explore"),
    }
}

/// The reachable positions that are not over, with the agents' names forgotten (the pair as a set of two vertices).
fn configurations(g: &Graph) -> usize {
    let mut seen: std::collections::HashSet<(u128, u8, u8, u32)> = std::collections::HashSet::new();
    let mut done: std::collections::HashSet<Pos2> = std::collections::HashSet::new();
    let mut todo = vec![root(g)];
    while let Some(p) = todo.pop() {
        if !done.insert(p) || over(&p) {
            continue;
        }
        seen.insert((p.chi, p.a.min(p.b), p.a.max(p.b), p.vis));
        todo.extend(successors(g, &p));
    }
    seen.len()
}

/// The graph that breaks the older, 21-rule sheet (the diamond from a tip) is explored, and so is every graph of up to six
/// vertices from every start; the number of positions, with the agents' names forgotten, equals the totals of an
/// independent implementation (a Python simulator written from the sheet), so the two are held to the same game.
#[test]
fn every_connected_graph_up_to_six_vertices_is_explored() {
    for s in 0..4 {
        assert!(explores(&graph(4, s, &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3)])), "the diamond from {}", s);
    }
    let expect = [(2usize, 14usize), (3, 148), (4, 1578), (5, 17638), (6, 278128)];
    for (n, want) in expect {
        let mut total = 0usize;
        for edges in super::super::paper_tests::connected_graphs(n) {
            for s in 0..n {
                let g = graph(n, s, &edges);
                assert!(explores(&g), "a2c4 fails on {} {:?} from {}", n, edges, s);
                total += configurations(&g);
            }
        }
        assert_eq!(total, want, "configurations on the connected graphs with {} vertices", n);
    }
}

/// Seven vertices (853 graphs, 5,971 graph-start pairs): slow in a debug build, `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn every_connected_graph_on_seven_vertices_is_explored() {
    let mut total = 0usize;
    for edges in super::super::paper_tests::connected_graphs(7) {
        for s in 0..7 {
            let g = graph(7, s, &edges);
            assert!(explores(&g), "a2c4 fails on 7 {:?} from {}", edges, s);
            total += configurations(&g);
        }
    }
    assert_eq!(total, 6_267_980);
}

/// Every reachable position of a few graphs has a rule, and none asks for a move that cannot be made.
#[test]
fn no_reachable_position_is_stuck_or_blocked() {
    for (n, edges) in [(4usize, vec![(0u8, 1u8), (0, 2), (0, 3), (1, 2), (1, 3)]), (5, vec![(0, 1), (1, 2), (2, 3), (3, 4), (0, 4), (1, 3)]), (6, vec![(0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (1, 2), (3, 4)])] {
        for s in 0..n {
            let g = graph(n, s, &edges);
            let mut seen: std::collections::HashSet<Pos2> = std::collections::HashSet::new();
            let mut todo = vec![root(&g)];
            while let Some(q) = todo.pop() {
                if !seen.insert(q) || over(&q) {
                    continue;
                }
                let j = joint_of(&g, &q).expect("a position with no rule");
                assert!(!j.blocked, "a move to a colour nobody has");
                todo.extend(successors(&g, &q));
            }
        }
    }
}

/// Positions the algorithm never reaches have no rule or cannot move: apart and not adjacent, two H vertices side by
/// side, or a move to a colour nobody has.
#[test]
fn positions_without_a_rule_are_stuck() {
    let g = graph(3, 0, &[(0, 1), (1, 2)]);
    assert!(joint_of(&g, &pos(&g, &[P, I, P], 0, 2)).is_none(), "the agents are not next to each other");
    assert!(successors(&g, &pos(&g, &[P, I, P], 0, 2)).is_empty());
    assert!(joint_of(&g, &pos(&g, &[H, H, I], 0, 1)).is_none(), "two H vertices side by side: no rule");
    let g2 = graph(2, 0, &[(0, 1)]);
    let j = joint_of(&g2, &pos(&g2, &[I, H], 0, 0)).unwrap();
    assert_eq!(LABELS[j.id as usize], "R1");
    assert!(j.blocked && successors(&g2, &pos(&g2, &[I, H], 0, 0)).is_empty());
}
